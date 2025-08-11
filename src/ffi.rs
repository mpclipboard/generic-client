use anyhow::{Context as _, Result};
use std::ffi::c_char;

pub(crate) fn string_to_cstring(s: String) -> *mut c_char {
    match std::ffi::CString::new(s) {
        Ok(text) => {
            let mut bytes = text.into_bytes_with_nul();
            let ptr = bytes.as_mut_ptr();
            std::mem::forget(bytes);
            ptr.cast()
        }
        Err(_) => {
            log::error!("clip text is NULL terminated");
            std::ptr::null_mut()
        }
    }
}

pub(crate) fn cstring_to_string(s: *const c_char) -> Result<String> {
    Ok(unsafe { std::ffi::CStr::from_ptr(s) }
        .to_str()
        .context("failed to convert *char to String")?
        .to_string())
}
