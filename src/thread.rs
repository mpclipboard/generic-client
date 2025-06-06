use crate::{Config, event::Event, runtime::Runtime};
use anyhow::{Context as _, Result, anyhow};
use mpclipboard_common::Clip;
use std::{sync::Mutex, thread::JoinHandle};
use tokio::sync::mpsc::{Receiver, Sender};

pub(crate) struct Thread;
static THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

impl Thread {
    pub(crate) fn start(incoming_rx: Receiver<Clip>, outcoming_tx: Sender<Event>, config: Config) {
        let handle = std::thread::spawn(move || {
            Runtime::start(incoming_rx, outcoming_tx, config);
        });

        let mut global = THREAD.lock().expect("lock is poisoned");
        *global = Some(handle);
    }

    pub(crate) fn stop() -> Result<()> {
        let mut thread = THREAD.lock().map_err(|_| anyhow!("lock is poisoned"))?;
        let thread = thread
            .take()
            .context("thread is not running, exiting normally")?;
        Runtime::stop()?;
        thread
            .join()
            .map_err(|_| anyhow!("failed to join thread (bug?)"))?;
        Ok(())
    }
}
