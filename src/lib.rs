#![allow(static_mut_refs)]

pub use crate::{
    config::{
        Config, shared_clipboard_config_new, shared_clipboard_config_read_from_xdg_config_dir,
    },
    event::Event,
};
use crate::{incoming_tx::IncomingTx, outcoming_rx::OutcomingRx, thread::Thread};
use anyhow::{Context, Result};
use shared_clipboard_common::Clip;
use tokio::sync::mpsc::channel;

mod config;
mod connection;
mod event;
mod incoming_tx;
mod main_loop;
mod outcoming_rx;
mod runtime;
mod thread;
mod websocket;

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_setup() {
    pretty_env_logger::init();
}

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_start_thread(config: *mut Config) {
    let config = Config::from_ptr(config);
    let (incoming_tx, incoming_rx) = channel::<Clip>(256);
    let (outcoming_tx, outcoming_rx) = channel::<Event>(256);

    Thread::start(incoming_rx, outcoming_tx, config);

    IncomingTx::set(incoming_tx);
    OutcomingRx::set(outcoming_rx);
}
#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_stop_thread() -> bool {
    Thread::stop()
        .inspect_err(|err| log::error!("{err:?}"))
        .is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_send(text: *const u8) {
    fn send(text: *const u8) -> Result<()> {
        let text = unsafe { std::ffi::CStr::from_ptr(text.cast()) }
            .to_str()
            .context("text passed to shared_clipboard_clip_new must be NULL-terminated")?;
        let clip = Clip::new(text);
        IncomingTx::blocking_send(clip)?;
        Ok(())
    }

    if let Err(err) = send(text) {
        log::error!("{err:?}");
    }
}

#[repr(C)]
pub struct Output {
    pub text: *mut u8,
    pub connectivity: *mut bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_poll() -> Output {
    fn poll() -> Result<Output> {
        let (clip, connectivity) = OutcomingRx::recv_squashed()?;

        let text = clip
            .map(|clip| clip.text)
            .map(string_to_bytes)
            .unwrap_or(std::ptr::null_mut());
        let connectivity = connectivity
            .map(Box::new)
            .map(|c| Box::leak(c) as *mut _)
            .unwrap_or(std::ptr::null_mut());

        Ok(Output { text, connectivity })
    }

    poll()
        .inspect_err(|err| log::error!("{err:?}"))
        .unwrap_or(Output {
            text: std::ptr::null_mut(),
            connectivity: std::ptr::null_mut(),
        })
}

fn string_to_bytes(s: String) -> *mut u8 {
    match std::ffi::CString::new(s) {
        Ok(text) => {
            let mut bytes = text.into_bytes_with_nul();
            let ptr = bytes.as_mut_ptr();
            std::mem::forget(bytes);
            ptr
        }
        Err(_) => {
            log::error!("clip text is NULL terminated");
            std::ptr::null_mut()
        }
    }
}
