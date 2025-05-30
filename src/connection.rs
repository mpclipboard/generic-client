use crate::{config::Config, websocket::Websocket};
use anyhow::{Context as _, Result, bail};
use shared_clipboard_common::{AuthRequest, AuthResponse, Clip};
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_websockets::Message;

pub struct Connection {
    config: Config,
    ws: Option<Websocket>,
    connectivity_tx: Sender<bool>,
}

impl Connection {
    pub(crate) fn new(config: Config) -> (Self, Receiver<bool>) {
        let (tx, rx) = channel::<bool>(256);
        let this = Self {
            config,
            ws: None,
            connectivity_tx: tx,
        };
        (this, rx)
    }

    async fn connect(&mut self) -> Result<()> {
        let mut ws = Websocket::new(&self.config.url).await?;
        authenticate(&mut ws, &self.config).await?;
        self.ws = Some(ws);
        self.connectivity_tx
            .send(true)
            .await
            .context("failed to report connectivity through channel")?;
        Ok(())
    }

    async fn reconnect(&mut self) -> Result<()> {
        self.connectivity_tx
            .send(false)
            .await
            .context("failed to report connectivity through channel")?;
        self.ws = None;
        log::info!("starting reconnect loop...");
        let mut delay = 2;
        const MAX_DELAY: u64 = 10;
        loop {
            log::info!("trying to reconnect now...");
            match self.connect().await {
                Ok(_) => return Ok(()),
                Err(err) => log::error!("failed to reconnect: {err:?}"),
            }
            tokio::time::sleep(Duration::from_secs(delay)).await;
            delay = (delay * 2).clamp(0, MAX_DELAY);
        }
    }

    pub(crate) async fn next(&mut self) -> Result<Clip> {
        loop {
            let Some(ws) = self.ws.as_mut() else {
                log::error!("not connected, can't poll; reconnecting...");
                self.reconnect().await?;
                continue;
            };

            let Some(message) = ws.next().await else {
                log::error!("connection is closed; reconnecting...");
                self.reconnect().await?;
                continue;
            };

            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    log::error!("connection error: {err:?}");
                    self.reconnect().await?;
                    continue;
                }
            };

            match Clip::try_from(message) {
                Ok(clip) => return Ok(clip),
                Err(err) => {
                    log::error!("communication error: {err:?}");
                    self.reconnect().await?;
                    continue;
                }
            };
        }
    }

    pub(crate) async fn send(&mut self, clip: Clip) {
        if let Some(ws) = self.ws.as_mut() {
            ws.send(Message::from(clip)).await
        } else {
            log::error!("failed to send message to ws server (not connected)")
        }
    }
}

pub(crate) async fn authenticate(ws: &mut Websocket, config: &Config) -> Result<()> {
    log::info!("Authenticating as {:?}", config.name);
    let message = Message::from(AuthRequest::new(&config.name, &config.token));
    ws.send(message).await;
    log::info!("Authentication message sent, waiting for reply...");

    let message = ws
        .next()
        .await
        .context("closed stream, no auth response")?
        .context("websocket error, no auth response")?;
    let auth = AuthResponse::try_from(message)?;
    log::info!("Got authentication response: {}", auth.success);

    if auth.success {
        Ok(())
    } else {
        bail!("auth failed")
    }
}
