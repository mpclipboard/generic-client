use anyhow::{Context as _, Result};
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::net::TcpStream;
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

pub(crate) struct Websocket {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Websocket {
    pub(crate) async fn new(url: impl AsRef<str>) -> Result<Self> {
        let uri = http::Uri::try_from(url.as_ref()).context("invalid url")?;
        let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");

        let mut builder = ClientBuilder::from_uri(uri);

        let connector = if is_wss {
            log::info!("wss protocol detected, enabling TLS");
            Connector::NativeTls(tokio_native_tls::TlsConnector::from(
                tokio_native_tls::native_tls::TlsConnector::new()
                    .context("failed to create native TLS connector")?,
            ))
        } else {
            log::info!("plain ws protocol detected, disabling TLS");
            Connector::Plain
        };

        builder = builder.connector(&connector);

        let (ws, response) = builder.connect().await?;
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
