use crate::{Config, clip::Clip, tls::TLS};
use anyhow::{Context as _, Result};
use futures::{SinkExt as _, StreamExt as _, future::BoxFuture};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{net::TcpStream, time::sleep};
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

pub(crate) enum Connection {
    Connecting {
        config: &'static Config,
        fut: BoxFuture<'static, Result<Conn, ()>>,
    },

    SendingAuthRequest {
        config: &'static Config,
        fut: BoxFuture<'static, Result<Conn, ()>>,
    },

    WaitingForAuthResponse {
        config: &'static Config,
        fut: BoxFuture<'static, Result<(bool, Conn), ()>>,
    },

    Connected {
        config: &'static Config,
        conn: Box<Conn>,
    },

    Disconnected {
        config: &'static Config,
        fut: BoxFuture<'static, ()>,
    },
}

#[derive(Debug)]
pub(crate) enum ConnectionEvent {
    Connecting,
    SendingAuthRequest,
    WaitingForAuthResponse,
    Connected,
    Disconnected,
    AuthFailed,
    ReceivedPing,
    ReceivedClip(Clip),
}

type Conn = WebSocketStream<MaybeTlsStream<TcpStream>>;

impl Connection {
    fn config(&self) -> &'static Config {
        match self {
            Self::Connecting { config, .. }
            | Self::SendingAuthRequest { config, .. }
            | Self::WaitingForAuthResponse { config, .. }
            | Self::Connected { config, .. }
            | Self::Disconnected { config, .. } => config,
        }
    }

    fn conn(&mut self) -> Option<&mut Conn> {
        if let Self::Connected { conn, .. } = self {
            Some(conn.as_mut())
        } else {
            None
        }
    }

    pub(crate) fn new(config: &'static Config) -> Self {
        connecting(config)
    }

    pub(crate) fn disconnect(&mut self) {
        if matches!(self, Self::Disconnected { .. }) {
            return;
        }
        *self = disconnected(self.config())
    }

    pub(crate) async fn send(&mut self, clip: &Clip) -> Result<()> {
        let json = serde_json::to_string(clip).context("failed to serialize clip")?;

        self.conn()
            .context("not connected")?
            .send(Message::text(json))
            .await
            .context("[ws] failed to send clip")
    }

    pub(crate) async fn recv(&mut self) -> ConnectionEvent {
        match self {
            Self::Connecting { config, fut } => match fut.await {
                Ok(conn) => {
                    log::info!("Connecting -> SendingAuthRequest");
                    *self = sending_auth_request(conn, config);
                    ConnectionEvent::SendingAuthRequest
                }
                Err(()) => {
                    log::info!("Connecting -> Disconnected");
                    *self = disconnected(config);
                    ConnectionEvent::Disconnected
                }
            },

            Self::SendingAuthRequest { config, fut } => match fut.await {
                Ok(conn) => {
                    log::info!("SendingAuthRequest -> WaitingForAuthResponse");
                    *self = waiting_for_auth_response(conn, config);
                    ConnectionEvent::WaitingForAuthResponse
                }
                Err(()) => {
                    log::info!("SendingAuthRequest -> Disconnected");
                    *self = disconnected(config);
                    ConnectionEvent::Disconnected
                }
            },

            Self::WaitingForAuthResponse { config, fut } => match fut.await {
                Ok((true, conn)) => {
                    log::info!("WaitingForAuthResponse -> Connected");
                    *self = Self::Connected {
                        config,
                        conn: Box::new(conn),
                    };
                    ConnectionEvent::Connected
                }
                Ok((false, _)) => {
                    log::info!("WaitingForAuthResponse -> Disconnected");
                    *self = disconnected(config);
                    ConnectionEvent::AuthFailed
                }
                Err(()) => {
                    log::info!("WaitingForAuthResponse -> Disconnected");
                    *self = disconnected(config);
                    ConnectionEvent::Disconnected
                }
            },

            Self::Connected { config, conn } => match read_message(conn).await {
                Ok(ConnectionMessage::Ping) => ConnectionEvent::ReceivedPing,
                Ok(ConnectionMessage::Clip(clip)) => ConnectionEvent::ReceivedClip(clip),
                Err(()) => {
                    log::info!("Connected -> Disconnected");
                    *self = disconnected(config);
                    ConnectionEvent::Disconnected
                }
            },

            Self::Disconnected { config, fut } => {
                fut.await;
                log::info!("Disconnected -> Connecting");
                *self = connecting(config);
                ConnectionEvent::Connecting
            }
        }
    }
}

