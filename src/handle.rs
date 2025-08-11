use crate::{Output, clip::Clip, event::Event};
use anyhow::Result;
use anyhow::anyhow;
use std::{ffi::c_int, io::PipeReader, os::fd::AsRawFd, thread::JoinHandle};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

pub struct Handle {
    pub(crate) ctx: Sender<Clip>,
    pub(crate) erx: Receiver<Event>,
    pub(crate) token: CancellationToken,
    pub(crate) handle: JoinHandle<()>,
    pub(crate) pipe_reader: Option<PipeReader>,
}

impl Handle {
    pub fn send(&self, text: &str) -> Result<()> {
        self.ctx
            .blocking_send(Clip::new(text))
            .map_err(|_| anyhow!("failed to send command: channel is closed"))
    }

    pub fn recv(&mut self) -> (Option<Clip>, Option<bool>) {
        let mut clip = None;
        let mut connectivity = None;

        while let Ok(event) = self.erx.try_recv() {
            match event {
                Event::ConnectivityChanged(value) => connectivity = Some(value),
                Event::NewClip(value) => clip = Some(value),
            }
        }

        (clip, connectivity)
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
) {
    let handle = unsafe { &*handle };

    let Ok(text) = unsafe { std::ffi::CStr::from_ptr(text) }.to_str() else {
        log::error!("text is not NULL-terminated");
        return;
    };

    if let Err(err) = handle.send(text) {
        log::error!("{err:?}");
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
