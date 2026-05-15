use std::sync::mpsc::Sender;

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_shared::ControllerConfig;

use crate::{config::SharedConfig, mqtt::SensorData, pump::PumpController};

use super::{phases::SystemPhase, system_context::SystemContext};

#[allow(clippy::too_many_arguments)]
pub fn tick(
    now_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut SystemContext,
    pumps: &mut PumpController,
    shared_config: &SharedConfig,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
    mqtt_tx: &Sender<String>,
) {
    let _ = (config, sensors, pumps, shared_config, nvs, dosing_report_tx, mqtt_tx);

    match &ctx.phase {
        SystemPhase::Monitoring => {}
        SystemPhase::DosingEC | SystemPhase::DosingPH => {
            let _ = ctx.dosing.tick(now_ms, config, pumps);
        }
        SystemPhase::WaterRefilling | SystemPhase::WaterDraining => {}
        SystemPhase::ActiveMixing | SystemPhase::Stabilizing | SystemPhase::Cooldown => {}
        SystemPhase::Fault(_) | SystemPhase::EmergencyStop(_) => {}
        _ => {}
    }
}
