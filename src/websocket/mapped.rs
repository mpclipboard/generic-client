use crate::{Config, websocket::with_ssl::WebSocketWithSsl};
use anyhow::{Result, anyhow};
use futures_util::{Stream, ready};
use mpclipboard_common::Clip;
use pin_project_lite::pin_project;
use tokio_websockets::Message;

pin_project! {
    pub(crate) struct MappedWebSocket {
        #[pin]
        inner: WebSocketWithSsl
    }
}

impl MappedWebSocket {
    pub(crate) async fn new(config: &'static Config) -> Result<Self> {
        let inner = WebSocketWithSsl::new(config).await?;
        Ok(Self { inner })
    }

    pub(crate) async fn send_clip(&mut self, clip: &Clip) {
        self.inner.send(Message::from(clip)).await
    }
    pub(crate) async fn send_ping(&mut self) {
        self.inner.send(Message::ping("")).await
    }
}

pub(crate) enum MappedEvent {
    Ping,
    Pong,
    Clip(Clip),
}

impl TryFrom<&Message> for MappedEvent {
    type Error = anyhow::Error;

    fn try_from(message: &Message) -> Result<Self, Self::Error> {
        if message.is_ping() {
            Ok(MappedEvent::Ping)
        } else if message.is_pong() {
            Ok(MappedEvent::Pong)
        } else if let Ok(clip) = Clip::try_from(message) {
            Ok(MappedEvent::Clip(clip))
        } else {
            Err(anyhow!("unknown message type: {message:?}"))
        }
    }
}

impl Stream for MappedWebSocket {
    type Item = Result<MappedEvent>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let message = ready!(self.project().inner.poll_next(cx));

        let message = match message {
            Some(Ok(message)) => Some(MappedEvent::try_from(&message)),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        };

        std::task::Poll::Ready(message)
    }
}
