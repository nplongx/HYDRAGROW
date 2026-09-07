// src/hw/nvs_store.rs
//! Trạng thái và phục hồi persistent snapshot xuống bộ Flash NVS.

use anyhow::{anyhow, Result};
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use hydragrow_controller_core::core::fsm::context::{NvsSnapshot, SystemContext};
use hydragrow_shared::recipe::CropRecipe;
use log::{info, warn};

const ACTIVE_RECIPE_KEY: &str = "active_recipe";
const ACTIVE_RECIPE_BUF_SIZE: usize = 4096;

/// Deterministic factory identity from the WiFi station eFuse MAC.
/// Used only when neither NVS nor a compile-time default provides an id,
/// so release firmware never needs a per-device baked-in identity.
fn factory_device_id() -> String {
    // SAFETY: esp_read_mac writes exactly 6 bytes into the provided buffer.
    unsafe {
        let mut mac = [0u8; 6];
        let err = esp_idf_sys::esp_read_mac(
            mac.as_mut_ptr(),
            esp_idf_sys::esp_mac_type_t_ESP_MAC_WIFI_STA,
        );
        if err == esp_idf_sys::ESP_OK as i32 {
            return hydragrow_controller_core::device_identity::format_factory_id(&mac);
        }
    }
    "esp32c3-unknown".to_string()
}

pub struct NvsStore {
    nvs: Option<EspDefaultNvs>,
}

impl NvsStore {
    pub fn new(nvs_partition: EspDefaultNvsPartition) -> Self {
        let nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
        Self { nvs }
    }

    pub fn load_active_recipe(&mut self) -> Result<Option<CropRecipe>> {
        let Some(nvs) = self.nvs.as_mut() else {
            return Ok(None);
        };

        let mut buf = [0u8; ACTIVE_RECIPE_BUF_SIZE];
        let Some(raw) = nvs.get_str(ACTIVE_RECIPE_KEY, &mut buf)? else {
            return Ok(None);
        };

        match serde_json::from_str::<CropRecipe>(raw) {
            Ok(recipe) => Ok(Some(recipe)),
            Err(error) => {
                warn!(
                    "recipe_rejected: active recipe JSON in NVS is invalid; clearing key: {:?}",
                    error
                );
                self.clear_active_recipe()?;
                Ok(None)
            }
        }
    }

    pub fn save_active_recipe(&mut self, recipe: &CropRecipe) -> Result<()> {
        let nvs = self
            .nvs
            .as_mut()
            .ok_or_else(|| anyhow!("NVS namespace 'agitech' is not available"))?;
        let serialized = serde_json::to_string(recipe)?;
        nvs.set_str(ACTIVE_RECIPE_KEY, &serialized)?;
        Ok(())
    }

    pub fn clear_active_recipe(&mut self) -> Result<()> {
        let nvs = self
            .nvs
            .as_mut()
            .ok_or_else(|| anyhow!("NVS namespace 'agitech' is not available"))?;
        nvs.remove(ACTIVE_RECIPE_KEY)?;
        Ok(())
    }

    pub fn load_or_init_device_id(&mut self, default_id: &str) -> String {
        let saved: Option<String> = self.nvs.as_mut().and_then(|nvs| {
            let mut buf = [0u8; 64];
            match nvs.get_str("device_id", &mut buf) {
                Ok(Some(id)) => Some(id.to_string()),
                _ => None,
            }
        });
        let resolved = hydragrow_controller_core::device_identity::resolve_device_id(
            saved.as_deref(),
            default_id,
            &factory_device_id(),
        );
        // Persist the resolved id so the fleet identity is stable across boots.
        if let Some(nvs) = self.nvs.as_mut() {
            if saved.as_deref() != Some(resolved.as_str()) {
                let _ = nvs.set_str("device_id", &resolved);
            }
        }
        info!("🆔 [NVS] device_id resolved: {}", resolved);
        resolved
    }

    pub fn load_or_init_mqtt_credentials(
        &mut self,
        default_user: &str,
        default_pass: &str,
    ) -> (String, String) {
        if let Some(nvs) = self.nvs.as_mut() {
            let mut user_buf = [0u8; 64];
            let mut pass_buf = [0u8; 128];

            let user = if let Ok(Some(u)) = nvs.get_str("mqtt_user", &mut user_buf) {
                u.to_string()
            } else {
                let _ = nvs.set_str("mqtt_user", default_user);
                default_user.to_string()
            };

            let pass = if let Ok(Some(p)) = nvs.get_str("mqtt_pass", &mut pass_buf) {
                p.to_string()
            } else {
                let _ = nvs.set_str("mqtt_pass", default_pass);
                default_pass.to_string()
            };

            (user, pass)
        } else {
            (default_user.to_string(), default_pass.to_string())
        }
    }

