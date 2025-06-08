use anyhow::{Context as _, Result};
use futures_util::{SinkExt as _, Stream, StreamExt as _};
use pin_project_lite::pin_project;
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

pin_project! {
    pub(crate) struct WebsocketWithSsl {
        #[pin]
        ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    }
}

impl WebsocketWithSsl {
    pub(crate) async fn new(url: &str) -> Result<Self> {
        let uri = http::Uri::try_from(url).context("invalid url")?;
        let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");
        let connector = build_connector(is_wss)?;

        let (ws, response) = ClientBuilder::from_uri(uri)
            .connector(&connector)
            .connect()
            .await?;
        log::info!("WS(S) connect response: {response:?}");

        Ok(Self { ws })
    }

    pub(crate) async fn send(&mut self, message: Message) {
        if let Err(err) = self.ws.send(message).await {
            log::error!("failed to send message over WS: {err:?}");
        }
    }

    pub(crate) async fn next(&mut self) -> Option<Result<Message, tokio_websockets::Error>> {
        self.ws.next().await
    }
}

fn build_connector(ssl: bool) -> Result<Connector> {
    if ssl {
        log::info!("wss protocol detected, enabling TLS");

        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = ClientConfig::with_platform_verifier()
            .context("failed to create SSL client with platform verifier")?;
        let connector = TlsConnector::from(Arc::new(config));

        Ok(Connector::Rustls(connector))
    } else {
        log::info!("plain ws protocol detected, disabling TLS");
        Ok(Connector::Plain)
    }
}

impl Stream for WebsocketWithSsl {
    type Item = Result<Message>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let message = futures_util::ready!(self.project().ws.poll_next(cx))
            .map(|message| message.context("got an error from Websocket stream"));
        std::task::Poll::Ready(message)
    }
}
