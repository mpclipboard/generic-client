use crate::{Config, Handle, main_loop::MainLoop};
use anyhow::{Context as _, Result};
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

pub struct Thread;

impl Thread {
    pub fn start(config: Config) -> Result<Handle> {
        let (ctx, crx) = unbounded_channel();
        let (etx, erx) = unbounded_channel();
        let token = CancellationToken::new();
        let (pipe_reader, pipe_writer) = std::io::pipe().context("failed to create io pipe")?;

        let handle = {
            let token = token.clone();
            std::thread::spawn(move || {
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
                    MainLoop::new(crx, etx, config, token, pipe_writer)
                        .start()
                        .await;
                })
            })
        };

        Ok(Handle {
            ctx,
            erx,
            token,
            handle,
            pipe_reader: Some(pipe_reader),
        })
    }
}

/// # Safety
///
/// `config` must be a valid owned pointer to Config
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_thread_start(config: *mut Config) -> *mut Handle {
    let config = unsafe { Box::from_raw(config) };
    let handle = match Thread::start(*config) {
        Ok(handle) => handle,
        Err(err) => {
            log::error!("{err:?}");
            return std::ptr::null_mut();
        }
    };

    Box::leak(Box::new(handle))
}
