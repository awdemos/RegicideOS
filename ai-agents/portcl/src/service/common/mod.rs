use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use crate::error::{PortCLError, Result};

pub fn is_systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}

pub fn is_openrc_available() -> bool {
    Path::new("/run/openrc").exists()
}

pub fn create_service_directory() -> Result<()> {
    fs::create_dir_all("/etc/portcl")?;
    let mut perms = fs::metadata("/etc/portcl")?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions("/etc/portcl", perms)?;
    Ok(())
}

pub fn create_service_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        let mut perms = fs::metadata(parent)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(parent, perms)?;
    }

    fs::write(path, content)?;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

pub fn run_systemctl(args: &[&str]) -> Result<()> {
    Command::new("systemctl")
        .args(args)
        .output()
        .map_err(PortCLError::from)?;
    Ok(())
}

pub fn run_rc_service(args: &[&str]) -> Result<()> {
    Command::new("rc-service")
        .args(args)
        .output()
        .map_err(PortCLError::from)?;
    Ok(())
}

pub fn is_service_running(service_name: &str) -> bool {
    Command::new("pidof")
        .arg(service_name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn check_service_process(service_name: &str) -> bool {
    is_service_running(service_name)
}
