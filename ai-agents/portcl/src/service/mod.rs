pub mod systemd;
pub mod openrc;

use crate::config::PortageConfig;
use crate::error::{PortCLError, Result};
use std::path::Path;

pub trait ServiceManager {
    fn install_service(&self, config: &PortageConfig) -> Result<()>;
    fn uninstall_service(&self) -> Result<()>;
    fn start_service(&self) -> Result<()>;
    fn stop_service(&self) -> Result<()>;
    fn restart_service(&self) -> Result<()>;
    fn service_status(&self) -> Result<ServiceStatus>;
    fn is_available(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Failed,
    NotInstalled,
    Unknown,
}

pub fn get_service_manager() -> Box<dyn ServiceManager> {
    if systemd::SystemdServiceManager::is_available() {
        Box::new(systemd::SystemdServiceManager::new())
    } else if openrc::OpenRCServiceManager::is_available() {
        Box::new(openrc::OpenRCServiceManager::new())
    } else {
        panic!("No supported service manager found");
    }
}