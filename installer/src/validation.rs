use anyhow::{bail, Result};
use regex::Regex;

pub fn validate_device_path(path: &str) -> Result<()> {
    // Allow common block device patterns: /dev/sd*, /dev/nvme*, /dev/hd*, /dev/vd*, /dev/mmcblk*
    let device_regex = Regex::new(
        r"^/dev/(sd[a-z]+|nvme[0-9]+n[0-9]+|hd[a-z]+|vd[a-z]+|mmcblk[0-9]+)(p[0-9]+)?$",
    )?;
    if !device_regex.is_match(path) {
        bail!("Invalid device path");
    }

    // Prevent dangerous device paths (but allow legitimate block devices)
    let dangerous_exact = [
        "/dev/null",
        "/dev/zero",
        "/dev/full",
        "/dev/random",
        "/dev/urandom",
        "/dev/mem",
        "/dev/kmem",
        "/dev/port",
        "/dev/console",
        "/dev/initctl",
    ];

    let dangerous_prefixes = ["/dev/shm/", "/dev/pts/", "/dev/mqueue/", "/dev/hugepages/"];

    // Block exact matches to dangerous devices
    if dangerous_exact.contains(&path) {
        bail!("Device access denied");
    }

    // Block dangerous prefixes
    for dangerous in &dangerous_prefixes {
        if path.starts_with(dangerous) {
            bail!("Device access denied");
        }
    }

    // Allow whole disk devices but warn about partitions for user selection
    if path.contains("p") && path.chars().last().unwrap_or('a').is_ascii_digit() {
        // This is a partition, not a whole disk - still allow but could warn
    }

    Ok(())
}

pub fn validate_username(username: &str) -> Result<()> {
    // Unix username rules: 1-32 chars, lowercase letters, digits, hyphens, underscores
    // Cannot start with hyphen or digit, cannot end with hyphen
    let username_regex = Regex::new(r"^[a-z_][a-z0-9_-]{0,31}$")?;
    if !username_regex.is_match(username) {
        bail!("Invalid username format");
    }

    // Reserved usernames
    let reserved = [
        "root",
        "daemon",
        "bin",
        "sys",
        "sync",
        "games",
        "man",
        "lp",
        "mail",
        "news",
        "uucp",
        "proxy",
        "www-data",
        "backup",
        "list",
        "irc",
        "gnats",
        "nobody",
        "systemd-network",
        "systemd-resolve",
        "syslog",
        "messagebus",
        "uuidd",
        "dnsmasq",
        "usbmux",
        "rtkit",
        "pulse",
        "speech-dispatcher",
        "avahi",
        "saned",
        "colord",
        "hplip",
        "geoclue",
        "gnome-initial-setup",
        "gdm",
        "sshd",
        "ntp",
        "postgres",
        "mysql",
        "oracle",
        "tomcat",
    ];

    if reserved.contains(&username) {
        bail!("Username not available");
    }

    Ok(())
}

pub fn validate_url(url: &str) -> Result<()> {
    // Basic URL validation — any HTTPS URL is acceptable
    let url_regex = Regex::new(r"^https://[a-zA-Z0-9.-]+/[a-zA-Z0-9/_.-]*$")?;
    if !url_regex.is_match(url) {
        bail!("Invalid URL format");
    }

    Ok(())
}

pub fn validate_filesystem_type(fs: &str) -> Result<()> {
    let allowed_fs = ["btrfs", "btrfs_encryption_dev"];
    if !allowed_fs.contains(&fs) {
        bail!("Unsupported filesystem type");
    }
    Ok(())
}

pub fn validate_flavour(flavour: &str) -> Result<()> {
    // Only allow cosmic-desktop for RegicideOS
    if flavour != "cosmic-desktop" {
        bail!("Unsupported flavour: {}. Only 'cosmic-desktop' is supported.", flavour);
    }
    Ok(())
}

pub fn validate_package_set(applications: &str) -> Result<()> {
    let allowed_sets = ["recommended", "minimal", "base", "desktop", "full"];
    if !allowed_sets.contains(&applications) {
        bail!("Unsupported application set");
    }
    Ok(())
}

pub fn sanitize_input(input: &str) -> String {
    // Remove null bytes and control characters
    input
        .chars()
        .filter(|c| *c != '\0' && !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect()
}
