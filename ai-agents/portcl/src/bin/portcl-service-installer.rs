use portcl::config::PortageConfig;
use portcl::error::{PortCLError, Result};
use portcl::service::{get_service_manager, ServiceStatus};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "portcl-service-installer")]
#[command(about = "Install and manage PortCL system services")]
#[command(version = "1.0.0")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/portcl/config.toml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install PortCL services
    Install,
    /// Uninstall PortCL services
    Uninstall,
    /// Start PortCL services
    Start,
    /// Stop PortCL services
    Stop,
    /// Restart PortCL services
    Restart,
    /// Show service status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli.config)?;

    let service_manager = get_service_manager();

    match cli.command.unwrap_or(Commands::Status) {
        Commands::Install => {
            println!("Installing PortCL services...");
            service_manager.install_service(&config)?;
            println!("✓ PortCL services installed successfully");
        }
        Commands::Uninstall => {
            println!("Uninstalling PortCL services...");
            service_manager.uninstall_service()?;
            println!("✓ PortCL services uninstalled successfully");
        }
        Commands::Start => {
            println!("Starting PortCL services...");
            service_manager.start_service()?;
            println!("✓ PortCL services started successfully");
        }
        Commands::Stop => {
            println!("Stopping PortCL services...");
            service_manager.stop_service()?;
            println!("✓ PortCL services stopped successfully");
        }
        Commands::Restart => {
            println!("Restarting PortCL services...");
            service_manager.restart_service()?;
            println!("✓ PortCL services restarted successfully");
        }
        Commands::Status => {
            let status = service_manager.service_status()?;
            match status {
                ServiceStatus::Running => println!("✓ PortCL services are running"),
                ServiceStatus::Stopped => println!("⏸ PortCL services are stopped"),
                ServiceStatus::Failed => println!("✗ PortCL services have failed"),
                ServiceStatus::NotInstalled => println!("⚠ PortCL services are not installed"),
                ServiceStatus::Unknown => println!("? PortCL service status is unknown"),
            }
        }
    }

    Ok(())
}

fn load_config(path: &str) -> Result<PortageConfig> {
    if !Path::new(path).exists() {
        // Create default config if it doesn't exist
        let default_config = PortageConfig::default();
        let config_content = toml::to_string_pretty(&default_config)
            .map_err(|e| PortCLError::Configuration(format!("Failed to serialize default config: {}", e)))?;

        std::fs::write(path, config_content)
            .map_err(|e| PortCLError::Io(e))?;

        println!("Created default configuration at: {}", path);
        Ok(default_config)
    } else {
        let config_content = std::fs::read_to_string(path)
            .map_err(|e| PortCLError::Io(e))?;

        let config: PortageConfig = toml::from_str(&config_content)
            .map_err(|e| PortCLError::Configuration(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }
}