use anyhow::{Context as _, Result, anyhow};
use mpclipboard_common::Clip;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc::Sender;

static INCOMING_TX: LazyLock<Mutex<Option<Sender<Clip>>>> = LazyLock::new(|| Mutex::new(None));

pub(crate) struct IncomingTx;

impl IncomingTx {
    pub(crate) fn set(value: Sender<Clip>) {
        let mut tx = INCOMING_TX.lock().unwrap_or_else(|_| {
            log::error!("can't set INCOMING_TX: lock is poisoned");
            std::process::exit(1);
        });
        *tx = Some(value);
    }

    pub(crate) fn blocking_send(clip: Clip) -> Result<()> {
        let tx = INCOMING_TX
            .lock()
            .map_err(|_| anyhow!("lock is poisoned"))?;
        let tx = tx
            .as_ref()
            .context("no INCOMING_TX, did you start the thread?")?;
        tx.blocking_send(clip)
            .context("failed to send clip, recv has been dropped (tokio thread has crashed?)")?;
        Ok(())
    }
}
