mod cli;
mod run;

use clap::Parser;
use cli::Cli;
use hydragrow_supervisor_query::{HttpQueryBackend, SupervisorQuery};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let backend = HttpQueryBackend::new(cli.backend_url.clone(), cli.api_key.clone());
    let query: SupervisorQuery = cli.command.into();

    match run::run(query, &backend, cli.compact).await {
        Ok(output) => {
            println!("{output}");
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
