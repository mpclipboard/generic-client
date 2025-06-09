use crate::{
    config::Config,
    websocket::mapped::{MappedEvent as InnerEvent, MappedWebSocket as InnerWebSocket},
};
use anyhow::Result;
use futures_util::{FutureExt, Stream, StreamExt as _, ready};
use mpclipboard_common::Clip;
use pin_project_lite::pin_project;
use std::{pin::Pin, time::Duration};
use tokio::{
    sync::mpsc::{Receiver, Sender, channel},
    time::{Sleep, sleep},
};

pin_project! {
    pub(crate) struct ReconnectingWebSocket {
        config: &'static Config,
        tx: Sender<bool>,
        #[pin]
        state: State
    }
}

enum State {
    Connected {
        ws: InnerWebSocket,
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

#[derive(Clone, Copy)]
struct Retry {
    attempts_count: u64,
}
impl Retry {
    fn starting() -> Self {
        Self { attempts_count: 0 }
    }

    fn track(&mut self) {
        self.attempts_count += 1
    }

    fn delay(&self) -> u64 {
        const MAX_DELAY: u64 = 30;
        let delay = 2_u64.pow(self.attempts_count as u32).clamp(0, MAX_DELAY);
        log::warn!(
            "[retry] attempts = {}, delay = {delay}",
            self.attempts_count
        );
        delay
    }
}

impl ReconnectingWebSocket {
    pub(crate) fn new(config: &'static Config) -> (Self, Receiver<bool>) {
        let (tx, rx) = channel::<bool>(256);

        let this = Self {
            config,
            state: State::ReadyToConnect {
                retry: Retry::starting(),
            },
            tx,
        };
        (this, rx)
    }

    pub(crate) async fn send(&mut self, clip: Clip) {
        if let State::Connected { ws } = &mut self.state {
            ws.send_clip(&clip).await
        } else {
            log::error!("failed to send message to ws server (not connected)")
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
                return Ready(Some(StartedConnecting));
            }
            State::Connecting { fut, retry } => {
                let retry = *retry;
                let ws = ready!(fut.poll_unpin(cx));
                match ws {
                    Ok(ws) => {
                        this.state.set(State::Connected { ws });
                        return Ready(Some(Connected));
                    }
                    Err(err) => {
                        let delay = retry.delay();
                        let timer = Box::pin(sleep(Duration::from_secs(delay)));
                        this.state.set(State::Sleeping { fut: timer, retry });
                        return Ready(Some(StartedSleeping(err)));
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
