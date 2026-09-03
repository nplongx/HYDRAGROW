use crate::harness::Harness;
use crate::plant::tank::Tank;
use crate::scenario::format::load_scenario;
use anyhow::{Context, Result};
use hydragrow_shared::ControllerConfig;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub scenario: Option<PathBuf>,
    pub ticks: u64,
    pub tick_ms: u64,
    pub device_id: String,
    pub mqtt: Option<String>,
    pub record: Option<PathBuf>,
}

pub fn build_harness(config: ControllerConfig, options: &RunOptions) -> Result<Harness> {
    let scenario = options
        .scenario
        .as_ref()
        .map(|path| load_scenario(path))
        .transpose()?;

    let tank = scenario
        .as_ref()
        .map(|s| Tank::from_initial(&s.initial_tank))
        .unwrap_or_default();

    let mut builder = Harness::builder(config, tank)
        .device_id(options.device_id.clone())
        .mqtt(options.mqtt.clone())
        .record(options.record.clone());

    if let Some(sc) = scenario {
        builder = builder.scenario(sc);
    }

    builder.build()
}

pub fn run_simulation(config: ControllerConfig, options: &RunOptions) -> Result<()> {
    let mut harness = build_harness(config, options)?;
    println!(
        "Starting simulation: device_id={} ticks={} tick_ms={}",
        options.device_id, options.ticks, options.tick_ms
    );

    for tick_idx in 1..=options.ticks {
        let result = harness.tick(options.tick_ms)?;
        println!(
            "Tick {}/{}: uptime={}ms phase={:?} events={}",
            tick_idx,
            options.ticks,
            harness.uptime_ms(),
            harness.ctx.phase,
            result.events.len()
        );
    }

    println!(
        "Simulation complete. Final state: uptime={}ms phase={:?} EC={:.2} pH={:.2}",
        harness.uptime_ms(),
        harness.ctx.phase,
        harness.tank.ec,
        harness.tank.ph
    );

    Ok(())
}

pub fn list_scenarios(scenarios_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(scenarios_dir).with_context(|| {
        format!(
            "failed to read scenario directory: {}",
            scenarios_dir.display()
        )
    })?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn run_interactive_step(mut harness: Harness) -> Result<()> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    println!("Interactive step REPL. Enter tick duration in ms (e.g. 1000), or 'q' to quit.");
    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF
        }
        match line.trim() {
            "" => continue,
            "q" | "quit" => break,
            value => {
                let dt_ms = match value.parse::<u64>() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("enter milliseconds, or q to quit");
                        continue;
                    }
                };
                let result = harness.tick(dt_ms)?;
                println!(
                    "uptime={} phase={:?} events={}",
                    harness.uptime_ms(),
                    harness.ctx.phase,
                    result.events.len()
                );
            }
        }
    }
    Ok(())
}
