use anyhow::{Context as _, Result, anyhow};
use mpclipboard_common::Clip;
use std::sync::Mutex;
use tokio::sync::{OnceCell, mpsc::Receiver};

pub(crate) enum Event {
    ConnectivityChanged(bool),
    NewClip(Clip),
}

static EVENTS_RECEIVER: OnceCell<Mutex<Receiver<Event>>> = OnceCell::const_new();

impl Event {
    pub(crate) fn set_receiver(receiver: Receiver<Event>) {
        match EVENTS_RECEIVER.get() {
            Some(global) => {
                let mut global = global.lock().expect("lock is poisoned");
                *global = receiver;
            }
            None => {
                EVENTS_RECEIVER
                    .set(Mutex::new(receiver))
                    .expect("we just checked that EVENTS_RECEIVER is None");
            }
        }
    }

    pub(crate) fn recv_squashed() -> Result<(Option<Clip>, Option<bool>)> {
        let mut rx = EVENTS_RECEIVER
            .get()
            .context("EVENTS_RECEIVER is not set, did you forget to call Event::set_receiver() ?")?
            .lock()
            .map_err(|_| anyhow!("lock is poisoned"))?;

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
