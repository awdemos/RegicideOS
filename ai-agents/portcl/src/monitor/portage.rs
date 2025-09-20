use crate::config::MonitoringConfig;
use crate::error::{PortCLError, Result};
use std::process::Command;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortageInfo {
    pub installed_packages: u32,
    pub available_updates: u32,
    pub world_packages: u32,
    pub last_sync: Option<DateTime<Utc>>,
    pub portage_version: String,
    pub profile: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub category: String,
    pub slot: Option<String>,
    pub repository: Option<String>,
    pub installed_size: Option<u64>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub use_flags: Vec<String>,
    pub dependencies: Vec<String>,
    pub build_time: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageQuery {
    pub name: String,
    pub category: Option<String>,
    pub slot: Option<String>,
    pub repository: Option<String>,
    pub version_constraint: Option<String>,
}

pub struct PortageMonitor {
    config: MonitoringConfig,
}

impl PortageMonitor {
    pub fn new(config: MonitoringConfig) -> Result<Self> {
        // Validate portage binary exists
        if !config.portage_path.exists() {
            return Err(PortCLError::Validation(format!(
                "Portage binary not found at: {}",
                config.portage_path.display()
            )));
        }

        Ok(Self { config })
    }

    pub async fn get_portage_info(&self) -> Result<PortageInfo> {
        debug!("Collecting Portage system information");

        let installed_packages = self.count_installed_packages().await?;
        let available_updates = self.count_available_updates().await?;
        let world_packages = self.count_world_packages().await?;
        let last_sync = self.get_last_sync_time().await?;
        let portage_version = self.get_portage_version().await?;
        let profile = self.get_current_profile().await?;
        let arch = self.get_system_arch().await?;

        Ok(PortageInfo {
            installed_packages,
            available_updates,
            world_packages,
            last_sync,
            portage_version,
            profile,
            arch,
        })
    }

    pub async fn get_package_info(&self, package: &str) -> Result<PackageInfo> {
        debug!("Getting package info for: {}", package);

        // Parse package name to extract category and name
        let (category, package_name) = self.parse_package_name(package)?;

        // Get package information using equery or portageq
        let output = self.run_portage_command(&[
            "equery", "list", "-o", package
        ]).await?;

        if output.is_empty() {
            return Err(PortCLError::Portage(format!(
                "Package {} not found", package
            )));
        }

        // Parse equery output to get installed version
        let installed_version = self.parse_installed_version(&output)?;

        // Get detailed package information
        let metadata = self.get_package_metadata(&category, &package_name, &installed_version).await?;

        Ok(metadata)
    }

    pub async fn search_packages(&self, query: &PackageQuery) -> Result<Vec<PackageInfo>> {
        debug!("Searching packages with query: {:?}", query);

        let mut search_cmd = vec!["emerge", "--search", "--quiet"];

        if let Some(repo) = &query.repository {
            search_cmd.extend(&["--repository", repo]);
        }

        let search_pattern = match (&query.category, &query.name) {
            (Some(cat), Some(name)) => format!("{}/{}", cat, name),
            (Some(cat), None) => format!("{}/", cat),
            (None, Some(name)) => name.to_string(),
            (None, None) => return Ok(Vec::new()),
        };

        search_cmd.push(&search_pattern);

        let output = self.run_portage_command(&search_cmd).await?;
        self.parse_search_results(&output).await
    }

    pub async fn get_installed_packages(&self) -> Result<Vec<PackageInfo>> {
        debug!("Getting list of installed packages");

        let output = self.run_portage_command(&[
            "qlist", "-I", "-v"
        ]).await?;

        self.parse_package_list(&output).await
    }

    async fn count_installed_packages(&self) -> Result<u32> {
        let output = self.run_portage_command(&[
            "equery", "list", "-C"
        ]).await?;

        Ok(output.lines().count() as u32)
    }

    async fn count_available_updates(&self) -> Result<u32> {
        let output = self.run_portage_command(&[
            "emerge", "--pretend", "--quiet", "--update", "--deep", "@world"
        ]).await?;

        Ok(output.lines().count() as u32)
    }

    async fn count_world_packages(&self) -> Result<u32> {
        let output = self.run_portage_command(&[
            "equery", "list", "-w"
        ]).await?;

        Ok(output.lines().count() as u32)
    }

    async fn get_last_sync_time(&self) -> Result<Option<DateTime<Utc>>> {
        let emerge_log_path = "/var/log/emerge.log";

        if !Path::new(emerge_log_path).exists() {
            return Ok(None);
        }

        let output = Command::new("tail")
            .args(["-n", "100", emerge_log_path])
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to read emerge log: {}", e)))?;

        let content = String::from_utf8_lossy(&output.stdout);

        // Look for sync completion entries
        for line in content.lines().rev() {
            if line.contains("=== Sync completed") {
                if let Some(timestamp) = line.split_whitespace().next() {
                    if let Ok(secs) = timestamp.parse::<i64>() {
                        return Ok(Some(DateTime::from_timestamp(secs, 0).unwrap_or_default()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_portage_version(&self) -> Result<String> {
        let output = self.run_portage_command(&[
            "emerge", "--version"
        ]).await?;

        let first_line = output.lines().next()
            .ok_or_else(|| PortCLError::Portage("Failed to get portage version".to_string()))?;

        // Extract version from "Portage 3.0.30 (python 3.11.6-1, etc.)"
        let version = first_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }

    async fn get_current_profile(&self) -> Result<String> {
        let output = Command::new("eselect")
            .args(["profile", "show"])
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to get profile: {}", e)))?;

        let content = String::from_utf8_lossy(&output.stdout);
        let profile = content.trim().to_string();

        Ok(profile)
    }

    async fn get_system_arch(&self) -> Result<String> {
        let output = Command::new("uname")
            .arg("-m")
            .output()
            .map_err(|e| PortCLError::System(format!("Failed to get arch: {}", e)))?;

        let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(arch)
    }

    async fn run_portage_command(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&args[0])
            .args(&args[1..])
            .output()
            .map_err(|e| PortCLError::Portage(format!(
                "Failed to run {}: {}", args[0], e
            )))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(PortCLError::Portage(format!(
                "Portage command failed: {}", error_msg
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn parse_package_name(&self, package: &str) -> Result<(String, String)> {
        if package.contains('/') {
            let parts: Vec<&str> = package.splitn(2, '/').collect();
            if parts.len() == 2 {
                return Ok((parts[0].to_string(), parts[1].to_string()));
            }
        }

        // If no category specified, try to find it
        Err(PortCLError::Portage(format!(
            "Package name must include category: {}", package
        )))
    }

    fn parse_installed_version(&self, output: &str) -> Result<String> {
        // Parse output like: "sys-apps/portage-3.0.30"
        let full_name = output.lines().next()
            .ok_or_else(|| PortCLError::Portage("Empty package list output".to_string()))?;

        // Extract version after the last hyphen
        let version = full_name.rsplit('-').next()
            .ok_or_else(|| PortCLError::Portage("Could not parse version".to_string()))?;

        Ok(version.to_string())
    }

    async fn get_package_metadata(&self, category: &str, name: &str, version: &str) -> Result<PackageInfo> {
        // Use equery to get detailed package information
        let info_output = self.run_portage_command(&[
            "equery", "list", "-o", &format!("{}/{}-{}", category, name, version)
        ]).await?;

        // Use equery to get metadata
        let metadata_output = self.run_portage_command(&[
            "equery", "meta", &format!("{}/{}", category, name)
        ]).await?;

        self.parse_package_metadata(category, name, version, &metadata_output)
    }

    fn parse_package_metadata(&self, category: &str, name: &str, version: &str, output: &str) -> Result<PackageInfo> {
        let mut package_info = PackageInfo {
            name: name.to_string(),
            version: version.to_string(),
            category: category.to_string(),
            slot: None,
            repository: None,
            installed_size: None,
            homepage: None,
            description: None,
            license: None,
            use_flags: Vec::new(),
            dependencies: Vec::new(),
            build_time: None,
            last_modified: None,
        };

        // Parse equery meta output
        for line in output.lines() {
            if line.starts_with("Description: ") {
                package_info.description = Some(line["Description: ".len()..].trim().to_string());
            } else if line.starts_with("Homepage: ") {
                package_info.homepage = Some(line["Homepage: ".len()..].trim().to_string());
            } else if line.starts_with("License: ") {
                package_info.license = Some(line["License: ".len()..].trim().to_string());
            } else if line.starts_with("Slot: ") {
                package_info.slot = Some(line["Slot: ".len()..].trim().to_string());
            } else if line.starts_with("Repository: ") {
                package_info.repository = Some(line["Repository: ".len()..].trim().to_string());
            } else if line.starts_with("Size: ") {
                // Parse size like "Size: 1234567 Bytes"
                let size_str = line["Size: ".len()..].split_whitespace().next().unwrap_or("0");
                package_info.installed_size = size_str.parse().ok();
            }
        }

        Ok(package_info)
    }

    async fn parse_search_results(&self, output: &str) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        // Parse emerge --search output
        // Example: "sys-apps/portage-3.0.30 [masked]"
        for line in output.lines() {
            if line.contains('*') || line.trim().is_empty() {
                continue;
            }

            if let Some(package_part) = line.split_whitespace().next() {
                if let Ok((category, name)) = self.parse_package_name(package_part) {
                    // This is a simplified parsing - in practice, you'd want more robust parsing
                    let package_info = PackageInfo {
                        name: name.to_string(),
                        version: "unknown".to_string(), // Would parse from full output
                        category: category.to_string(),
                        slot: None,
                        repository: None,
                        installed_size: None,
                        homepage: None,
                        description: None,
                        license: None,
                        use_flags: Vec::new(),
                        dependencies: Vec::new(),
                        build_time: None,
                        last_modified: None,
                    };
                    packages.push(package_info);
                }
            }
        }

        Ok(packages)
    }

    async fn parse_package_list(&self, output: &str) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        // Parse qlist output, one package per line
        for line in output.lines() {
            if let Ok((category, name_version)) = self.parse_package_name(line) {
                // Extract version from name-version combination
                if let Some((name, version)) = name_version.rsplit_once('-') {
                    let package_info = PackageInfo {
                        name: name.to_string(),
                        version: version.to_string(),
                        category: category.to_string(),
                        slot: None,
                        repository: None,
                        installed_size: None,
                        homepage: None,
                        description: None,
                        license: None,
                        use_flags: Vec::new(),
                        dependencies: Vec::new(),
                        build_time: None,
                        last_modified: None,
                    };
                    packages.push(package_info);
                }
            }
        }

        Ok(packages)
    }
}