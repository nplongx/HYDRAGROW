//! DosingAnalyticsObserver — Tích lũy thống kê dosing, xuất analytics report.
//!
//! Subscribe: PublishDosingReport, SetDosingPump
//! Output: Analytics JSON → dosing_report_tx

use super::ObserverContext;
use crate::core::fsm::events::OrchestratorEvent;
use log::warn;

/// Thống kê tích lũy trong một cửa sổ thời gian (rolling window)
#[derive(Debug, Default)]
pub struct DosingWindow {
    pub nutrient_a_ml_total: f32,
    pub nutrient_b_ml_total: f32,
    pub ph_up_ml_total: f32,
    pub ph_down_ml_total: f32,
    pub cycle_count: u32,
    pub successful_cycles: u32, // Cycles đạt target EC/pH trong tolerance
    pub window_start_ms: u64,
}

pub struct DosingAnalyticsObserver {
    /// Cửa sổ thống kê hiện tại (1 giờ rolling)
    pub current_window: DosingWindow,
    /// Cửa sổ đã hoàn tất (giờ trước)
    pub previous_window: Option<DosingWindow>,
    /// Tổng cycle count kể từ boot
    pub total_cycles: u64,
    /// Moving average delta EC per ml (exponential)
    pub ema_delta_ec_per_ml: f32,
    /// Moving average delta pH per ml
    pub ema_delta_ph_per_ml: f32,
    /// EMA alpha
    alpha: f32,
}

impl DosingAnalyticsObserver {
    pub fn new() -> Self {
        Self {
            current_window: DosingWindow::default(),
            previous_window: None,
            total_cycles: 0,
            ema_delta_ec_per_ml: 0.0,
            ema_delta_ph_per_ml: 0.0,
            alpha: 0.15,
        }
    }

    pub fn on_event(&mut self, event: &OrchestratorEvent, oc: &ObserverContext<'_>) {
        if let OrchestratorEvent::PublishDosingReport { report_json } = event {
            self.handle_dosing_report(report_json, oc);
        }
    }

    fn handle_dosing_report(&mut self, report_json: &str, oc: &ObserverContext<'_>) {
        // Parse report để extract metrics
        let report: serde_json::Value = match serde_json::from_str(report_json) {
            Ok(v) => v,
            Err(_) => return,
        };

        // Roll window nếu đã qua 1 giờ
        let window_elapsed_ms = oc
            .now_ms
            .saturating_sub(self.current_window.window_start_ms);
        if window_elapsed_ms > 3_600_000 {
            let completed = std::mem::take(&mut self.current_window);
            self.previous_window = Some(completed);
            self.current_window.window_start_ms = oc.now_ms;
        }

        // Cộng dồn ml vào cửa sổ hiện tại
        if let Some(dose) = report.get("dose") {
            self.current_window.nutrient_a_ml_total += dose
                .get("pump_a_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            self.current_window.nutrient_b_ml_total += dose
                .get("pump_b_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            self.current_window.ph_up_ml_total +=
                dose.get("ph_up_ml").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            self.current_window.ph_down_ml_total += dose
                .get("ph_down_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
        }

        self.current_window.cycle_count += 1;
        self.total_cycles = self.total_cycles.saturating_add(1);

        // Cập nhật EMA hiệu suất
        let delta_ec = report
            .get("delta_ec")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        let total_nutrient =
            self.current_window.nutrient_a_ml_total + self.current_window.nutrient_b_ml_total;

        if total_nutrient > 0.1 && delta_ec > 0.0 {
            let observed = delta_ec / total_nutrient;
            self.ema_delta_ec_per_ml = if self.ema_delta_ec_per_ml == 0.0 {
                observed
            } else {
                self.alpha * observed + (1.0 - self.alpha) * self.ema_delta_ec_per_ml
            };
        }

        // Kiểm tra cycle có thành công không (error_ec và error_ph đều trong tolerance)
        let error_ec = report
            .get("error_ec")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::MAX)
            .abs();
        let error_ph = report
            .get("error_ph")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::MAX)
            .abs();
        if error_ec < oc.config.tds_tolerance as f64 && error_ph < oc.config.ph_tolerance as f64 {
            self.current_window.successful_cycles += 1;
        }

        // Gửi analytics enriched report
        let success_rate = if self.current_window.cycle_count > 0 {
            self.current_window.successful_cycles as f32 / self.current_window.cycle_count as f32
        } else {
            0.0
        };

        let analytics_payload = serde_json::json!({
            "type": "dosing_analytics",
            "device_id": oc.config.device_id,
            "timestamp_ms": oc.now_ms,
            "raw_report": report,
            "analytics": {
                "total_cycles_boot": self.total_cycles,
                "current_window": {
                    "duration_ms": window_elapsed_ms,
                    "nutrient_a_ml": self.current_window.nutrient_a_ml_total,
                    "nutrient_b_ml": self.current_window.nutrient_b_ml_total,
                    "ph_up_ml": self.current_window.ph_up_ml_total,
                    "ph_down_ml": self.current_window.ph_down_ml_total,
                    "cycle_count": self.current_window.cycle_count,
                    "success_rate": success_rate,
                },
                "ema_delta_ec_per_ml": self.ema_delta_ec_per_ml,
                "ema_delta_ph_per_ml": self.ema_delta_ph_per_ml,
            }
        })
        .to_string();

        if oc.dosing_report_tx.send(analytics_payload).is_err() {
            warn!("⚠️ [ANALYTICS] dosing_report channel full");
        }
    }
}
