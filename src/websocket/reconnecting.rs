use crate::{
    config::Config,
    websocket::{
        mapped::{MappedEvent as InnerEvent, MappedWebSocket as InnerWebSocket},
        retry::Retry,
    },
};
use anyhow::Result;
use futures_util::{FutureExt, Stream, StreamExt as _, ready};
use mpclipboard_common::Clip;
use pin_project_lite::pin_project;
use std::{pin::Pin, time::Duration};
use tokio::time::{Sleep, sleep};

pin_project! {
    pub(crate) struct ReconnectingWebSocket {
        config: &'static Config,
        #[pin]
        state: State
    }
}

enum State {
    Connected {
        ws: Box<InnerWebSocket>,
    },
    ReadyToConnect {
        retry: Retry,
    },
    Connecting {
        fut: Pin<Box<dyn Future<Output = Result<InnerWebSocket>> + Send + 'static>>,
        retry: Retry,
    },
    Sleeping {
        fut: Pin<Box<Sleep>>,
        retry: Retry,
    },
}

impl ReconnectingWebSocket {
    pub(crate) fn new(config: &'static Config) -> Self {
        Self {
            config,
            state: State::ReadyToConnect {
                retry: Retry::starting(),
            },
        }
    }

    pub(crate) async fn send_clip(&mut self, clip: Clip) {
        if let State::Connected { ws } = &mut self.state {
            ws.send_clip(&clip).await
        } else {
            log::error!("failed to send message to ws server (not connected)")
        }
    }

    pub(crate) async fn send_ping(&mut self) {
        if let State::Connected { ws } = &mut self.state {
            ws.send_ping().await
        } else {
            log::error!("failed to send ping to ws server (not connected)")
        }
    }

    pub(crate) fn reset_connection(&mut self) {
        self.state = State::ReadyToConnect {
            retry: Retry::starting(),
        }
    }
}

pub(crate) enum Event {
    StartedConnecting,
    Disconnected,
    Connected,
    StartedSleeping(anyhow::Error),
    FinishedSleeping,
    MessageError(anyhow::Error),

    Ping,
    Pong,
    Clip(Clip),
}

impl From<InnerEvent> for Event {
    fn from(message: InnerEvent) -> Self {
        match message {
            InnerEvent::Ping => Self::Ping,
            InnerEvent::Pong => Self::Pong,
            InnerEvent::Clip(clip) => Self::Clip(clip),
        }
    }
}

impl Stream for ReconnectingWebSocket {
    type Item = Event;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use Event::*;
        use std::task::Poll::Ready;

        let mut this = self.project();

        match this.state.as_mut().get_mut() {
            State::Connected { ws } => {
                let message = ready!(ws.next().poll_unpin(cx));
                match message {
                    None => {
                        this.state.set(State::ReadyToConnect {
                            retry: Retry::starting(),
                        });
                        Ready(Some(Disconnected))
                    }
                    Some(Err(err)) => Ready(Some(MessageError(err))),
                    Some(Ok(message)) => Ready(Some(Event::from(message))),
                }
            }
            State::ReadyToConnect { retry } => {
                let mut retry = *retry;
                let fut = Box::pin(InnerWebSocket::new(this.config));
                retry.track();
                this.state.set(State::Connecting { fut, retry });
                Ready(Some(StartedConnecting))
            }
            State::Connecting { fut, retry } => {
                let retry = *retry;
                let ws = ready!(fut.poll_unpin(cx));
                match ws {
                    Ok(ws) => {
                        this.state.set(State::Connected { ws: Box::new(ws) });
                        Ready(Some(Connected))
                    }
                    Err(err) => {
                        let delay = retry.delay();
                        let timer = Box::pin(sleep(Duration::from_secs(delay)));
                        this.state.set(State::Sleeping { fut: timer, retry });
                        Ready(Some(StartedSleeping(err)))
                    }
                }
            }
            State::Sleeping { fut, retry } => {
                let retry = *retry;
                ready!(fut.poll_unpin(cx));
                this.state.set(State::ReadyToConnect { retry });
                Ready(Some(FinishedSleeping))
            }
        }
    }
}
