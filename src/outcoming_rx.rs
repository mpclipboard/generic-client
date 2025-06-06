use crate::Event;
use anyhow::{Context as _, Result, anyhow};
use mpclipboard_common::Clip;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc::Receiver;

static OUTCOMING_RX: LazyLock<Mutex<Option<Receiver<Event>>>> = LazyLock::new(|| Mutex::new(None));

pub(crate) struct OutcomingRx;

impl OutcomingRx {
    pub(crate) fn set(value: Receiver<Event>) {
        let mut rx = OUTCOMING_RX.lock().unwrap_or_else(|_| {
            log::error!("can't set OUTCOMING_RX: lock is poisoned");
            std::process::exit(1);
        });
        *rx = Some(value);
    }

    pub(crate) fn recv_squashed() -> Result<(Option<Clip>, Option<bool>)> {
        let mut rx = OUTCOMING_RX
            .lock()
            .map_err(|_| anyhow!("lock is poisoned"))?;
        let rx = rx
            .as_mut()
            .context("no OUTCOMING_RX, did you start the thread?")?;

        let mut clip = None;
        let mut connectivity = None;

        while let Ok(event) = rx.try_recv() {
            match event {
                Event::ConnectivityChanged(value) => connectivity = Some(value),
                Event::NewClip(value) => clip = Some(value),
            }
        }

        Ok((clip, connectivity))
    }
}
