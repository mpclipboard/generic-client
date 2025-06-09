use anyhow::{Context as _, Result, anyhow};
use futures_util::{SinkExt as _, Stream, ready};
use http::{HeaderName, HeaderValue};
use pin_project_lite::pin_project;
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;
use std::{str::FromStr, sync::Arc};
use tokio::{net::TcpStream, sync::OnceCell};
use tokio_rustls::TlsConnector;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

use crate::Config;

pin_project! {
    pub(crate) struct WebsocketWithSsl {
        #[pin]
        ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    }
}

impl WebsocketWithSsl {
    pub(crate) async fn new(config: &Config) -> Result<Self> {
        let uri = http::Uri::try_from(&config.url).context("invalid url")?;
        let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");
        let connector = if is_wss {
            let connector = TLS_CONNECTOR
                .get()
                .context("no CONNECTOR, did you forget to call init_connector() ?")?
                .clone();
            log::info!("wss protocol detected, enabling TLS");
            Connector::Rustls(connector)
        } else {
            log::info!("plain ws protocol detected, disabling TLS");
            Connector::Plain
        };

        let (ws, response) = ClientBuilder::from_uri(uri)
            .connector(&connector)
            .add_header(
                HeaderName::from_str("Token").expect("failed to create Token header, bug?"),
                HeaderValue::from_str(&config.token)
                    .context("token can't be used as an HTTP header")?,
            )
            .context("failed to add Token header to the WebSocket stream")?
            .add_header(
                HeaderName::from_str("Name").expect("failed to create Device header, bug?"),
                HeaderValue::from_str(&config.name)
                    .context("name can't be used as an HTTP header")?,
            )
            .context("failed to add name header to the WebSocket stream")?
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
}

static TLS_CONNECTOR: OnceCell<TlsConnector> = OnceCell::const_new();

pub(crate) fn init_tls_connector() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let config = ClientConfig::with_platform_verifier()
        .context("failed to create SSL client with platform verifier")?;
    let connector = TlsConnector::from(Arc::new(config));

    TLS_CONNECTOR
        .set(connector)
        .map_err(|_| anyhow!("websocket::init_connector must be called exactly once"))?;

    Ok(())
}

impl Stream for WebsocketWithSsl {
    type Item = Result<Message>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let message = ready!(self.project().ws.poll_next(cx))
            .map(|message| message.context("got an error from Websocket stream"));
        std::task::Poll::Ready(message)
    }
}
