use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub size: String,
    pub label: Option<String>,
    pub format: String,
    #[serde(rename = "type")]
    pub partition_type: String,
    pub subvolumes: Option<Vec<String>>,
    pub inside: Option<Box<Partition>>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub drive: String,
    pub repository: String,
    pub flavour: String,
    pub release_branch: String,
    pub filesystem: String,
    pub username: String,
    pub applications: String,
}

pub fn check_username(username: &str) -> bool {
    if username.is_empty() {
        return true;
    }
    
    let regex = Regex::new(r"^[a-z_][a-z0-9_]{0,30}$").unwrap();
    regex.is_match(username)
}

pub fn human_to_bytes(size: &str) -> Result<u64> {
    if size.is_empty() {
        return Ok(0);
    }
    
    let (number, unit) = size.split_at(size.len() - 1);
    let number: u64 = number.parse().map_err(|_| anyhow::anyhow!("Invalid size number"))?;
    
    let multiplier = match unit.to_uppercase().as_str() {
        "B" => 1,
        "K" => 1024,
        "M" => 1024_u64.pow(2),
        "G" => 1024_u64.pow(3),
        "T" => 1024_u64.pow(4),
        "P" => 1024_u64.pow(5),
        _ => bail!("Invalid size unit: {}", unit),
    };
    
    Ok(number * multiplier)
}

pub fn is_efi() -> bool {
    Path::new("/sys/firmware/efi").exists()
}

pub fn get_fs() -> Vec<String> {
    vec!["btrfs".to_string(), "btrfs_encryption_dev".to_string()]
}

pub fn get_package_sets() -> Vec<String> {
    vec!["recommended".to_string(), "minimal".to_string()]
}

pub fn get_flatpak_packages(applications_set: &str) -> String {
    let package_sets: HashMap<&str, Vec<&str>> = [
        ("recommended", vec![
            "io.gitlab.librewolf-community",
            "org.mozilla.Thunderbird",
            "org.gnome.TextEditor",
            "org.gnome.Rhythmbox3",
            "org.gnome.Calculator",
            "org.gnome.Totem",
            "org.gnome.Loupe",
            "org.libreoffice.LibreOffice"
        ]),
        ("minimal", vec![
            "dev.zed.Zed"
        ])
    ].into_iter().collect();
    
    package_sets.get(applications_set)
        .map(|packages| packages.join(" "))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_username_valid() {
        assert!(check_username(""));  // Empty username is allowed
        assert!(check_username("user"));
        assert!(check_username("_user"));
        assert!(check_username("user123"));
        assert!(check_username("user_name"));
        assert!(check_username("a"));
    }

    #[test]
    fn test_check_username_invalid() {
        assert!(!check_username("User"));  // Capital letters not allowed
        assert!(!check_username("123user"));  // Can't start with number
        assert!(!check_username("user-name"));  // Dashes not allowed
        assert!(!check_username("user@domain"));  // Special chars not allowed
        assert!(!check_username("thisusernameistoolongtobevalidbecauseitisover30characters"));  // Too long (>30 chars)
    }

    #[test]
    fn test_human_to_bytes() -> Result<()> {
        assert_eq!(human_to_bytes("512B")?, 512);
        assert_eq!(human_to_bytes("1K")?, 1024);
        assert_eq!(human_to_bytes("2M")?, 2 * 1024 * 1024);
        assert_eq!(human_to_bytes("3G")?, 3 * 1024_u64.pow(3));
        assert_eq!(human_to_bytes("1T")?, 1024_u64.pow(4));
        assert_eq!(human_to_bytes("1P")?, 1024_u64.pow(5));
        
        // Test edge cases - empty string returns 0
        assert_eq!(human_to_bytes("")?, 0);
        
        // Test error cases
        assert!(human_to_bytes("invalid").is_err());
        assert!(human_to_bytes("512X").is_err());
        
        Ok(())
    }

    #[test]
    fn test_get_fs() {
        let filesystems = get_fs();
        assert!(filesystems.contains(&"btrfs".to_string()));
        assert!(filesystems.contains(&"btrfs_encryption_dev".to_string()));
        assert_eq!(filesystems.len(), 2);
    }

    #[test]
    fn test_get_package_sets() {
        let package_sets = get_package_sets();
        assert!(package_sets.contains(&"recommended".to_string()));
        assert!(package_sets.contains(&"minimal".to_string()));
        assert_eq!(package_sets.len(), 2);
    }

    #[test]
    fn test_get_flatpak_packages() {
        let recommended_packages = get_flatpak_packages("recommended");
        assert!(!recommended_packages.is_empty());
        assert!(recommended_packages.contains("io.gitlab.librewolf-community"));
        assert!(recommended_packages.contains("org.libreoffice.LibreOffice"));

        let minimal_packages = get_flatpak_packages("minimal");
        assert!(!minimal_packages.is_empty());
        assert!(minimal_packages.contains("dev.zed.Zed"));

        let unknown_packages = get_flatpak_packages("unknown");
        assert!(unknown_packages.is_empty());
    }

    #[test]
    fn test_partition_struct() {
        let partition = Partition {
            size: "512M".to_string(),
            label: Some("EFI".to_string()),
            format: "vfat".to_string(),
            partition_type: "uefi".to_string(),
            subvolumes: None,
            inside: None,
        };
        
        assert_eq!(partition.size, "512M");
        assert_eq!(partition.label.unwrap(), "EFI");
        assert_eq!(partition.format, "vfat");
        assert_eq!(partition.partition_type, "uefi");
    }

    #[test]
    fn test_config_struct() {
        let config = Config {
            drive: "/dev/sda".to_string(),
            repository: "https://repo.xenialinux.com/releases/".to_string(),
            flavour: "cosmic-fedora".to_string(),
            release_branch: "main".to_string(),
            filesystem: "btrfs".to_string(),
            username: "testuser".to_string(),
            applications: "recommended".to_string(),
        };
        
        assert_eq!(config.drive, "/dev/sda");
        assert_eq!(config.repository, "https://repo.xenialinux.com/releases/");
        assert_eq!(config.flavour, "cosmic-desktop");
        assert_eq!(config.release_branch, "main");
        assert_eq!(config.filesystem, "btrfs");
        assert_eq!(config.username, "testuser");
        assert_eq!(config.applications, "recommended");
    }
}

// Integration tests for config validation
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_regicide_restrictions() {
        // Test that RegicideOS enforces cosmic-desktop and Xenia repository
        let valid_config = Config {
            drive: "/dev/sda".to_string(),
            repository: "https://repo.xenialinux.com/releases/".to_string(),
            flavour: "cosmic-fedora".to_string(),
            release_branch: "main".to_string(),
            filesystem: "btrfs".to_string(),
            username: "user".to_string(),
            applications: "recommended".to_string(),
        };

        // This would be the ideal config for RegicideOS
        assert_eq!(valid_config.repository, "https://repo.xenialinux.com/releases/");
        assert_eq!(valid_config.flavour, "cosmic-desktop");
    }
    
    #[test]
    fn test_filesystem_options() {
        let filesystems = get_fs();
        
        // RegicideOS should support both regular and encrypted BTRFS
        assert!(filesystems.contains(&"btrfs".to_string()));
        assert!(filesystems.contains(&"btrfs_encryption_dev".to_string()));
        
        // Should not contain any other filesystem types
        for fs in &filesystems {
            assert!(fs.contains("btrfs"));
        }
    }
}
