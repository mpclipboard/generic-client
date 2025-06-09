use anyhow::{Context as _, Result};
use mpclipboard_common::Clip;
use tokio::sync::{OnceCell, mpsc::Sender};

pub(crate) enum Command {
    NewClip(Clip),
}

static COMMANDS_SENDER: OnceCell<Sender<Command>> = OnceCell::const_new();

impl Command {
    pub(crate) fn set_sender(sender: Sender<Command>) {
        if COMMANDS_SENDER.set(sender).is_err() {
            log::error!("Command::set_sender must be called exactly once");
            std::process::exit(1);
        }
    }

    pub(crate) fn send(self) -> Result<()> {
        COMMANDS_SENDER
            .get()
            .context("COMMAND_SENDER is not set, did you forget to call Command::set_sender() ?")?
            .blocking_send(self)
            .context("failed to send command")
    }
}
