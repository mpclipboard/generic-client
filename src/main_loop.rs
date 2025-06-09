use crate::{
    Config,
    command::Command,
    event::Event,
    websocket::{WebSocket, WebSocketEvent},
};
use anyhow::{Result, anyhow, bail};
use futures_util::StreamExt as _;
use mpclipboard_common::Store;
use tokio::sync::mpsc::{Receiver, Sender};

pub(crate) struct MainLoop {
    commands: Receiver<Command>,
    events: Sender<Event>,
    exit: Receiver<()>,
    store: Store,
    ws: WebSocket,
}

impl MainLoop {
    pub(crate) fn new(
        commands: Receiver<Command>,
        events: Sender<Event>,
        exit: Receiver<()>,
        config: &'static Config,
    ) -> Self {
        Self {
            commands,
            events,
            exit,
            store: Store::new(),
            ws: WebSocket::new(config),
        }
    }

    pub(crate) async fn start(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.exit.recv() => {
                    log::info!("received exit signal, stopping...");
                    break
                },

                command = self.commands.recv() => {
                    let Some(command) = command else {
                        bail!("channel of commands is closed");
                    };
                    self.on_command(command).await?;
                }

                ws_event = self.ws.next() => {
                    let Some(ws_event) = ws_event else {
                        bail!("ws channel is closed. bug?");
                    };
                    self.on_ws_event(ws_event).await?;
                }
            }
        }

        Ok(())
    }

    async fn on_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::NewClip(clip) => {
                if self.store.add(&clip) {
                    log::info!("new clip from local keyboard: {clip:?}");
                    self.ws.send(clip).await;
                }
            }
        }
        Ok(())
    }

    async fn send_event(&mut self, event: Event) -> Result<()> {
        self.events
            .send(event)
            .await
            .map_err(|_| anyhow!("[ws] failed to send event, channel is closed"))
    }

    async fn on_ws_event(&mut self, ws_event: WebSocketEvent) -> Result<()> {
        match ws_event {
            WebSocketEvent::StartedConnecting => {
                log::warn!("[ws] started connecting");
            }
            WebSocketEvent::Disconnected => {
                log::warn!("[ws] disconnected");
                self.send_event(Event::ConnectivityChanged(false)).await?;
            }
            WebSocketEvent::Connected => {
                log::warn!("[ws] connected");
                self.send_event(Event::ConnectivityChanged(true)).await?;
            }
            WebSocketEvent::StartedSleeping(err) => {
                log::warn!("[ws] started sleeping, {err:?}");
            }
            WebSocketEvent::FinishedSleeping => {
                log::warn!("[ws] finished sleeping");
            }
            WebSocketEvent::Ping => {
                log::info!("[ws] received ping");
            }
            WebSocketEvent::Pong => {
                log::info!("[ws] received pong");
            }
            WebSocketEvent::Clip(clip) => {
                if self.store.add(&clip) {
                    log::info!("new clip from ws: {clip:?}");
                    self.send_event(Event::NewClip(clip)).await?;
                }
            }
            WebSocketEvent::MessageError(err) => {
                log::error!("[ws] error during ws communication: {err:?}");
            }
        }

        Ok(())
    }
}
