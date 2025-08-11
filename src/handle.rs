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
    pub(crate) pipe_reader: PipeReader,
}

impl Handle {
    pub fn send(&mut self, text: &str) -> Result<()> {
        self.ctx
            .blocking_send(Clip::new(text))
            .map_err(|_| anyhow!("failed to send command: channel is closed"))
    }

    pub fn recv(&mut self) -> Result<(Option<Clip>, Option<bool>)> {
        let mut clip = None;
        let mut connectivity = None;

        while let Ok(event) = self.erx.try_recv() {
            match event {
                Event::ConnectivityChanged(value) => connectivity = Some(value),
                Event::NewClip(value) => clip = Some(value),
            }
        }

        Ok((clip, connectivity))
    }

    pub fn stop(self) -> Result<()> {
        self.token.cancel();
        self.handle
            .join()
            .map_err(|_| anyhow!("failed to join thread (bug?)"))?;
        Ok(())
    }

    pub fn fd(&self) -> i32 {
        self.pipe_reader.as_raw_fd()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_handle_send(handle: *mut Handle, text: *const std::ffi::c_char) {
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        log::error!("NULL handle");
        return;
    };

    let Ok(text) = unsafe { std::ffi::CStr::from_ptr(text) }.to_str() else {
        log::error!("text is not NULL-terminated");
        return;
    };

    if let Err(err) = handle.send(text) {
        log::error!("{err:?}");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_handle_poll(handle: *mut Handle) -> Output {
    let Some(handle) = (unsafe { handle.as_mut() }) else {
        log::error!("handle is NULL");
        return Output::null();
    };

    let (clip, connectivity) = match handle.recv() {
        Ok(pair) => pair,
        Err(err) => {
            log::error!("{err:?}");
            return Output::null();
        }
    };

    Output::new(clip, connectivity)
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_handle_stop(handle: *mut Handle) -> bool {
    let handle = unsafe { Box::from_raw(handle) };
    match handle.stop() {
        Ok(()) => true,
        Err(err) => {
            log::error!("failed to stop thread: {err:?}");
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_handle_get_fd(handle: *const Handle) -> c_int {
    let Some(handle) = (unsafe { handle.as_ref() }) else {
        log::error!("handle is NULL");
        return -1;
    };
    handle.fd()
}
