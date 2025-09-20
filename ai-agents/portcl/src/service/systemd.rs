use super::{ServiceManager, ServiceStatus};
use crate::config::PortCLConfig;
use crate::error::{PortCLError, Result};
use std::path::Path;
use std::process::Command;
use std::fs;
use serde::{Deserialize, Serialize};

pub struct SystemdServiceManager {
    service_files: Vec<&'static str>,
}

impl SystemdServiceManager {
    pub fn new() -> Self {
        Self {
            service_files: vec![
                "portcl-agent.service",
                "portcl-monitor.service",
            ],
        }
    }

    pub fn is_available() -> bool {
        Path::new("/run/systemd/system").exists()
            || Command::new("systemctl")
                .arg("--version")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn run_systemctl(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("systemctl")
            .args(args)
            .output()
            .map_err(|e| PortCLError::Service(format!("Failed to run systemctl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PortCLError::Service(format!("systemctl failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_service_status_from_systemd(&self, service_name: &str) -> Result<ServiceStatus> {
        let output = Command::new("systemctl")
            .args(&["is-active", service_name])
            .output()
            .map_err(|e| PortCLError::Service(format!("Failed to check service status: {}", e)))?;

        let status_code = output.status.code().unwrap_or(-1);
        match status_code {
            0 => Ok(ServiceStatus::Running),
            3 => Ok(ServiceStatus::Stopped),
            _ => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("not loaded") || stderr.contains("not found") {
                    Ok(ServiceStatus::NotInstalled)
                } else {
                    Ok(ServiceStatus::Failed)
                }
            }
        }
    }

    fn install_service_file(&self, service_name: &str, config: &PortCLConfig) -> Result<()> {
        let source_path = format!("systemd/{}", service_name);
        let target_path = format!("/etc/systemd/system/{}", service_name);

        // Read service file template
        let service_content = fs::read_to_string(&source_path)
            .map_err(|e| PortCLError::Io(e))?;

        // Replace placeholders if needed
        let final_content = service_content
            .replace("/usr/local/bin/", &config.general.binary_path)
            .replace("/etc/portcl/config.toml", &config.config_path);

        // Write to systemd directory
        fs::write(&target_path, final_content)
            .map_err(|e| PortCLError::Io(e))?;

        println!("Installed service file: {}", target_path);
        Ok(())
    }

    fn create_config_directory(&self, config: &PortCLConfig) -> Result<()> {
        let config_dir = Path::new(&config.config_path).parent().unwrap();
        fs::create_dir_all(config_dir)
            .map_err(|e| PortCLError::Io(e))?;

        let log_dir = "/var/log/portcl";
        fs::create_dir_all(log_dir)
            .map_err(|e| PortCLError::Io(e))?;

        let lib_dir = "/var/lib/portcl";
        fs::create_dir_all(lib_dir)
            .map_err(|e| PortCLError::Io(e))?;

        Ok(())
    }
}

impl ServiceManager for SystemdServiceManager {
    fn install_service(&self, config: &PortCLConfig) -> Result<()> {
        println!("Installing PortCL systemd services...");

        // Create necessary directories
        self.create_config_directory(config)?;

        // Install service files
        for service_name in &self.service_files {
            self.install_service_file(service_name, config)?;
        }

        // Reload systemd
        self.run_systemctl(&["daemon-reload"])?;

        // Enable services
        for service_name in &self.service_files {
            self.run_systemctl(&["enable", service_name])?;
            println!("Enabled service: {}", service_name);
        }

        println!("PortCL systemd services installed successfully");
        Ok(())
    }

    fn uninstall_service(&self) -> Result<()> {
        println!("Uninstalling PortCL systemd services...");

        // Stop and disable services
        for service_name in &self.service_files {
            // Stop service if running
            let _ = self.run_systemctl(&["stop", service_name]);

            // Disable service
            let _ = self.run_systemctl(&["disable", service_name]);

            // Remove service file
            let service_path = format!("/etc/systemd/system/{}", service_name);
            if Path::new(&service_path).exists() {
                fs::remove_file(&service_path)
                    .map_err(|e| PortCLError::Io(e))?;
                println!("Removed service file: {}", service_path);
            }
        }

        // Reload systemd
        self.run_systemctl(&["daemon-reload"])?;

        println!("PortCL systemd services uninstalled successfully");
        Ok(())
    }

    fn start_service(&self) -> Result<()> {
        println!("Starting PortCL services...");

        for service_name in &self.service_files {
            self.run_systemctl(&["start", service_name])?;
            println!("Started service: {}", service_name);
        }

        Ok(())
    }

    fn stop_service(&self) -> Result<()> {
        println!("Stopping PortCL services...");

        for service_name in &self.service_files {
            // Don't fail if service is not running
            let _ = self.run_systemctl(&["stop", service_name]);
            println!("Stopped service: {}", service_name);
        }

        Ok(())
    }

    fn restart_service(&self) -> Result<()> {
        println!("Restarting PortCL services...");

        for service_name in &self.service_files {
            self.run_systemctl(&["restart", service_name])?;
            println!("Restarted service: {}", service_name);
        }

        Ok(())
    }

    fn service_status(&self) -> Result<ServiceStatus> {
        let mut overall_status = ServiceStatus::NotInstalled;

        for service_name in &self.service_files {
            let status = self.get_service_status_from_systemd(service_name)?;

            match status {
                ServiceStatus::Failed => return Ok(ServiceStatus::Failed),
                ServiceStatus::Running => overall_status = ServiceStatus::Running,
                ServiceStatus::Stopped if overall_status == ServiceStatus::NotInstalled => {
                    overall_status = ServiceStatus::Stopped;
                }
                _ => {}
            }
        }

        Ok(overall_status)
    }

    fn is_available(&self) -> bool {
        Self::is_available()
    }
}