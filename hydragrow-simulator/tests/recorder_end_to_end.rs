use anyhow::Result;
use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;
use std::fs;
use std::path::PathBuf;

/// `Recorder` was previously only unit-tested in isolation
/// (`telemetry/recorder.rs::test_record_csv_line`) — never through the actual
/// `HarnessBuilder::record` -> `Harness::tick` -> `Recorder::record` wiring that the
/// CLI's `--record` flag depends on. This proves that wiring end-to-end.
#[test]
fn harness_records_real_tick_rows_to_csv() -> Result<()> {
    let path = PathBuf::from(std::env::temp_dir()).join("hydragrow_sim_recorder_e2e.csv");
    let _ = fs::remove_file(&path);

    let config = ControllerConfig::default();
    let tank = Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    let mut harness = Harness::builder(config, tank)
        .noise(NoiseConfig::none())
        .record(Some(path.clone()))
        .build()?;

    for _ in 0..3 {
        harness.tick(1000)?;
    }
    drop(harness);

    let content = fs::read_to_string(&path)?;
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines[0], "time,phase,ec,ph,temp,level,pump_a,pump_b");
    assert_eq!(lines.len(), 4, "header plus 3 recorded ticks");
    assert!(lines[1].starts_with("1000,"), "line was: {}", lines[1]);
    assert!(lines[3].starts_with("3000,"), "line was: {}", lines[3]);

    fs::remove_file(&path)?;
    Ok(())
}
