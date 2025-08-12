use std::ffi::c_char;

use crate::{clip::Clip, ffi::cstring_to_string};

#[derive(Default)]
pub struct Store {
    clip: Option<Clip>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn add(&mut self, clip: &Clip) -> bool {
        let do_update = self.clip.is_none()
            || self
                .clip
                .as_ref()
                .is_some_and(|current| clip.newer_than(current));

        if do_update {
            self.clip = Some(clip.clone());
        }

        do_update
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_store_new() -> *mut Store {
    Box::into_raw(Box::new(Store::new()))
}

/// # Safety
///
/// `store` must be a valid pointer to Store
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_store_drop(store: *mut Store) {
    unsafe { std::ptr::drop_in_place(store) };
}

/// # Safety
///
/// `store` must be a valid pointer to Store
/// `clip` must be a valid pointer to Clip
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_store_add_clip(store: *mut Store, clip: *const Clip) -> bool {
    let store = unsafe { &mut *store };
    let clip = unsafe { &*clip };
    store.add(clip)
}

/// # Safety
///
/// `store` must be a valid pointer to Store
/// `text` must be a NULL-terminated C String
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mpclipboard_store_add_text(
    store: *mut Store,
    text: *const c_char,
) -> bool {
    let store = unsafe { &mut *store };
    let Ok(text) = cstring_to_string(text) else {
        log::error!("NULL text");
        return false;
    };
    let clip = Clip::new(&text);
    store.add(&clip)
}
