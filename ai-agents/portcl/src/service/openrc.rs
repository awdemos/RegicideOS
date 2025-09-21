use super::{ServiceManager, ServiceStatus};
use crate::config::PortageConfig;
use crate::error::{PortCLError, Result};
use std::path::Path;
use std::process::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;

pub struct OpenRCServiceManager {
    service_files: Vec<&'static str>,
}

impl OpenRCServiceManager {
    pub fn new() -> Self {
        Self {
            service_files: vec![
                "portcl-agent",
                "portcl-monitor",
            ],
        }
    }

    pub fn is_available() -> bool {
        Path::new("/etc/init.d").exists()
            && Command::new("rc-status")
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
    }

    fn run_rc_service(&self, service_name: &str, action: &str) -> Result<String> {
        let output = Command::new("rc-service")
            .args(&[service_name, action])
            .output()
            .map_err(|e| PortCLError::Service(format!("Failed to run rc-service: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PortCLError::Service(format!("rc-service failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_service_status(&self, service_name: &str) -> Result<ServiceStatus> {
        let output = Command::new("rc-status")
            .arg("service")
            .arg(service_name)
            .output()
            .map_err(|e| PortCLError::Service(format!("Failed to check service status: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("started") {
            Ok(ServiceStatus::Running)
        } else if output_str.contains("stopped") {
            Ok(ServiceStatus::Stopped)
        } else if output_str.contains("crashed") {
            Ok(ServiceStatus::Failed)
        } else {
            Ok(ServiceStatus::NotInstalled)
        }
    }

    fn install_service_file(&self, service_name: &str, config: &PortageConfig) -> Result<()> {
        let source_path = format!("init.d/{}", service_name);
        let target_path = format!("/etc/init.d/{}", service_name);

        // Read service file template
        let service_content = fs::read_to_string(&source_path)
            .map_err(|e| PortCLError::Io(e))?;

        // Replace placeholders if needed
        let final_content = service_content;

        // Write to init.d directory
        fs::write(&target_path, final_content)
            .map_err(|e| PortCLError::Io(e))?;

        // Make executable
        let mut perms = fs::metadata(&target_path)
            .map_err(|e| PortCLError::Io(e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms)
            .map_err(|e| PortCLError::Io(e))?;

        println!("Installed service file: {}", target_path);
        Ok(())
    }

    fn create_config_directory(&self, config: &PortageConfig) -> Result<()> {
        let config_dir = Path::new("/etc/portcl");
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

impl ServiceManager for OpenRCServiceManager {
    fn install_service(&self, config: &PortageConfig) -> Result<()> {
        println!("Installing PortCL OpenRC services...");

        // Create necessary directories
        self.create_config_directory(config)?;

        // Install service files
        for service_name in &self.service_files {
            self.install_service_file(service_name, config)?;
        }

        // Add services to default runlevel
        for service_name in &self.service_files {
            let output = Command::new("rc-update")
                .args(&["add", service_name, "default"])
                .output()
                .map_err(|e| PortCLError::Service(format!("Failed to add service to runlevel: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(PortCLError::Service(format!("rc-update failed: {}", stderr)));
            }
            println!("Added service to default runlevel: {}", service_name);
        }

        println!("PortCL OpenRC services installed successfully");
        Ok(())
    }

    fn uninstall_service(&self) -> Result<()> {
        println!("Uninstalling PortCL OpenRC services...");

        // Remove services from runlevel
        for service_name in &self.service_files {
            // Remove from runlevel
            let _ = Command::new("rc-update")
                .args(&["del", service_name, "default"])
                .output();

            // Stop service if running
            let _ = self.run_rc_service(service_name, "stop");

            // Remove service file
            let service_path = format!("/etc/init.d/{}", service_name);
            if Path::new(&service_path).exists() {
                fs::remove_file(&service_path)
                    .map_err(|e| PortCLError::Io(e))?;
                println!("Removed service file: {}", service_path);
            }
        }

        println!("PortCL OpenRC services uninstalled successfully");
        Ok(())
    }

    fn start_service(&self) -> Result<()> {
        println!("Starting PortCL services...");

        for service_name in &self.service_files {
            self.run_rc_service(service_name, "start")?;
            println!("Started service: {}", service_name);
        }

        Ok(())
    }

    fn stop_service(&self) -> Result<()> {
        println!("Stopping PortCL services...");

        for service_name in &self.service_files {
            // Don't fail if service is not running
            let _ = self.run_rc_service(service_name, "stop");
            println!("Stopped service: {}", service_name);
        }

        Ok(())
    }

    fn restart_service(&self) -> Result<()> {
        println!("Restarting PortCL services...");

        for service_name in &self.service_files {
            self.run_rc_service(service_name, "restart")?;
            println!("Restarted service: {}", service_name);
        }

        Ok(())
    }

    fn service_status(&self) -> Result<ServiceStatus> {
        let mut overall_status = ServiceStatus::NotInstalled;

        for service_name in &self.service_files {
            let status = self.get_service_status(service_name)?;

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