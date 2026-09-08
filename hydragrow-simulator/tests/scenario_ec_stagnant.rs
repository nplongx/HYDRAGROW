use anyhow::Result;
use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::harness::Harness;

/// `PumpStuckOn(PUMP_A)` forces the virtual pump physically on every tick regardless
/// of what the FSM commands (see `Injector::apply_hardware_faults`, which reapplies
/// the fault unconditionally each tick). The observable, provable consequence is not
/// "the pump bit is true" (the injector guarantees that trivially) — it is that the
/// tank keeps receiving nutrient flow and EC keeps drifting upward even after the FSM
/// believes dosing has finished and moved on, because the FSM has no hardware
/// read-back to detect the divergence.
#[test]
fn test_ec_stagnant_scenario() -> Result<()> {
    let config = ControllerConfig {
        ec_target: 1.5,
        ec_tolerance: 0.05,
        max_ec_delta: 0.5,
        dosing_min_pwm_percent: 50,
        cooldown_sec: 1,
        ..Default::default()
    };

    let mut harness = Harness::from_scenario(config, "src/scenario/library/ec_stagnant.json")?;
    let initial_ec = harness.tank.ec;

    for _ in 0..15 {
        harness.tick(1000)?;
    }

    assert_eq!(harness.uptime_ms(), 15_000);
    assert!(
        harness.hw.pump_a.on,
        "PumpStuckOn fault must keep pump A physically on regardless of FSM commands"
    );
    assert!(
        harness.tank.ec > initial_ec,
        "a pump stuck on must keep injecting nutrient into the tank even after the FSM \
         believes dosing finished, so EC must keep drifting upward: initial={} final={}",
        initial_ec,
        harness.tank.ec
    );
    assert_ne!(
        harness.ctx.phase,
        hydragrow_shared::fsm::SystemPhase::Booting
    );

    Ok(())
}
