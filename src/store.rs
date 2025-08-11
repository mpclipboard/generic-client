use crate::clip::Clip;

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

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_store_drop(store: *mut Store) {
    unsafe { std::ptr::drop_in_place(store) };
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_store_add(store: *mut Store, clip: *mut Clip) -> bool {
    let Some(store) = (unsafe { store.as_mut() }) else {
        log::error!("NULL store");
        return false;
    };

    let clip = unsafe { Box::from_raw(clip) };
    store.add(&clip)
}
