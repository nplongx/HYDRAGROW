# Simulator Phase 5 - CLI Ergonomics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a CLI interface to run the simulation interactively or step-by-step for FSM debugging.

**Architecture:** Use `clap` to define subcommands (`run`, `step`, `scenario-list`). For step mode, provide a simple interactive REPL loop reading stdin and stepping the harness, preserving state between commands.

**Tech Stack:** Rust, `clap`.

---

### Task 1: CLI Args parsing

**Files:**
- Modify: `hydragrow-simulator/src/main.rs`

- [ ] **Step 1: Define `clap` Parser**

```rust
// hydragrow-simulator/src/main.rs
use clap::{Parser, Subcommand};

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
```

- [ ] **Step 2: Run test (cargo check)**

Run: `cd hydragrow-simulator && cargo check`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add hydragrow-simulator/src/main.rs
git commit -m "feat(simulator): add CLI arguments parser using clap"
```

### Task 2: Implement the Interactive Step REPL

**Files:**
- Modify: `hydragrow-simulator/src/main.rs`

- [ ] **Step 1: Add simple REPL loop**

```rust
// Add inside main.rs
use std::io::{self, Write};
use hydragrow_simulator::harness::Harness;
// (Initialize Harness with defaults or from scenario if provided)

fn run_interactive_step(mut harness: Harness) {
    println!("Interactive step mode. Type duration in ms (e.g., '100') and press Enter. 'q' to quit.");
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "q" || input == "quit" {
            break;
        }

        if let Ok(dt) = input.parse::<u64>() {
            let result = harness.tick(dt);
            println!("Tick +{}ms. Uptime: {}", dt, harness.uptime_ms());
            println!("Events emitted: {:?}", result.events);
            println!("Context Delta: {:?}", result.delta);
        } else {
            println!("Invalid input. Please enter ms as number.");
        }
    }
}
```

- [ ] **Step 2: Wire commands in main()**

```rust
fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Step { scenario } => {
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
```

- [ ] **Step 3: Test interactively**

Run: `cd hydragrow-simulator && cargo run -- step` (Verify it prompts and exits on 'q').

- [ ] **Step 4: Commit**

```bash
git add hydragrow-simulator/src/main.rs
git commit -m "feat(simulator): implement interactive step REPL for FSM debugging"
```
