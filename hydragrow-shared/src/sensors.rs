//! Sensor ingress, deserialization, and validation.

use crate::SensorData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
pub struct IncomingSensorPayload {
    pub temp: Option<f32>,
    #[serde(alias = "tds")]
    pub ec: Option<f32>,
    pub ph: Option<f32>,
    pub water_level: Option<f32>,
    pub ph_voltage_mv: Option<f32>,
    pub time: Option<String>,
    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,
    pub is_continuous: Option<bool>,
    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    #[serde(alias = "err_tds")]
    pub err_ec: Option<bool>,
}

impl IncomingSensorPayload {
    /// Checks if this payload contains any sensor measurements.
    pub fn has_measurements(&self) -> bool {
        self.temp.is_some()
            || self.ec.is_some()
            || self.ph.is_some()
            || self.water_level.is_some()
            || self.ph_voltage_mv.is_some()
    }

    /// Checks whether all provided measurements are finite and physically valid.
    pub fn is_valid(&self) -> bool {
        if !self.has_measurements() {
            return false;
        }

        if self
            .temp
            .is_some_and(|t| !t.is_finite() || !(-50.0..=100.0).contains(&t))
        {
            return false;
        }
        if self
            .ec
            .is_some_and(|e| !e.is_finite() || !(0.0..=20.0).contains(&e))
        {
            return false;
        }
        if self
            .ph
            .is_some_and(|p| !p.is_finite() || !(0.0..=14.0).contains(&p))
        {
            return false;
        }
        if self
            .water_level
            .is_some_and(|w| !w.is_finite() || !(0.0..=500.0).contains(&w))
        {
            return false;
        }
        if self
            .ph_voltage_mv
            .is_some_and(|mv| !mv.is_finite() || !(0.0..=5000.0).contains(&mv))
        {
            return false;
        }

        true
    }
}

impl SensorData {
    /// Field-wise merge of incoming sensor payload:
    /// - Only overwrites fields that are `Some`.
    /// - Advances `controller_received_ms` ONLY if the payload contains valid measurements.
    /// - Returns `true` if accepted, `false` otherwise.
    pub fn merge_incoming_payload(
        &mut self,
        payload: &IncomingSensorPayload,
        now_uptime_ms: u64,
    ) -> bool {
        if !payload.is_valid() {
            return false;
        }

        if let Some(t) = payload.temp {
            self.temp = t;
        }
        if let Some(e) = payload.ec {
            self.ec = e;
        }
        if let Some(p) = payload.ph {
            self.ph = p;
        }
        if let Some(w) = payload.water_level {
            self.water_level = w;
        }
        if let Some(mv) = payload.ph_voltage_mv {
            self.ph_voltage_mv = Some(mv as f64);
        }
        if let Some(cont) = payload.is_continuous {
            self.is_continuous = Some(cont);
        }
        if let Some(ew) = payload.err_water {
            self.err_water = Some(ew);
        }
        if let Some(et) = payload.err_temp {
            self.err_temp = Some(et);
        }
        if let Some(ee) = payload.err_ec {
            self.err_ec = Some(ee);
        }
        if let Some(ep) = payload.err_ph {
            self.err_ph = Some(ep);
        }
        if let Some(rssi) = payload.rssi {
            self.rssi = Some(rssi);
        }
        if let Some(free_heap) = payload.free_heap {
            self.free_heap = Some(free_heap);
        }
        if let Some(uptime) = payload.uptime {
            self.uptime = Some(uptime);
        }
        if let Some(ref time) = payload.time {
            self.time = time.clone();
        }

        self.controller_received_ms = Some(now_uptime_ms);
        true
    }
}
