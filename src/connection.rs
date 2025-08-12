use crate::{Config, clip::Clip, tls::TLS};
use anyhow::Result;
use futures::{SinkExt as _, StreamExt as _, future::BoxFuture};
use http::Uri;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{net::TcpStream, time::sleep};
use tokio_websockets::{ClientBuilder, Connector, MaybeTlsStream, Message, WebSocketStream};

pub(crate) struct Connection {
    state: State,
    config: Config,
    pending: Option<Clip>,
}

enum State {
    Connecting {
        fut: BoxFuture<'static, Result<Conn, ()>>,
    },

    SendingAuthRequest {
        fut: BoxFuture<'static, Result<Conn, ()>>,
    },

    WaitingForAuthResponse {
        fut: BoxFuture<'static, Result<(bool, Conn), ()>>,
    },

    Connected {
        conn: Box<Conn>,
    },

    Disconnected {
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
    pub(crate) fn new(config: Config) -> Self {
        Self {
            state: connecting(&config.uri),
            config,
            pending: None,
        }
    }

    pub(crate) fn disconnect(&mut self) {
        if matches!(self.state, State::Disconnected { .. }) {
            return;
        }
        self.state = disconnected()
    }

    pub(crate) async fn send(&mut self, clip: Clip) {
        let json = serde_json::to_string(&clip).expect("failed to serialize clip");

        match &mut self.state {
            State::Connected { conn, .. } => {
                if let Err(err) = conn.send(Message::text(json)).await {
                    log::error!("[ws] failed to send clip: {err:?}");
                }
            }
            State::Connecting { .. }
            | State::SendingAuthRequest { .. }
            | State::WaitingForAuthResponse { .. }
            | State::Disconnected { .. } => self.pending = Some(clip),
        }
    }

    pub(crate) async fn send_pending_if_any(&mut self) {
        let State::Connected { conn } = &mut self.state else {
            return;
        };
        let Some(clip) = self.pending.as_mut() else {
            return;
        };
        let json = serde_json::to_string(clip).expect("failed to serialize clip");

        match conn.send(Message::text(json)).await {
            Ok(()) => {
                log::info!("[ws] pending clip has been sent");
                self.pending = None
            }
            Err(err) => log::error!("[ws] failed to send clip: {err:?}"),
        }
    }

    pub(crate) async fn recv(&mut self) -> ConnectionEvent {
        match &mut self.state {
            State::Connecting { fut } => match fut.await {
                Ok(conn) => {
                    log::info!("Connecting -> SendingAuthRequest");
                    self.state = sending_auth_request(conn, &self.config);
                    ConnectionEvent::SendingAuthRequest
                }
                Err(()) => {
                    log::info!("Connecting -> Disconnected");
                    self.state = disconnected();
                    ConnectionEvent::Disconnected
                }
            },

            State::SendingAuthRequest { fut } => match fut.await {
                Ok(conn) => {
                    log::info!("SendingAuthRequest -> WaitingForAuthResponse");
                    self.state = waiting_for_auth_response(conn);
                    ConnectionEvent::WaitingForAuthResponse
                }
                Err(()) => {
                    log::info!("SendingAuthRequest -> Disconnected");
                    self.state = disconnected();
                    ConnectionEvent::Disconnected
                }
            },

            State::WaitingForAuthResponse { fut } => match fut.await {
                Ok((true, conn)) => {
                    log::info!("WaitingForAuthResponse -> Connected");
                    self.state = State::Connected {
                        conn: Box::new(conn),
                    };
                    ConnectionEvent::Connected
                }
                Ok((false, _)) => {
                    log::info!("WaitingForAuthResponse -> Disconnected");
                    self.state = disconnected();
                    ConnectionEvent::AuthFailed
                }
                Err(()) => {
                    log::info!("WaitingForAuthResponse -> Disconnected");
                    self.state = disconnected();
                    ConnectionEvent::Disconnected
                }
            },

            State::Connected { conn } => match read_message(conn).await {
                Ok(ConnectionMessage::Ping) => ConnectionEvent::ReceivedPing,
                Ok(ConnectionMessage::Clip(clip)) => ConnectionEvent::ReceivedClip(clip),
                Err(()) => {
                    log::info!("Connected -> Disconnected");
                    self.state = disconnected();
                    ConnectionEvent::Disconnected
                }
            },

            State::Disconnected { fut } => {
                fut.await;
                log::info!("Disconnected -> Connecting");
                self.state = connecting(&self.config.uri);
                ConnectionEvent::Connecting
            }
        }
    }
}

fn connecting(uri: &Uri) -> State {
    async fn async_impl(uri: Uri) -> Result<Conn, ()> {
        log::info!("Connecting to {uri}");
        let is_wss = uri.scheme().map(|scheme| scheme.as_str()) == Some("wss");
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

        let client = ClientBuilder::from_uri(uri).connector(&connector);
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

    State::Connecting {
        fut: Box::pin(async_impl(uri.clone())),
    }
}

fn sending_auth_request(conn: Conn, config: &Config) -> State {
    async fn async_impl(mut conn: Conn, config: Config) -> Result<Conn, ()> {
        #[derive(Serialize, Debug)]
        pub(crate) struct Auth {
            pub(crate) name: String,
            pub(crate) token: String,
        }

        let auth = Auth {
            name: config.name,
            token: config.token,
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

    State::SendingAuthRequest {
        fut: Box::pin(async_impl(conn, config.clone())),
    }
}

fn waiting_for_auth_response(conn: Conn) -> State {
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

    State::WaitingForAuthResponse {
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

fn disconnected() -> State {
    async fn async_impl() {
        sleep(Duration::from_secs(5)).await
    }
    State::Disconnected {
        fut: Box::pin(async_impl()),
    }
}
