use crate::{
    Config,
    websocket::{WebsocketMessage, with_ssl::WebsocketWithSsl},
};
use anyhow::Result;
use futures_util::{Stream, ready};
use mpclipboard_common::Clip;
use pin_project_lite::pin_project;
use tokio_websockets::Message as WebSocketMessage;

pin_project! {
    pub(crate) struct MappedWebsocket {
        #[pin]
        inner: WebsocketWithSsl
    }
}

impl MappedWebsocket {
    pub(crate) async fn new(config: &Config) -> Result<Self> {
        let inner = WebsocketWithSsl::new(config).await?;
        Ok(Self { inner })
    }

    pub(crate) async fn send_clip(&mut self, clip: &Clip) {
        self.inner.send(WebSocketMessage::from(clip)).await
    }
    pub(crate) async fn send_ping(&mut self) {
        self.inner.send(WebSocketMessage::ping("")).await
    }
}

impl Stream for MappedWebsocket {
    type Item = Result<WebsocketMessage>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let message = ready!(self.project().inner.poll_next(cx));

        let message = match message {
            Some(Ok(message)) => Some(WebsocketMessage::try_from(&message)),
            Some(Err(err)) => Some(Err(err)),
            None => None,
        };

        std::task::Poll::Ready(message)
    }
}
