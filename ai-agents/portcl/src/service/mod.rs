pub mod common;
pub mod openrc;
pub mod systemd;

pub use common::{
    check_service_process, create_service_directory, create_service_file, is_service_running,
    run_rc_service, run_systemctl,
};

use crate::config::PortageConfig;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Failed,
    NotInstalled,
    Unknown,
}

pub trait ServiceManager {
    fn start_service(&self) -> Result<()>;
    fn stop_service(&self) -> Result<()>;
    fn restart_service(&self) -> Result<()>;
    fn service_status(&self) -> Result<ServiceStatus>;
    fn install_service(&self, config: &PortageConfig) -> Result<()>;
    fn uninstall_service(&self) -> Result<()>;
    fn is_available(&self) -> bool;
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
