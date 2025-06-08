use anyhow::{Context as _, Result};
use futures_util::{SinkExt as _, StreamExt as _};
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

pub(crate) struct Websocket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Websocket {
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
