use crate::{ControllerConfig, SensorData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HestiaState {
    Comfortable,
    Warning,
    Critical,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HestiaAction {
    None,
    EcDosing,
    PhDosing,
    WaterRefill,
    WaterDrain,
    Misting,
    Mixing,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HestiaTrendDirection {
    Stable,
    Improving,
    Degrading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HestiaContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<SensorData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes_since_previous: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minutes_since_last_intervention: Option<f32>,
    pub last_action: HestiaAction,
    pub matrix_is_warm: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean_kalman_confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
}

impl Default for HestiaContext {
    fn default() -> Self {
        Self {
            previous: None,
            minutes_since_previous: None,
            minutes_since_last_intervention: None,
            last_action: HestiaAction::None,
            matrix_is_warm: false,
            mean_kalman_confidence: None,
            phase: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HestiaAxisAssessment {
    pub comfort: f32,
    pub weight: f32,
    pub trend: HestiaTrendDirection,
    pub trend_factor: f32,
    pub action_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HestiaAxesAssessment {
    pub ec: HestiaAxisAssessment,
    pub ph: HestiaAxisAssessment,
    pub water_level: HestiaAxisAssessment,
    pub temp: HestiaAxisAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HestiaAssessment {
    pub score: f32,
    pub state: HestiaState,
    pub confidence: f32,
    pub axes: HestiaAxesAssessment,
    pub reasons: Vec<String>,
}

pub struct HestiaEngine;

impl HestiaEngine {
    pub fn evaluate(
        current: &SensorData,
        config: &ControllerConfig,
        context: &HestiaContext,
    ) -> HestiaAssessment {
        let ec_comfort = if config.enable_ec_sensor {
            target_comfort(
                current.ec,
                config.ec_target,
                config.ec_tolerance,
                config.min_ec_limit,
                config.max_ec_limit,
            )
        } else {
            1.0
        };
        let ph_comfort = if config.enable_ph_sensor {
            target_comfort(
                current.ph,
                config.ph_target,
                config.ph_tolerance,
                config.min_ph_limit,
                config.max_ph_limit,
            )
        } else {
            1.0
        };
        let water_comfort = if config.enable_water_level_sensor {
            target_comfort(
                current.water_level,
                config.water_level_target,
                config.water_level_tolerance,
                config.water_level_min,
                config.water_level_max,
            )
        } else {
            1.0
        };
        let temp_comfort = if config.enable_temp_sensor {
            trapezoid_comfort(
                current.temp,
                config.min_temp_limit,
                22.0,
                28.0_f32.min(config.misting_temp_threshold.max(22.0)),
                config.max_temp_limit,
            )
        } else {
            1.0
        };

        let previous = context.previous.as_ref();
        let minutes = context.minutes_since_previous.unwrap_or(30.0).max(1.0);

        let mut ec = axis(
            ec_comfort,
            previous.map(|s| {
                target_comfort(
                    s.ec,
                    config.ec_target,
                    config.ec_tolerance,
                    config.min_ec_limit,
                    config.max_ec_limit,
                )
            }),
            minutes,
            0.35,
            action_factor(context, HestiaAction::EcDosing),
        );
        let mut ph = axis(
            ph_comfort,
            previous.map(|s| {
                target_comfort(
                    s.ph,
                    config.ph_target,
                    config.ph_tolerance,
                    config.min_ph_limit,
                    config.max_ph_limit,
                )
            }),
            minutes,
            0.30,
            action_factor(context, HestiaAction::PhDosing),
        );
        let mut water_level = axis(
            water_comfort,
            previous.map(|s| {
                target_comfort(
                    s.water_level,
                    config.water_level_target,
                    config.water_level_tolerance,
                    config.water_level_min,
                    config.water_level_max,
                )
            }),
            minutes,
            0.20,
            action_factor_for_any(
                context,
                &[HestiaAction::WaterRefill, HestiaAction::WaterDrain],
            ),
        );
        let mut temp = axis(
            temp_comfort,
            previous.map(|s| {
                trapezoid_comfort(
                    s.temp,
                    config.min_temp_limit,
                    22.0,
                    28.0_f32.min(config.misting_temp_threshold.max(22.0)),
                    config.max_temp_limit,
                )
            }),
            minutes,
            0.15,
            action_factor(context, HestiaAction::Misting),
        );

        normalize_weights(&mut [&mut ec, &mut ph, &mut water_level, &mut temp]);

        let score = 100.0
            * (ec.comfort * ec.weight
                + ph.comfort * ph.weight
                + water_level.comfort * water_level.weight
                + temp.comfort * temp.weight);
        let mut reasons = reasons_for(current, config, &ec, &ph, &water_level, &temp, context);
        let state = if is_recovery_context(context) {
            reasons.push("recent_intervention_recovery".to_string());
            HestiaState::Recovery
        } else if score >= 80.0 {
            HestiaState::Comfortable
        } else if score >= 60.0 {
            HestiaState::Warning
        } else {
            HestiaState::Critical
        };

        HestiaAssessment {
            score: round2(score),
            state,
            confidence: confidence(context),
            axes: HestiaAxesAssessment {
                ec,
                ph,
                water_level,
                temp,
            },
            reasons,
        }
    }
}

fn axis(
    comfort: f32,
    previous_comfort: Option<f32>,
    minutes_since_previous: f32,
    base_weight: f32,
    action_factor: f32,
) -> HestiaAxisAssessment {
    let (trend, trend_factor) = match previous_comfort {
        Some(previous) => {
            let projected_30_min_delta = (comfort - previous) * (30.0 / minutes_since_previous);
            if (-0.02..=0.02).contains(&projected_30_min_delta) {
                (HestiaTrendDirection::Stable, 1.0)
            } else if projected_30_min_delta > 0.02 {
                (HestiaTrendDirection::Improving, 0.95)
            } else if projected_30_min_delta >= -0.05 {
                (HestiaTrendDirection::Degrading, 1.1)
            } else if projected_30_min_delta >= -0.10 {
                (HestiaTrendDirection::Degrading, 1.3)
            } else {
                (HestiaTrendDirection::Degrading, 1.6)
            }
        }
        None => (HestiaTrendDirection::Stable, 1.0),
    };

    HestiaAxisAssessment {
        comfort,
        weight: base_weight * trend_factor * action_factor,
        trend,
        trend_factor,
        action_factor,
    }
}

fn target_comfort(value: f32, target: f32, tolerance: f32, min: f32, max: f32) -> f32 {
    trapezoid_comfort(value, min, target - tolerance, target + tolerance, max)
}

fn trapezoid_comfort(value: f32, min: f32, ideal_min: f32, ideal_max: f32, max: f32) -> f32 {
    if value <= min || value >= max {
        return 0.0;
    }
    if value >= ideal_min && value <= ideal_max {
        return 1.0;
    }
    if value < ideal_min {
        return ((value - min) / (ideal_min - min).max(0.0001)).clamp(0.0, 1.0);
    }
    ((max - value) / (max - ideal_max).max(0.0001)).clamp(0.0, 1.0)
}

fn normalize_weights(axes: &mut [&mut HestiaAxisAssessment]) {
    let sum: f32 = axes.iter().map(|axis| axis.weight).sum();
    if sum <= 0.0001 {
        return;
    }
    for axis in axes.iter_mut() {
        axis.weight = round4(axis.weight / sum);
    }
}

fn action_factor(context: &HestiaContext, action: HestiaAction) -> f32 {
    if context.last_action != action {
        return 1.0;
    }
    elapsed_action_factor(context.minutes_since_last_intervention)
}

fn action_factor_for_any(context: &HestiaContext, actions: &[HestiaAction]) -> f32 {
    if !actions.contains(&context.last_action) {
        return 1.0;
    }
    elapsed_action_factor(context.minutes_since_last_intervention)
}

fn elapsed_action_factor(minutes: Option<f32>) -> f32 {
    match minutes {
        Some(value) if value < 30.0 => 0.5,
        Some(value) if value < 60.0 => 0.7,
        _ => 1.0,
    }
}

fn is_recovery_context(context: &HestiaContext) -> bool {
    let recent_action = context.last_action != HestiaAction::None
        && context
            .minutes_since_last_intervention
            .map(|minutes| minutes < 30.0)
            .unwrap_or(false);
    let recovery_phase = context
        .phase
        .as_deref()
        .map(|phase| {
            matches!(
                phase,
                "ActiveMixing"
                    | "Stabilizing"
                    | "Cooldown"
                    | "MimoDosing"
                    | "WaterRefilling"
                    | "WaterDraining"
            )
        })
        .unwrap_or(false);

    recent_action || recovery_phase
}

fn confidence(context: &HestiaContext) -> f32 {
    let mut value = 0.60;
    if context.previous.is_some() {
        value += 0.10;
    }
    if context.matrix_is_warm {
        value += 0.15;
    }
    if let Some(kalman) = context.mean_kalman_confidence {
        value += kalman.clamp(0.0, 1.0) * 0.15;
    }
    round2(value.clamp(0.0, 1.0))
}

fn reasons_for(
    current: &SensorData,
    config: &ControllerConfig,
    ec: &HestiaAxisAssessment,
    ph: &HestiaAxisAssessment,
    water_level: &HestiaAxisAssessment,
    temp: &HestiaAxisAssessment,
    context: &HestiaContext,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if config.enable_ec_sensor && ec.comfort < 0.6 {
        reasons.push("ec_out_of_range".to_string());
    }
    if config.enable_ph_sensor && ph.comfort < 0.6 {
        reasons.push("ph_out_of_range".to_string());
    }
    if config.enable_water_level_sensor && current.water_level <= config.water_level_critical_min {
        reasons.push("water_level_critical".to_string());
    } else if config.enable_water_level_sensor && water_level.comfort < 0.6 {
        reasons.push("water_level_out_of_range".to_string());
    }
    if config.enable_temp_sensor && temp.comfort < 0.6 {
        reasons.push("temp_out_of_range".to_string());
    }

    if ec.trend == HestiaTrendDirection::Degrading {
        reasons.push("ec_degrading".to_string());
    }
    if ph.trend == HestiaTrendDirection::Degrading {
        reasons.push("ph_degrading".to_string());
    }
    if water_level.trend == HestiaTrendDirection::Degrading {
        reasons.push("water_level_degrading".to_string());
    }
    if temp.trend == HestiaTrendDirection::Degrading {
        reasons.push("temp_degrading".to_string());
    }

    match context.last_action {
        HestiaAction::EcDosing if ec.action_factor < 1.0 => {
            reasons.push("recent_ec_dosing".to_string())
        }
        HestiaAction::PhDosing if ph.action_factor < 1.0 => {
            reasons.push("recent_ph_dosing".to_string())
        }
        HestiaAction::WaterRefill if water_level.action_factor < 1.0 => {
            reasons.push("recent_water_refill".to_string())
        }
        HestiaAction::WaterDrain if water_level.action_factor < 1.0 => {
            reasons.push("recent_water_drain".to_string())
        }
        HestiaAction::Misting if temp.action_factor < 1.0 => {
            reasons.push("recent_misting".to_string())
        }
        _ => {}
    }

    reasons
}

fn round2(value: f32) -> f32 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f32) -> f32 {
    (value * 10000.0).round() / 10000.0
}
