use crate::ffi::string_to_cstring;
use std::ffi::c_char;

#[repr(C)]
pub struct Output {
    pub text: *mut c_char,
    pub connectivity: *mut bool,
}

impl Output {
    pub(crate) fn null() -> Self {
        Self {
            text: std::ptr::null_mut(),
            connectivity: std::ptr::null_mut(),
        }
    }

    pub(crate) fn new(text: Option<String>, connectivity: Option<bool>) -> Self {
        let mut out = Self::null();
        if let Some(text) = text {
            out.text = string_to_cstring(text);
        }
        if let Some(connectivity) = connectivity {
            out.connectivity = Box::leak(Box::new(connectivity));
        }
        out
    }
}
