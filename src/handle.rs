use crate::{clip::Clip, event::Event, thread::Thread};
use anyhow::Result;
use anyhow::anyhow;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Handle {
    pub(crate) ctx: Sender<Clip>,
    pub(crate) erx: Receiver<Event>,
    pub(crate) thread: Thread,
}

impl Handle {
    pub(crate) fn borrow_from_ptr(ptr: *mut Handle) -> Option<&'static mut Self> {
        unsafe { ptr.as_mut() }
    }

    pub(crate) fn owned_from_ptr(ptr: *mut Handle) -> Option<Box<Self>> {
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

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
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_send(handle: *mut Handle, text: *const u8) {
    let Some(handle) = Handle::borrow_from_ptr(handle) else {
        log::error!("no handle given");
        return;
    };

    let text = unsafe { std::ffi::CStr::from_ptr(text.cast()) };
    let Ok(text) = text.to_str() else {
        log::error!("text passed to mpclipboard_clip_new must be NULL-terminated");
        return;
    };

    if let Err(err) = handle.send(text) {
        log::error!("{err:?}");
    }
}

#[repr(C)]
pub struct Output {
    pub text: *mut u8,
    pub connectivity: *mut bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_poll(handle: *mut Handle) -> Output {
    let mut output = Output {
        text: std::ptr::null_mut(),
        connectivity: std::ptr::null_mut(),
    };

    let Some(handle) = Handle::borrow_from_ptr(handle) else {
        log::error!("handle is NULL");
        return output;
    };

    let (clip, connectivity) = match handle.recv() {
        Ok(pair) => pair,
        Err(err) => {
            log::error!("{err:?}");
            return output;
        }
    };

    if let Some(clip) = clip {
        output.text = string_to_null_terminated_bytes(clip.text);
    }
    if let Some(connectivity) = connectivity {
        output.connectivity = Box::leak(Box::new(connectivity));
    }

    output
}

fn string_to_null_terminated_bytes(s: String) -> *mut u8 {
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
