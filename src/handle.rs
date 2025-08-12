use crate::{Output, clip::Clip, event::Event};
use anyhow::anyhow;
use anyhow::{Context as _, Result};
use std::{ffi::c_int, io::PipeReader, os::fd::AsRawFd, thread::JoinHandle};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    oneshot::Sender,
};
use tokio_util::sync::CancellationToken;

pub struct Handle {
    pub(crate) ctx: UnboundedSender<(Clip, Sender<bool>)>,
    pub(crate) erx: UnboundedReceiver<Event>,
    pub(crate) token: CancellationToken,
    pub(crate) handle: JoinHandle<()>,
    pub(crate) pipe_reader: Option<PipeReader>,
}

impl Handle {
    pub fn send(&self, text: &str) -> Result<bool> {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        let clip = Clip::new(text);
        self.ctx
            .send((clip, tx))
            .map_err(|_| anyhow!("failed to send command: channel is closed"))?;
        rx.blocking_recv()
            .context("failed to recv reply: channel is closed")
    }

    pub fn recv(&mut self) -> (Option<String>, Option<bool>) {
        let mut text = None;
        let mut connectivity = None;

        while let Ok(event) = self.erx.try_recv() {
            match event {
                Event::ConnectivityChanged(connected) => connectivity = Some(connected),
                Event::NewClip(clip) => text = Some(clip.text),
            }
        }

        (text, connectivity)
    }

    pub fn stop(self) -> Result<()> {
        self.token.cancel();
        self.handle
            .join()
            .map_err(|_| anyhow!("failed to join thread (bug?)"))?;
        Ok(())
    }

    pub fn pipe_reader(&mut self) -> Option<PipeReader> {
        self.pipe_reader.take()
    }
}

/// # Safety
///
/// `handle` must be a valid pointer to Handle
/// `text` must be a NULL terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_handle_send(
    handle: *const Handle,
    text: *const std::ffi::c_char,
) -> bool {
    let handle = unsafe { &*handle };

    let Ok(text) = unsafe { std::ffi::CStr::from_ptr(text) }.to_str() else {
        log::error!("text is not NULL-terminated");
        return false;
    };

    match handle.send(text) {
        Ok(is_new) => is_new,
        Err(err) => {
            log::error!("{err:?}");
            false
        }
    }
}

/// # Safety
///
/// `handle` must be a valid pointer to Handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_handle_poll(handle: *mut Handle) -> Output {
    let handle = unsafe { &mut *handle };
    let (clip, connectivity) = handle.recv();
    Output::new(clip, connectivity)
}

/// # Safety
///
/// `handle` must be a valid pointer to Handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_handle_stop(handle: *mut Handle) -> bool {
    let handle = unsafe { Box::from_raw(handle) };
    match handle.stop() {
        Ok(()) => true,
        Err(err) => {
            log::error!("failed to stop thread: {err:?}");
            false
        }
    }
}

/// # Safety
///
/// `handle` must be a valid pointer to Handle
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_handle_take_fd(handle: *mut Handle) -> c_int {
    let handle = unsafe { &mut *handle };
    let Some(pipe_reader) = handle.pipe_reader() else {
        return -1;
    };
    let fd = pipe_reader.as_raw_fd();
    std::mem::forget(pipe_reader);
    fd
}
