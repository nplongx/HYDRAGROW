use hydragrow_shared::ControllerConfig;

/// Calculates the change in EC for a given pump flow and tank volume
/// First-order linear model as specified in Phase 2.
pub fn calculate_ec_change(pump_flow_ml: f32, volume_l: f32, config: &ControllerConfig) -> f32 {
    if volume_l <= 0.0 {
        return 0.0;
    }
    (pump_flow_ml * config.ec_gain_per_ml) / volume_l
}

/// Calculates the change in pH for a given pump flow, tank volume, and direction (up/down)
pub fn calculate_ph_change(pump_flow_ml: f32, volume_l: f32, is_up: bool, config: &ControllerConfig) -> f32 {
    if volume_l <= 0.0 {
        return 0.0;
    }
    let shift = if is_up { config.ph_shift_up_per_ml } else { -config.ph_shift_down_per_ml };
    (pump_flow_ml * shift) / volume_l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_ec_change() {
        let config = ControllerConfig {
            ec_gain_per_ml: 0.1,
            ..Default::default()
        };
        let change = calculate_ec_change(5.0, 10.0, &config);
        // pump flow (5.0) * ec_gain_per_ml (0.1) / volume (10.0)
        assert_eq!(change, 0.05);
    }
}
