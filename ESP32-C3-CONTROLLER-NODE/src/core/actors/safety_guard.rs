use std::collections::HashMap;

pub struct SafetyGuard {
    hourly_doses: HashMap<String, Vec<(u64, f32)>>,
    refill_history: Vec<u64>,
    drain_history: Vec<u64>,
    pub manual_timeouts: HashMap<String, u64>,
    pub safety_override_until: u64,
    pub last_ec_before_dose: Option<f32>,
    pub last_ph_before_dose: Option<f32>,
}

impl SafetyGuard {
    pub fn new() -> Self {
        Self {
            hourly_doses: HashMap::new(),
            refill_history: Vec::new(),
            drain_history: Vec::new(),
            manual_timeouts: HashMap::new(),
            safety_override_until: 0,
            last_ec_before_dose: None,
            last_ph_before_dose: None,
        }
    }

    pub fn check_hourly_dose(
        &mut self,
        pump: &str,
        now_sec: u64,
        dose_ml: f32,
        max_ml: f32,
    ) -> bool {
        // Lay lich su bom (thoi gian bat, luong cham) va loai bor nhung lich su ngoai pham vi 1 gio
        let history = self.hourly_doses.entry(pump.to_string()).or_default();
        history.retain(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600);
        let total = history.iter().map(|(_, ml)| *ml).sum::<f32>();
        if total + dose_ml > max_ml {
            return false;
        }
        history.push((now_sec, dose_ml));
        true
    }

    /// Kiểm tra xem dose có vượt budget không, KHÔNG ghi vào lịch sử.
    pub fn peek_hourly_dose(&self, pump: &str, now_sec: u64, dose_ml: f32, max_ml: f32) -> bool {
        let history = match self.hourly_doses.get(pump) {
            Some(h) => h,
            None => return true,
        };
        let total: f32 = history
            .iter()
            .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
            .map(|(_, ml)| *ml)
            .sum();
        total + dose_ml <= max_ml
    }

    /// Ghi dose vào lịch sử mà không kiểm tra. Chỉ gọi sau khi peek đã pass.
    pub fn commit_hourly_dose(&mut self, pump: &str, now_sec: u64, dose_ml: f32) {
        let history = self.hourly_doses.entry(pump.to_string()).or_default();
        history.retain(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600);
        history.push((now_sec, dose_ml));
    }

    pub fn record_drain(&mut self, now_sec: u64, max: u32) -> bool {
        self.drain_history
            .retain(|ts| now_sec.saturating_sub(*ts) <= 3600);
        if self.drain_history.len() as u32 >= max {
            return false;
        }
        self.drain_history.push(now_sec);
        true
    }

    pub fn record_refill(&mut self, now_sec: u64, max: u32) -> bool {
        self.refill_history
            .retain(|ts| now_sec.saturating_sub(*ts) <= 3600);
        if self.refill_history.len() as u32 >= max {
            return false;
        }
        self.refill_history.push(now_sec);
        true
    }

    pub fn hourly_doses(&self) -> &HashMap<String, Vec<(u64, f32)>> {
        &self.hourly_doses
    }

    pub fn refill_history(&self) -> &[u64] {
        &self.refill_history
    }

    pub fn drain_history(&self) -> &[u64] {
        &self.drain_history
    }

    pub fn flush_for_reset(&mut self) {
        self.hourly_doses.clear();
        self.refill_history.clear();
        self.drain_history.clear();
        self.last_ec_before_dose = None;
        self.last_ph_before_dose = None;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_not_consumed_when_second_pump_fails() {
        let mut guard = SafetyGuard::new();
        let now_sec = 1000u64;
        let max_ml = 10.0f32;

        assert!(guard.peek_hourly_dose("NutrientA", now_sec, 5.0, max_ml));
        guard.commit_hourly_dose("NutrientA", now_sec, 5.0);
        assert!(!guard.peek_hourly_dose("NutrientB", now_sec, 11.0, max_ml));
        assert!(guard.peek_hourly_dose("NutrientA", now_sec, 5.0, max_ml));
    }

    #[test]
    fn old_check_hourly_dose_keeps_backward_compat_commit_behavior() {
        let mut guard = SafetyGuard::new();
        assert!(guard.check_hourly_dose("PumpA", 1000, 5.0, 10.0));
        assert!(!guard.check_hourly_dose("PumpA", 1000, 6.0, 10.0));
    }
}
