// src/hw/nvs_store.rs
//! Trừu tượng hóa việc lưu trữ persistent snapshot xuống bộ nhớ Flash NVS.

use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use log::{info, warn};

use crate::core::fsm::context::{NvsSnapshot, SystemContext};

pub struct NvsStore {
    nvs: Option<EspDefaultNvs>,
}

impl NvsStore {
    pub fn new(nvs_partition: EspDefaultNvsPartition) -> Self {
        let nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
        Self { nvs }
    }

    pub fn load_or_init_device_id(&mut self, default_id: &str) -> String {
        if let Some(nvs) = self.nvs.as_mut() {
            let mut buf = [0u8; 64];
            if let Ok(Some(saved_id)) = nvs.get_str("device_id", &mut buf) {
                return saved_id.to_string();
            } else {
                let _ = nvs.set_str("device_id", default_id);
            }
        }
        default_id.to_string()
    }

    pub fn load_runtime_snapshot(&mut self, ctx: &mut SystemContext) {
        if let Some(nvs) = self.nvs.as_mut() {
            let mut buf = [0u8; 2048];
            if let Ok(Some(raw)) = nvs.get_str("runtime_snap", &mut buf) {
                if let Ok(snapshot) = serde_json::from_str::<NvsSnapshot>(raw) {
                    info!("📦 Khôi phục thành công NvsSnapshot từ Flash!");
                    ctx.tuner.adaptive_ec_ratio = snapshot.step_ratio_ec.clamp(0.1, 2.0);
                    ctx.tuner.best_ec_ratio = snapshot.best_ec_ratio.clamp(0.1, 2.0);
                    ctx.tuner.adaptive_ph_ratio = snapshot.step_ratio_ph.clamp(0.1, 2.0);
                    ctx.tuner.best_ph_ratio = snapshot.best_ph_ratio.clamp(0.1, 2.0);
                    ctx.dosing.retry_ec = snapshot.retry_ec;
                    ctx.dosing.retry_ph = snapshot.retry_ph;
                    ctx.dosing_cycle_count = snapshot.dosing_cycle_count;
                    ctx.last_water_change_sec = snapshot.last_water_change_sec;
                }
            }
        }
    }

    pub fn save_snapshot(&mut self, ctx: &SystemContext, now_sec: u64) {
        if let Some(nvs) = self.nvs.as_mut() {
            let snapshot = NvsSnapshot::from_context(ctx, now_sec);
            if let Ok(serialized) = serde_json::to_string(&snapshot) {
                if let Err(e) = nvs.set_str("runtime_snap", &serialized) {
                    warn!("⚠️ Lỗi khi lưu NvsSnapshot: {:?}", e);
                }
            }
        }
    }

    pub fn save_last_water_change(&mut self, timestamp_sec: u64) {
        if let Some(nvs) = self.nvs.as_mut() {
            let _ = nvs.set_u64("last_w_change", timestamp_sec);
        }
    }
}