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

/// # Safety
///
/// `clip` must be a valid pointer to Clip
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_clip_get_text(clip: *const Clip) -> *mut c_char {
    let clip = unsafe { &*clip };
    string_to_cstring(clip.text.clone())
}

/// # Safety
///
/// `clip` must be a valid pointer to Clip
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_clip_drop(clip: *mut Clip) {
    unsafe { std::ptr::drop_in_place(clip) };
}