    pub fn load_runtime_snapshot(&mut self, ctx: &mut SystemContext) {
        if let Some(nvs) = self.nvs.as_mut() {
            let mut buf = [0u8; 2048];
            if let Ok(Some(raw)) = nvs.get_str("runtime_snap", &mut buf) {
                if let Ok(snapshot) = serde_json::from_str::<NvsSnapshot>(raw) {
                    info!("Khôi phục thành công NvsSnapshot từ Flash!");

                    // Ưu tiên đọc biến A/B mới, nếu chưa có (lỗi parse default = 0.0) thì lấy biến gộp cũ
                    ctx.tuner.adaptive_ec_a_ratio = if snapshot.step_ratio_ec_a > 0.0 {
                        snapshot.step_ratio_ec_a
                    } else {
                        snapshot.step_ratio_ec
                    }
                    .clamp(0.1, 2.0);
                    ctx.tuner.adaptive_ec_b_ratio = if snapshot.step_ratio_ec_b > 0.0 {
                        snapshot.step_ratio_ec_b
                    } else {
                        snapshot.step_ratio_ec
                    }
                    .clamp(0.1, 2.0);

                    ctx.tuner.best_ec_a_ratio = if snapshot.best_ec_a_ratio > 0.0 {
                        snapshot.best_ec_a_ratio
                    } else {
                        snapshot.best_ec_ratio
                    }
                    .clamp(0.1, 2.0);
                    ctx.tuner.best_ec_b_ratio = if snapshot.best_ec_b_ratio > 0.0 {
                        snapshot.best_ec_b_ratio
                    } else {
                        snapshot.best_ec_ratio
                    }
                    .clamp(0.1, 2.0);

                    ctx.tuner.adaptive_ph_up_ratio = if snapshot.step_ratio_ph_up > 0.0 {
                        snapshot.step_ratio_ph_up
                    } else {
                        snapshot.step_ratio_ph
                    }
                    .clamp(0.05, 1.0);
                    ctx.tuner.adaptive_ph_down_ratio = if snapshot.step_ratio_ph_down > 0.0 {
                        snapshot.step_ratio_ph_down
                    } else {
                        snapshot.step_ratio_ph
                    }
                    .clamp(0.05, 1.0);

                    ctx.tuner.best_ph_up_ratio = if snapshot.best_ph_up_ratio > 0.0 {
                        snapshot.best_ph_up_ratio
                    } else {
                        snapshot.best_ph_ratio
                    }
                    .clamp(0.05, 1.0);
                    ctx.tuner.best_ph_down_ratio = if snapshot.best_ph_down_ratio > 0.0 {
                        snapshot.best_ph_down_ratio
                    } else {
                        snapshot.best_ph_ratio
                    }
                    .clamp(0.05, 1.0);

                    ctx.dosing.retry_ec = snapshot.retry_ec;
                    ctx.dosing.retry_ph = snapshot.retry_ph;
                    ctx.dosing_cycle_count = snapshot.dosing_cycle_count;
                    ctx.last_water_change_sec = snapshot.last_water_change_sec;
                    // hourly_dose_ec_ml và hourly_dose_ph_ml KHÔNG được restore vào ctx.safety
                    // vì timestamps trong safety guard dùng uptime_sec (bắt đầu từ 0 sau reboot),
                    // còn giá trị trong NVS được lưu theo wall-clock.
                    // Budget sẽ tự tích lũy lại từ 0 sau reboot.
                    ctx.current_stage_index = snapshot.current_stage_index;
                }
            }

            if ctx.current_stage_index.is_none() {
                if let Ok(Some(stage_index)) = nvs.get_u64("current_stage") {
                    if stage_index != u64::MAX {
                        ctx.current_stage_index = Some(stage_index as usize);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn factory_reset(&mut self) -> Result<()> {
        if let Some(nvs) = self.nvs.as_mut() {
            let _ = nvs.remove(ACTIVE_RECIPE_KEY);
            let _ = nvs.remove("runtime_snap");
            let _ = nvs.remove("current_stage");
            let _ = nvs.remove("last_w_change");
            let _ = nvs.remove("safety_budget");
        }
        Ok(())
    }
}
