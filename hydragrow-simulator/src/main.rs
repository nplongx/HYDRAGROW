use clap::{Parser, Subcommand};
// use std::io::{self, Write};
// use hydragrow_simulator::harness::Harness;

#[derive(Parser)]
#[command(name = "hydragrow-sim")]
#[command(about = "HydraGrow Simulator Digital Twin", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a scenario continuously
    Run {
        #[arg(short, long)]
        scenario: Option<String>,
        #[arg(short, long)]
        mqtt: Option<String>,
        #[arg(short, long)]
        device_id: Option<String>,
        #[arg(short, long)]
        record: Option<String>,
    },
    /// Starts an interactive step-by-step REPL
    Step {
        #[arg(short, long)]
        scenario: Option<String>,
    },
    /// Lists available scenarios
    ScenarioList,
}

// fn run_interactive_step(mut harness: Harness) {
//     println!("Interactive step mode. Type duration in ms (e.g., '100') and press Enter. 'q' to quit.");
//     loop {
//         print!("> ");
//         io::stdout().flush().unwrap();
//
//         let mut input = String::new();
//         io::stdin().read_line(&mut input).unwrap();
//         let input = input.trim();
//
//         if input == "q" || input == "quit" {
//             break;
//         }
//
//         if let Ok(dt) = input.parse::<u64>() {
//             let result = harness.tick(dt);
//             println!("Tick +{}ms. Uptime: {}", dt, harness.uptime_ms());
//             println!("Events emitted: {:?}", result.events);
//             println!("Context Delta: {:?}", result.delta);
//         } else {
//             println!("Invalid input. Please enter ms as number.");
//         }
//     }
// }

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Step { scenario: _ } => {
            // Load config and initial tank...
            // run_interactive_step(harness);
            println!("Starting step repl...");
        }
        Commands::ScenarioList => {
            println!("Available scenarios: ec_stagnant.json, sensor_timeout.json, etc.");
        }
        Commands::Run { .. } => {
            println!("Continuous run not fully wired yet");
        }
    }
}
