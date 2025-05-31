use crate::{Config, event::Event, main_loop::MainLoop};
use anyhow::{Context as _, Result, anyhow};
use shared_clipboard_common::Clip;
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub(crate) struct Runtime;

static mut STOP_TX: Option<Sender<()>> = None;

impl Runtime {
    pub(crate) fn start(incoming_rx: Receiver<Clip>, outcoming_tx: Sender<Event>, config: Config) {
        let (stop_tx, stop_rx) = channel::<()>(1);
        unsafe { STOP_TX = Some(stop_tx) };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|err| {
                log::error!("failed to start tokio runtime: {err:?}");
                std::process::exit(1);
            });

        rt.block_on(async move {
            let main_loop = MainLoop::new(incoming_rx, outcoming_tx, stop_rx, config);
            if let Err(err) = main_loop.start().await {
                log::error!("main loop error, stopping...");
                log::error!("{err:?}")
            }
        });

        log::info!("tokio has finished");
    }

    pub(crate) fn stop() -> Result<()> {
        let tx = unsafe { STOP_TX.take() }.context("runtime has not started, can't stop")?;
        tx.blocking_send(())
            .map_err(|_| anyhow!("failed to send shutdown message"))?;
        Ok(())
    }
}
