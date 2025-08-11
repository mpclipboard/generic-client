use crate::ffi::string_to_cstring;
use serde::{Deserialize, Serialize};
use std::{
    ffi::c_char,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Clip {
    pub text: String,
    pub timestamp: u128,
}

impl Clip {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis(),
        }
    }

    pub(crate) fn newer_than(&self, other: &Clip) -> bool {
        self.timestamp > other.timestamp && self.text != other.text
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_clip_get_text(clip: *const Clip) -> *mut c_char {
    let Some(clip) = (unsafe { clip.as_ref() }) else {
        log::error!("NULL clip");
        return std::ptr::null_mut();
    };

    string_to_cstring(clip.text.clone())
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_clip_drop(clip: *mut Clip) {
    unsafe { std::ptr::drop_in_place(clip) };
}
