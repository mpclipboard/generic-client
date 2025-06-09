use crate::{
    Config,
    command::Command,
    event::Event,
    websocket::{WebSocket, WebSocketEvent},
};
use anyhow::{Result, anyhow, bail};
use futures_util::StreamExt as _;
use mpclipboard_common::Store;
use std::time::Duration;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::{Instant, Interval, interval},
};

pub(crate) struct MainLoop {
    commands: Receiver<Command>,
    events: Sender<Event>,
    exit: Receiver<()>,
    store: Store,
    ws: WebSocket,

    timer: Interval,
    schedule: Schedule,
    last_communication_at: Instant,
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

            timer: interval(Duration::from_secs(1)),
            schedule: Schedule::new(),
            last_communication_at: Instant::now(),
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

                _ = self.timer.tick() => {
                    self.tick().await?;
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
                    self.ws.send_clip(clip).await;
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
                self.last_communication_at = Instant::now();
            }
            WebSocketEvent::StartedSleeping(err) => {
                log::warn!("[ws] started sleeping, {err:?}");
            }
            WebSocketEvent::FinishedSleeping => {
                log::warn!("[ws] finished sleeping");
            }
            WebSocketEvent::Ping => {
                log::info!("[ws] received ping");
                self.last_communication_at = Instant::now();
            }
            WebSocketEvent::Pong => {
                log::info!("[ws] received pong");
                self.last_communication_at = Instant::now();
            }
            WebSocketEvent::Clip(clip) => {
                if self.store.add(&clip) {
                    log::info!("new clip from ws: {clip:?}");
                    self.send_event(Event::NewClip(clip)).await?;
                }
                self.last_communication_at = Instant::now();
            }
            WebSocketEvent::MessageError(err) => {
                log::error!("[ws] error during ws communication: {err:?}");
                self.last_communication_at = Instant::now();
            }
        }

        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        self.schedule.tick();

        if self.schedule.do_ping() {
            self.ws.send_ping().await;
        }
        if self.schedule.do_offline_check() {
            self.do_offline_check().await?;
        }
        Ok(())
    }

    async fn do_offline_check(&mut self) -> Result<()> {
        log::info!("doing offline check");
        let delta = Instant::now() - self.last_communication_at;
        const OFFLINE_AFTER: Duration = Duration::from_secs(15);
        if delta > OFFLINE_AFTER {
            log::error!("offline for {}s, resetting connection", delta.as_secs());
            self.ws.reset_connection();
            self.send_event(Event::ConnectivityChanged(false)).await?;
        }
        Ok(())
    }
}

struct Schedule {
    tick: u64,
}
impl Schedule {
    fn new() -> Self {
        Self { tick: 0 }
    }

    fn tick(&mut self) {
        self.tick = (self.tick + 1) % Self::CYCLE;
    }

    const PING_EVERY: u64 = 5;
    fn do_ping(&self) -> bool {
        self.tick % Self::PING_EVERY == 0
    }

    const OFFLINE_CHECK_EVERY: u64 = 15;
    fn do_offline_check(&self) -> bool {
        self.tick % Self::OFFLINE_CHECK_EVERY == 0
    }

    const CYCLE: u64 = lcm(Self::PING_EVERY, Self::OFFLINE_CHECK_EVERY);
}

const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
const fn lcm(a: u64, b: u64) -> u64 {
    a * b / gcd(a, b)
}