fn connecting(config: &'static Config) -> Connection {
    async fn async_impl(config: &'static Config) -> Result<Conn, ()> {
        log::info!("Connecting to {}", config.uri);
        let is_wss = config.uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");
        let connector = if is_wss {
            log::info!("wss protocol detected, enabling TLS");
            let connector = match TLS::get() {
                Ok(connector) => connector,
                Err(err) => {
                    log::error!("[ws] {err:?}");
                    return Err(());
                }
            };
            Connector::Rustls(connector)
        } else {
            log::info!("plain ws protocol detected, disabling TLS");
            Connector::Plain
        };

        let client = ClientBuilder::from_uri(config.uri.clone()).connector(&connector);
        let (conn, response) = match client.connect().await {
            Ok(pair) => pair,
            Err(err) => {
                log::error!("[ws] {err:?}");
                return Err(());
            }
        };
        log::info!("[ws] handshake response code: {}", response.status());
        Ok(conn)
    }

    Connection::Connecting {
        config,
        fut: Box::pin(async_impl(config)),
    }
}

fn sending_auth_request(conn: Conn, config: &'static Config) -> Connection {
    async fn async_impl(mut conn: Conn, config: &'static Config) -> Result<Conn, ()> {
        #[derive(Serialize, Debug)]
        pub(crate) struct Auth {
            pub(crate) name: &'static str,
            pub(crate) token: &'static str,
        }

        let auth = Auth {
            name: &config.name,
            token: &config.token,
        };
        let Ok(json) = serde_json::to_string(&auth) else {
            log::error!("malformed name/token");
            return Err(());
        };
        let message = Message::text(json);

        if let Err(err) = conn.send(message).await {
            log::error!("failed to send auth request: {err:?}");
            return Err(());
        }

        Ok(conn)
    }

    Connection::SendingAuthRequest {
        config,
        fut: Box::pin(async_impl(conn, config)),
    }
}

fn waiting_for_auth_response(conn: Conn, config: &'static Config) -> Connection {
    async fn async_impl(mut conn: Conn) -> Result<(bool, Conn), ()> {
        let message = conn.next().await;
        let Some(message) = message else {
            return Err(());
        };
        let message = match message {
            Ok(message) => message,
            Err(err) => {
                log::error!("[ws] {err:?}");
                return Err(());
            }
        };
        let Some(message) = message.as_text() else {
            log::error!("[ws] expected TEXT (auth) message");
            return Err(());
        };

        #[derive(Deserialize)]
        struct AuthReply {
            success: bool,
        }

        match serde_json::from_str::<AuthReply>(message) {
            Ok(reply) => Ok((reply.success, conn)),
            Err(err) => {
                log::error!("[ws] failed to parse AuthReply: {err:?}");
                Err(())
            }
        }
    }

    Connection::WaitingForAuthResponse {
        config,
        fut: Box::pin(async_impl(conn)),
    }
}

pub(crate) enum ConnectionMessage {
    Ping,
    Clip(Clip),
}

async fn read_message(conn: &mut Conn) -> Result<ConnectionMessage, ()> {
    let message = match conn.next().await {
        Some(Ok(message)) => message,
        Some(Err(err)) => {
            log::error!("[ws] {err:?}");
            return Err(());
        }
        None => {
            log::error!("[ws] stream is closed");
            return Err(());
        }
    };

    if message.is_ping() {
        return Ok(ConnectionMessage::Ping);
    }

    let Some(message) = message.as_text() else {
        log::error!("[ws] received message is neither PING nor TEXT");
        return Err(());
    };

    match serde_json::from_str::<Clip>(message) {
        Ok(clip) => Ok(ConnectionMessage::Clip(clip)),
        Err(err) => {
            log::error!("[ws] failed to parse clip: {err:?}");
            Err(())
        }
    }
}

fn disconnected(config: &'static Config) -> Connection {
    async fn async_impl() {
        sleep(Duration::from_secs(5)).await
    }
    Connection::Disconnected {
        config,
        fut: Box::pin(async_impl()),
    }
}
