use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Clip {
    pub(crate) text: String,
    pub(crate) timestamp: u128,
}

impl Clip {
    pub(crate) fn new(text: &str) -> Self {
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
