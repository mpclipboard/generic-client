use crate::{Config, connection::Connection, event::Event};
use anyhow::{Context, Result, anyhow};
use shared_clipboard_common::{Clip, Store};
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub(crate) struct MainLoop {
    incoming_rx: Receiver<Clip>,
    outcoming_tx: Sender<Event>,
    stop_rx: Receiver<()>,
    store: Store,
    ws: Connection,
    connectivity_rx: Receiver<bool>,
}

impl MainLoop {
    pub(crate) fn new(
        incoming_rx: Receiver<Clip>,
        outcoming_tx: Sender<Event>,
        stop_rx: Receiver<()>,
        config: Config,
    ) -> Self {
        let (ws, connectivity_rx) = Connection::new(config);
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

fn spawn_ws_task(mut ws: Connection) -> (Sender<Clip>, Receiver<Clip>) {
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

                clip = ws.next() => {
                    match clip {
                        Ok(clip) => {
                            if outcoming_tx.send(clip).await.is_err() {
                                log::error!("[ws] failed to send clip back, stopping...");
                                break;
                            }
                        }

                        Err(err) => {
                            log::error!("[ws] error during ws communication, stopping...");
                            log::error!("{err:?}");
                            break;
                        }
                    }
                }
            }
        }
    });

    (incoming_tx, outcoming_rx)
}
