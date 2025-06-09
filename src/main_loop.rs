use crate::{
    Config,
    event::Event,
    websocket::{WebSocket, WebSocketEvent},
};
use anyhow::{Context, Result, anyhow};
use futures_util::StreamExt as _;
use mpclipboard_common::{Clip, Store};
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub(crate) struct MainLoop {
    incoming_rx: Receiver<Clip>,
    outcoming_tx: Sender<Event>,
    stop_rx: Receiver<()>,
    store: Store,
    ws: WebSocket,
    connectivity_rx: Receiver<bool>,
}

impl MainLoop {
    pub(crate) fn new(
        incoming_rx: Receiver<Clip>,
        outcoming_tx: Sender<Event>,
        stop_rx: Receiver<()>,
        config: &'static Config,
    ) -> Self {
        let (ws, connectivity_rx) = WebSocket::new(config);
        Self {
            incoming_rx,
            outcoming_tx,
            stop_rx,
            store: Store::new(),
            ws,
            connectivity_rx,
        }
    }

    pub(crate) async fn start(self) -> Result<()> {
        let Self {
            mut incoming_rx,
            outcoming_tx,
            mut stop_rx,
            mut store,
            ws,
            mut connectivity_rx,
        } = self;

        let (ws_tx, mut ws_rx) = spawn_ws_task(ws);

        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    log::info!("received exit signal, stopping...");
                    break
                },

                clip = incoming_rx.recv() => {
                    let clip = clip.context("channel of incoming messages is closed")?;
                    if store.add(&clip) {
                        log::info!("new clip from local keyboard: {clip:?}");
                        if ws_tx.send(clip).await.is_err() {
                            log::error!("channel of messages from tokio to ws is closed");
                        }
                    }
                }

                clip = ws_rx.recv() => {
                    let clip = clip.context("channel of ws clips is closed")?;
                    if store.add(&clip) {
                        log::info!("new clip from ws: {clip:?}");
                        outcoming_tx
                            .send(Event::NewClip(clip))
                            .await
                            .map_err(|_| anyhow!("failed to send clip to main thread"))?;
                    }
                }

                connectivity = connectivity_rx.recv() => {
                    let connectivity = connectivity.context("connectivity channel is closed")?;
                    outcoming_tx
                        .send(Event::ConnectivityChanged(connectivity))
                        .await
                        .map_err(|_| anyhow!("failed to report connectivity"))?;
                }
            }
        }

        Ok(())
    }
}

fn spawn_ws_task(mut ws: WebSocket) -> (Sender<Clip>, Receiver<Clip>) {
    let (incoming_tx, mut incoming_rx) = channel::<Clip>(255);
    let (outcoming_tx, outcoming_rx) = channel::<Clip>(255);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                clip = incoming_rx.recv() => {
                    if let Some(clip) = clip {
                        ws.send(clip).await;
                    } else {
                        log::error!("[ws] failed to read clip for sending, stopping...");
                        break;
                    }
                }

                message = ws.next() => {
                    let Some(message) = message else {
                        log::error!("WS Connection is closed, bug?");
                        break;
                    };
                    match message {
                        WebSocketEvent::StartedConnecting => {
                            log::warn!("[ws] started connecting");
                        },
                        WebSocketEvent::Disconnected => {
                            log::warn!("[ws] disconnected");
                        },
                        WebSocketEvent::Connected => {
                            log::warn!("[ws] connected");
                        },
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
                            if outcoming_tx.send(clip).await.is_err() {
                                log::error!("[ws] failed to send clip back, stopping...");
                                break;
                            }
                        }
                        WebSocketEvent::MessageError(err) => {
                            log::error!("[ws] error during ws communication: {err:?}");
                        }
                    }
                }
            }
        }
    });

    (incoming_tx, outcoming_rx)
}
