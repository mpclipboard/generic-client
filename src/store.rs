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
