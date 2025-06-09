#[derive(Clone, Copy)]
pub(crate) struct Retry {
    attempts_count: u64,
}
impl Retry {
    pub(crate) fn starting() -> Self {
        Self { attempts_count: 0 }
    }

    pub(crate) fn track(&mut self) {
        self.attempts_count += 1
    }

    pub(crate) fn delay(&self) -> u64 {
        const MAX_DELAY: u64 = 30;
        let delay = 2_u64.pow(self.attempts_count as u32).clamp(0, MAX_DELAY);
        log::warn!(
            "[retry] attempts = {}, delay = {delay}",
            self.attempts_count
        );
        delay
    }
}
