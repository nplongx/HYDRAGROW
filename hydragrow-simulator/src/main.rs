use anyhow::Result;
use clap::{Parser, Subcommand};
use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::scenario::format::load_scenario;
use hydragrow_simulator::simulation::{
    RunOptions, list_scenarios, run_interactive_step, run_simulation,
};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "hydragrow-sim")]
#[command(about = "HydraGrow Simulator Digital Twin", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Runs a scenario continuously or for a fixed number of ticks
    Run {
        #[arg(short, long)]
        scenario: Option<PathBuf>,
        #[arg(long, default_value_t = 100)]
        ticks: u64,
        #[arg(long, default_value_t = 1000)]
        tick_ms: u64,
        #[arg(long, default_value = "sim-dev")]
        device_id: String,
        #[arg(short, long)]
        mqtt: Option<String>,
        #[arg(short, long)]
        record: Option<PathBuf>,
    },
    /// Starts an interactive step-by-step REPL
    Step {
        #[arg(short, long)]
        scenario: Option<PathBuf>,
    },
    /// Lists available scenarios
    ScenarioList,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let default_config = ControllerConfig {
        control_mode: hydragrow_shared::ControlMode::Auto,
        ..Default::default()
    };

    match cli.command {
        Commands::Run {
            scenario,
            ticks,
            tick_ms,
            device_id,
            mqtt,
            record,
        } => {
            let options = RunOptions {
                scenario,
                ticks,
                tick_ms,
                device_id,
                mqtt,
                record,
            };
            run_simulation(default_config, &options)?;
        }
        Commands::Step { scenario } => {
            let tank = if let Some(path) = &scenario {
                let sc = load_scenario(path)?;
                Tank::from_initial(&sc.initial_tank)
            } else {
                Tank::default()
            };
            let mut builder = hydragrow_simulator::harness::Harness::builder(default_config, tank);
            if let Some(path) = scenario {
                let sc = load_scenario(&path)?;
                builder = builder.scenario(sc);
            }
            let harness = builder.build()?;
            run_interactive_step(harness)?;
        }
        Commands::ScenarioList => {
            let scenario_dir = Path::new("src/scenario/library");
            let scenarios = list_scenarios(scenario_dir)?;
            println!("Available scenarios:");
            for sc in scenarios {
                println!(" - {}", sc.file_name().unwrap().to_string_lossy());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_options() {
        let cli = Cli::try_parse_from([
            "hydragrow-sim",
            "run",
            "--scenario",
            "src/scenario/library/ec_stagnant.json",
            "--ticks",
            "100",
            "--tick-ms",
            "1000",
            "--device-id",
            "sim-01",
            "--record",
            "out.csv",
        ])
        .unwrap();

        match cli.command {
            Commands::Run {
                ticks,
                tick_ms,
                device_id,
                scenario,
                record,
                ..
            } => {
                assert_eq!(ticks, 100);
                assert_eq!(tick_ms, 1000);
                assert_eq!(device_id, "sim-01");
                assert_eq!(
                    scenario,
                    Some(PathBuf::from("src/scenario/library/ec_stagnant.json"))
                );
                assert_eq!(record, Some(PathBuf::from("out.csv")));
            }
            _ => panic!("wrong command"),
        }
    }
}
