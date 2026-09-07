use anyhow::Result;
use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::{ControlMode, ControllerConfig};
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;

fn test_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        cooldown_sec: 1,
        pump_a_capacity_ml_per_sec: 2.0,
        pump_b_capacity_ml_per_sec: 2.0,
        ec_gain_per_ml: 0.3,
        dosing_min_pwm_percent: 30,
        ..Default::default()
    }
}

fn test_tank() -> Tank {
    Tank {
        volume_l: 10.0,
        ec: 0.3,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    }
}

/// Proves a real causal chain: a frozen EC sensor pinned below target makes every
/// dosing cycle observe `actual_delta_ec ≈ 0` despite real nutrient delivery, so the
/// residual diagnostic in `stabilizing.rs` (`diagnose_hardware_fault`, streak >= 3)
/// trips the real, already-implemented `FaultCode::EcDosingFailed`.
///
/// The control run (same config, no fault injected) must NOT reach that fault in the
/// same tick budget — the tank genuinely converges toward target — proving the frozen
/// sensor is load-bearing and this is not asserting a tautology.
#[test]
fn frozen_ec_sensor_drives_repeated_dosing_into_hardware_fault() -> Result<()> {
    const TICKS: usize = 600;

    let mut harness = Harness::from_scenario(
        test_config(),
        "src/scenario/library/ec_sensor_frozen_budget_exceeded.json",
    )?;

    let mut reached_dosing_fault = false;
    for _ in 0..TICKS {
        harness.tick(1000)?;
        if harness.ctx.phase == SystemPhase::Fault(FaultCode::EcDosingFailed) {
            reached_dosing_fault = true;
            break;
        }
    }

    assert!(
        reached_dosing_fault,
        "a frozen EC sensor stuck below target must eventually trip EcDosingFailed \
         via the residual diagnostic; final phase was {:?}, tank ec={}",
        harness.ctx.phase,
        harness.tank.ec
    );

    // Control: identical plant and config, no injected fault — the tank converges
    // and the same fault must not appear within the same budget.
    let mut control =
        Harness::builder(test_config(), test_tank()).noise(NoiseConfig::none()).build()?;
    for _ in 0..TICKS {
        control.tick(1000)?;
        assert_ne!(
            control.ctx.phase,
            SystemPhase::Fault(FaultCode::EcDosingFailed),
            "without the frozen sensor the tank converges, so EcDosingFailed must not trip"
        );
    }

    Ok(())
}
