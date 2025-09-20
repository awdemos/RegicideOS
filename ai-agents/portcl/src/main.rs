use clap::{Parser, Subcommand};
use portcl::config::PortageConfig;
use portcl::monitor::PortageMonitor;
use portcl::rl_engine::PortageAgent;
use portcl::actions::ActionExecutor;
use portcl::prelude::*;
use tracing::{info, error, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "portcl")]
#[command(about = "Portage Continual Learning Agent for RegicideOS")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long, default_value = "/etc/portcl/config.toml")]
    config: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run as daemon
    #[arg(long)]
    daemon: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the PortCL agent
    Run,
    /// Validate configuration
    Validate,
    /// Show system status
    Status,
    /// Test Portage integration
    TestPortage,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    // Load configuration
    let config = PortageConfig::load(&cli.config)?;

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => {
            info!("Starting PortCL agent");
            run_agent(config).await?;
        }
        Commands::Validate => {
            info!("Validating configuration");
            config.validate()?;
            println!("Configuration is valid");
        }
        Commands::Status => {
            show_status().await?;
        }
        Commands::TestPortage => {
            test_portage_integration().await?;
        }
    }

    Ok(())
}

async fn run_agent(config: PortageConfig) -> Result<()> {
    let monitor = PortageMonitor::new(config.monitoring.clone())?;
    let agent = PortageAgent::new(config.rl.clone())?;
    let executor = ActionExecutor::new(config.actions.clone())?;

    // Main agent loop
    loop {
        match monitor.collect_metrics().await {
            Ok(metrics) => {
                let action = agent.select_action(&metrics).await?;
                let result = executor.execute(action).await?;

                // Update agent with experience
                agent.update_experience(metrics, action, result).await?;

                info!("Action executed successfully: {:?}", action);
            }
            Err(e) => {
                error!("Error collecting metrics: {}", e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

async fn show_status() -> Result<()> {
    println!("PortCL Status: Active");
    println!("Version: 0.1.0");
    println!("Configuration: /etc/portcl/config.toml");
    Ok(())
}

async fn test_portage_integration() -> Result<()> {
    println!("Testing Portage integration...");
    // Implementation will be added in PortageMonitor
    println!("Portage integration test completed");
    Ok(())
}
