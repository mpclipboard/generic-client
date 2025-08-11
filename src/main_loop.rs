use crate::{
    Config,
    connection::{Connection, ConnectionEvent},
    event::Event,
};
use crate::{clip::Clip, store::Store};
use std::{
    io::{PipeWriter, Write as _},
    time::Duration,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::{Instant, Interval, interval},
};
use tokio_util::sync::CancellationToken;

pub(crate) struct MainLoop {
    clips_to_send: Receiver<Clip>,
    events: Sender<Event>,
    token: CancellationToken,
    store: Store,
    conn: Connection,
    pipe_writer: PipeWriter,

    timer: Interval,
    reconnect_at: Instant,
}

impl MainLoop {
    pub(crate) fn new(
        clips_to_send: Receiver<Clip>,
        events: Sender<Event>,
        config: Config,
        token: CancellationToken,
        pipe_writer: PipeWriter,
    ) -> Self {
        Self {
            clips_to_send,
            events,
            token,
            store: Store::new(),
            conn: Connection::new(config),
            pipe_writer,

            timer: interval(Duration::from_secs(1)),
            reconnect_at: fifteen_secs_from_now(),
        }
    }

    pub(crate) async fn start(&mut self) {
        loop {
            tokio::select! {
                _ = self.token.cancelled() => {
                    log::info!("received exit signal, stopping...");
                    break;
                },

                Some(clip_to_send) = self.clips_to_send.recv() => {
                    self.send_clip(clip_to_send).await;
                }

                event = self.conn.recv() => {
                    self.process_event(event).await;
                }

                _ = self.timer.tick() => {
                    self.tick().await;
                }
            }
        }
    }

    async fn send_clip(&mut self, clip: Clip) {
        if self.store.add(&clip) {
            log::info!("new clip from local keyboard: {clip:?}");
            if let Err(err) = self.conn.send(&clip).await {
                log::error!("{err:?}");
            }
        }
    }

    async fn send_event(&mut self, event: Event) {
        if self.events.send(event).await.is_err() {
            log::error!("[ws] failed to send event: channel is closed");
        }
        if let Err(err) = self.pipe_writer.write(b"1") {
            log::error!("failed to trigger notification via pipe writer: {err:?}")
        }
    }

    async fn process_event(&mut self, event: ConnectionEvent) {
        match event {
            ConnectionEvent::Connecting => {}
            ConnectionEvent::SendingAuthRequest => {}
            ConnectionEvent::WaitingForAuthResponse => {}
            ConnectionEvent::Connected => {
                self.send_event(Event::ConnectivityChanged(true)).await;
            }
            ConnectionEvent::Disconnected => {
                self.send_event(Event::ConnectivityChanged(false)).await;
            }
            ConnectionEvent::AuthFailed => {}
            ConnectionEvent::ReceivedPing => {
                self.reconnect_at = fifteen_secs_from_now();
            }
            ConnectionEvent::ReceivedClip(clip) => {
                if self.store.add(&clip) {
                    log::info!("new clip from ws: {clip:?}");
                    self.send_event(Event::NewClip(clip)).await;
                }
            }
        }
    }

    async fn tick(&mut self) {
        if self.reconnect_at < Instant::now() {
            self.reconnect_at = fifteen_secs_from_now();
            self.conn.disconnect();
            self.send_event(Event::ConnectivityChanged(false)).await;
        }
    }
}

fn fifteen_secs_from_now() -> Instant {
    Instant::now() + Duration::from_secs(15)
}
