use portcl::config::PortCLConfig;
use portcl::error::{PortCLError, Result};
use portcl::service::{get_service_manager, ServiceStatus};
use clap::{App, Arg, SubCommand};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("portcl-service-installer")
        .version("1.0.0")
        .about("Install and manage PortCL system services")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Path to configuration file")
            .default_value("/etc/portcl/config.toml"))
        .subcommand(SubCommand::with_name("install")
            .about("Install PortCL services"))
        .subcommand(SubCommand::with_name("uninstall")
            .about("Uninstall PortCL services"))
        .subcommand(SubCommand::with_name("start")
            .about("Start PortCL services"))
        .subcommand(SubCommand::with_name("stop")
            .about("Stop PortCL services"))
        .subcommand(SubCommand::with_name("restart")
            .about("Restart PortCL services"))
        .subcommand(SubCommand::with_name("status")
            .about("Show service status"))
        .get_matches();

    let config_path = matches.value_of("config").unwrap();
    let config = load_config(config_path)?;

    let service_manager = get_service_manager();

    match matches.subcommand() {
        ("install", Some(_)) => {
            println!("Installing PortCL services...");
            service_manager.install_service(&config)?;
            println!("✓ PortCL services installed successfully");
        }
        ("uninstall", Some(_)) => {
            println!("Uninstalling PortCL services...");
            service_manager.uninstall_service()?;
            println!("✓ PortCL services uninstalled successfully");
        }
        ("start", Some(_)) => {
            println!("Starting PortCL services...");
            service_manager.start_service()?;
            println!("✓ PortCL services started successfully");
        }
        ("stop", Some(_)) => {
            println!("Stopping PortCL services...");
            service_manager.stop_service()?;
            println!("✓ PortCL services stopped successfully");
        }
        ("restart", Some(_)) => {
            println!("Restarting PortCL services...");
            service_manager.restart_service()?;
            println!("✓ PortCL services restarted successfully");
        }
        ("status", Some(_)) => {
            let status = service_manager.service_status()?;
            match status {
                ServiceStatus::Running => println!("✓ PortCL services are running"),
                ServiceStatus::Stopped => println!("⏸ PortCL services are stopped"),
                ServiceStatus::Failed => println!("✗ PortCL services have failed"),
                ServiceStatus::NotInstalled => println!("⚠ PortCL services are not installed"),
                ServiceStatus::Unknown => println!("? PortCL service status is unknown"),
            }
        }
        _ => {
            println!("No subcommand specified. Use --help for usage information.");
        }
    }

    Ok(())
}

fn load_config(path: &str) -> Result<PortCLConfig> {
    if !Path::new(path).exists() {
        // Create default config if it doesn't exist
        let default_config = PortCLConfig::default();
        let config_content = toml::to_string_pretty(&default_config)
            .map_err(|e| PortCLError::Config(format!("Failed to serialize default config: {}", e)))?;

        std::fs::write(path, config_content)
            .map_err(|e| PortCLError::Io(e))?;

        println!("Created default configuration at: {}", path);
        Ok(default_config)
    } else {
        let config_content = std::fs::read_to_string(path)
            .map_err(|e| PortCLError::Io(e))?;

        let config: PortCLConfig = toml::from_str(&config_content)
            .map_err(|e| PortCLError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }
}