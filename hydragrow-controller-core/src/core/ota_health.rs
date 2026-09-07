//! Pure, host-testable gating for the ESP-IDF app-rollback safety net.
//! Ensures the firmware crate calls `EspOta::mark_running_slot_valid()`
//! exactly once per boot, and only after the device has proven itself
//! healthy by reaching MQTT. This module has no esp-idf dependency — the
//! actual esp-idf call lives in ESP32-C3-CONTROLLER-NODE/src/runtime/health.rs.

/// Tracks whether the running firmware image has already been confirmed
/// healthy this boot, so the bootloader's pending-rollback flag is cleared
/// exactly once.
#[derive(Debug, Default)]
pub struct OtaValidationGate {
    marked: bool,
}

impl OtaValidationGate {
    pub fn new() -> Self {
        Self { marked: false }
    }

    /// Call this every time the device reaches a "known healthy" event
    /// (e.g. MQTT connected). Returns `true` exactly once — the caller
    /// should call `EspOta::mark_running_slot_valid()` only when this
    /// returns `true`.
    pub fn mark_if_needed(&mut self) -> bool {
        if self.marked {
            return false;
        }
        self.marked = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_call_returns_true() {
        let mut gate = OtaValidationGate::new();
        assert!(gate.mark_if_needed());
    }

    #[test]
    fn second_call_returns_false() {
        let mut gate = OtaValidationGate::new();
        assert!(gate.mark_if_needed());
        assert!(!gate.mark_if_needed());
    }

    #[test]
    fn many_calls_only_mark_once() {
        let mut gate = OtaValidationGate::new();
        let marks: Vec<bool> = (0..5).map(|_| gate.mark_if_needed()).collect();
        assert_eq!(marks, vec![true, false, false, false, false]);
    }
}
