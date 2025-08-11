use crate::Clip;

#[repr(C)]
pub struct Output {
    pub clip: *mut Clip,
    pub connectivity: *mut bool,
}

impl Output {
    pub(crate) fn null() -> Self {
        Self {
            clip: std::ptr::null_mut(),
            connectivity: std::ptr::null_mut(),
        }
    }

    pub(crate) fn new(clip: Option<Clip>, connectivity: Option<bool>) -> Self {
        let mut out = Self::null();
        if let Some(clip) = clip {
            out.clip = Box::into_raw(Box::new(clip));
        }
        if let Some(connectivity) = connectivity {
            out.connectivity = Box::leak(Box::new(connectivity));
        }
        out
    }
}
