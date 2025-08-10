use crate::{Config, clip::Clip, event::Event, main_loop::MainLoop};
use anyhow::{Result, anyhow};
use std::thread::JoinHandle;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

pub(crate) struct Thread {
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl Thread {
    pub(crate) fn spawn(
        clips_to_send: Receiver<Clip>,
        events: Sender<Event>,
        config: &'static Config,
    ) -> Self {
        let token = CancellationToken::new();
        let handle = {
            let token = token.clone();
            std::thread::spawn(move || {
                Self::start_tokio_runtime(clips_to_send, events, config, token);
            })
        };

        Self { token, handle }
    }

    pub(crate) fn stop(self) -> Result<()> {
        self.token.cancel();
        self.handle
            .join()
            .map_err(|_| anyhow!("failed to join thread (bug?)"))?;
        Ok(())
    }

    fn start_tokio_runtime(
        clips_to_send: Receiver<Clip>,
        events: Sender<Event>,
        config: &'static Config,
        token: CancellationToken,
    ) {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                log::error!("failed to start tokio: {err:?}");
                return;
            }
        };

        rt.block_on(async move {
            MainLoop::new(clips_to_send, events, config, token)
                .start()
                .await
        })
    }
}
