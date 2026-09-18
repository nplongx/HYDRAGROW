//! Deterministic clock for Digital Twin execution and tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualClock {
    now_ms: u64,
    uptime_ms: u64,
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self {
            now_ms: 1_700_000_000_000,
            uptime_ms: 0,
        }
    }
}

impl VirtualClock {
    pub fn new(now_ms: u64) -> Self {
        Self {
            now_ms,
            uptime_ms: 0,
        }
    }
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }
    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }
    pub fn advance(&mut self, dt_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(dt_ms);
        self.uptime_ms = self.uptime_ms.saturating_add(dt_ms);
    }
    pub fn jump_wall_clock(&mut self, delta_ms: i64) {
        if delta_ms >= 0 {
            self.now_ms = self.now_ms.saturating_add(delta_ms as u64);
        } else {
            self.now_ms = self.now_ms.saturating_sub(delta_ms.unsigned_abs());
        }
    }
    pub fn restart(&mut self) {
        self.uptime_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advance_and_restart_are_deterministic() {
        let mut c = VirtualClock::new(1_000);
        c.advance(250);
        assert_eq!((c.now_ms(), c.uptime_ms()), (1_250, 250));
        c.restart();
        assert_eq!((c.now_ms(), c.uptime_ms()), (1_250, 0));
    }
}
