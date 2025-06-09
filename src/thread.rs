use crate::{Command, Config, event::Event, runtime::Runtime};
use anyhow::{Context as _, Result, anyhow};
use std::{sync::Mutex, thread::JoinHandle};
use tokio::sync::mpsc::{Receiver, Sender};

pub(crate) struct Thread;
static THREAD: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

impl Thread {
    pub(crate) fn start(
        commands: Receiver<Command>,
        events: Sender<Event>,
        config: &'static Config,
    ) {
        let handle = std::thread::spawn(move || {
            Runtime::start(commands, events, config);
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
