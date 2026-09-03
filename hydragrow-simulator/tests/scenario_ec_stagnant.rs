use anyhow::Result;
use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::harness::Harness;

#[test]
fn test_ec_stagnant_scenario() -> Result<()> {
    let config = ControllerConfig {
        ec_target: 1.5,
        max_ec_delta: 0.5,
        dosing_min_pwm_percent: 50,
        ..Default::default()
    };

    let mut harness = Harness::from_scenario(config, "src/scenario/library/ec_stagnant.json")?;

    for _ in 0..15 {
        harness.tick(1000)?;
    }

    assert_eq!(harness.uptime_ms(), 15_000);
    assert!(harness.hw.pump_a.on);
    assert_ne!(
        harness.ctx.phase,
        hydragrow_shared::fsm::SystemPhase::Booting
    );

    Ok(())
}
