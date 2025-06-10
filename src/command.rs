use std::sync::Mutex;

use anyhow::{Context as _, Result};
use mpclipboard_common::Clip;
use tokio::sync::{OnceCell, mpsc::Sender};

pub(crate) enum Command {
    NewClip(Clip),
}

static COMMANDS_SENDER: OnceCell<Mutex<Sender<Command>>> = OnceCell::const_new();

impl Command {
    pub(crate) fn set_sender(sender: Sender<Command>) {
        match COMMANDS_SENDER.get() {
            Some(global) => {
                let mut global = global.lock().expect("lock is poisoned");
                *global = sender;
            }
            None => {
                COMMANDS_SENDER
                    .set(Mutex::new(sender))
                    .expect("we just checked that COMMAND_SENDER is None");
            }
        }
    }

    pub(crate) fn send(self) -> Result<()> {
        COMMANDS_SENDER
            .get()
            .context("COMMAND_SENDER is not set, did you forget to call Command::set_sender() ?")?
            .lock()
            .expect("lock is poisoned")
            .blocking_send(self)
            .context("failed to send command")
    }
}
