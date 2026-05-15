use std::collections::HashMap;

pub struct SafetyGuard {
    hourly_doses: HashMap<String, Vec<(u64, f32)>>,
    refill_history: Vec<u64>,
    drain_history: Vec<u64>,
    pub manual_timeouts: HashMap<String, u64>,
    pub safety_override_until: u64,
    pub last_ec_before_dose: Option<f32>,
    pub last_ph_before_dose: Option<f32>,
    pub last_ph_dose_up: Option<bool>,
    pub last_water_before_refill: Option<f32>,
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
            last_ph_dose_up: None,
            last_water_before_refill: None,
        }
    }

    pub fn check_hourly_dose(
        &mut self,
        pump: &str,
        now_sec: u64,
        dose_ml: f32,
        max_ml: f32,
    ) -> bool {
        let history = self.hourly_doses.entry(pump.to_string()).or_default();
        history.retain(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600);
        let total = history.iter().map(|(_, ml)| *ml).sum::<f32>();
        if total + dose_ml > max_ml {
            return false;
        }
        history.push((now_sec, dose_ml));
        true
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

    pub fn replace_hourly_histories(
        &mut self,
        doses: HashMap<String, Vec<(u64, f32)>>,
        refill: Vec<u64>,
        drain: Vec<u64>,
    ) {
        self.hourly_doses = doses;
        self.refill_history = refill;
        self.drain_history = drain;
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
        self.last_water_before_refill = None;
    }
}
