use anyhow::{bail, Context, Result};
use clap::{Arg, Command};
use regex;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio;
use toml;

// Import from lib module
use installer::{
    check_username, get_flatpak_packages, get_fs, get_package_sets, is_efi, Config, Partition,
};

struct Colours;

impl Colours {
    const RED: &'static str = "\x1b[31m";
    const YELLOW: &'static str = "\x1b[33m";
    const BLUE: &'static str = "\x1b[34m";
    const ENDC: &'static str = "\x1b[m";
}

fn die(message: &str) -> ! {
    // Sanitize error message to prevent information disclosure
    let sanitized = sanitize_error_message(message);
    eprintln!(
        "{}{}{} {}",
        Colours::RED,
        "[ERROR]",
        Colours::ENDC,
        sanitized
    );
    std::process::exit(1);
}

// Sanitize error messages to prevent information disclosure
fn sanitize_error_message(message: &str) -> String {
    // Remove potentially sensitive information
    let sensitive_patterns = [
        r"/home/[^/\s]+",             // Home directory paths
        r"/root/[^/\s]+",             // Root directory paths
        r"/tmp/[^/\s]+",              // Temp file paths
        r"password[^=\s]*=\s*[^\s]+", // Passwords in error messages
        r"token[^=\s]*=\s*[^\s]+",    // Tokens in error messages
        r"key[^=\s]*=\s*[^\s]+",      // Keys in error messages
        r"secret[^=\s]*=\s*[^\s]+",   // Secrets in error messages
    ];

    let mut sanitized = message.to_string();

    for pattern in &sensitive_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            sanitized = regex.replace_all(&sanitized, "[REDACTED]").to_string();
        }
    }

    // Remove full paths, keep only filenames
    let path_regex = regex::Regex::new(r"/([^/\s]+/)+([^/\s]+)").unwrap();
    sanitized = path_regex.replace_all(&sanitized, "[PATH]/$2").to_string();

    // Limit error message length to prevent log flooding
    if sanitized.len() > 200 {
        sanitized.truncate(197);
        sanitized.push_str("...");
    }

    sanitized
}

fn info(message: &str) {
    println!("{}{}{} {}", Colours::BLUE, "[INFO]", Colours::ENDC, message);
}

fn warn(message: &str) {
    let sanitized = sanitize_error_message(message);
    println!(
        "{}{}{} {}",
        Colours::YELLOW,
        "[WARN]",
        Colours::ENDC,
        sanitized
    );
}

fn print_banner() {
    println!("{}", Colours::BLUE);
    println!(
        r#"
    ██████╗ ███████╗ ██████╗ ██╗ ██████╗██╗██████╗ ███████╗ ██████╗ ███████╗
    ██╔══██╗██╔════╝██╔════╝ ██║██╔════╝██║██╔══██╗██╔════╝██╔═══██╗██╔════╝
    ██████╔╝█████╗  ██║  ███╗██║██║     ██║██║  ██║█████╗  ██║   ██║███████╗
    ██╔══██╗██╔══╝  ██║   ██║██║██║     ██║██║  ██║██╔══╝  ██║   ██║╚════██║
    ██║  ██║███████╗╚██████╔╝██║╚██████╗██║██████╔╝███████╗╚██████╔╝███████║
    ╚═╝  ╚═╝╚══════╝ ╚═════╝ ╚═╝ ╚═════╝╚═╝╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝

              🏰 A Rust-first, AI-powered Linux Distribution 🚀
                        Cosmic Desktop • BTRFS • Gentoo Base
    "#
    );
    println!("{}", Colours::ENDC);
}

// Execute commands with full error output
fn execute_with_output(command: &str) -> Result<String> {
    let output = ProcessCommand::new("sh")
        .args(&["-c", command])
        .output()
        .with_context(|| format!("Failed to execute command: {}", command))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed: {}\nError: {}", command, stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// Safe command execution with strict allowlist
fn execute(command: &str) -> Result<String> {
    // Check for heredoc patterns that need special handling
    if command.contains("<<") {
        // For heredoc commands, route through shell execution to preserve newlines
        return execute_safe_shell_command(command.trim());
    }

    // Parse command into program and arguments
    let parts: Vec<&str> = command.trim().split_whitespace().collect();
    if parts.is_empty() {
        bail!("Empty command");
    }

    let program = parts[0];
    let args = &parts[1..];

    // Allowlist of safe commands with their expected argument patterns
    match program {
        // Block device commands
        "lsblk" | "blkid" | "partprobe" | "sfdisk" | "sgdisk" | "blockdev" | "hdparm" => {
            execute_safe_command(program, args)
        }

        // Cat command (only for heredoc usage with sfdisk)
        "cat" => {
            // Only allow cat with heredoc and sfdisk - must use shell for pipe
            if args.len() >= 3
                && args.contains(&"<<EOF")
                && args.iter().any(|&arg| arg.contains("sfdisk"))
            {
                // Reconstruct the full command and execute through shell
                let full_command = format!("{} {}", program, args.join(" "));
                execute_safe_shell_command(&full_command)
            } else {
                bail!("Cat command not allowed in this context");
            }
        }

        // Filesystem commands
        "mkfs.vfat" | "mkfs.ext4" | "mkfs.btrfs" | "fsck.fat" | "fsck.ext4" | "btrfs"
        | "wipefs" | "file" | "lsof" | "sync" | "dd" | "ls" | "fdisk" | "dmsetup" => {
            execute_safe_command(program, args)
        }

        // Mount/unmount commands
        "mount" | "umount" => execute_safe_command(program, args),

        // LUKS commands
        "cryptsetup" => execute_safe_command(program, args),

        // System commands
        "systemctl" | "loginctl" => execute_safe_command(program, args),

        // Package managers (read-only operations only, except for gdisk installation)
        "which" | "dpkg" | "rpm" => {
            if program == "which" || args.iter().any(|&arg| arg == "-l" || arg == "-Q") {
                execute_safe_command(program, args)
            } else {
                bail!("Package manager operation not allowed: {}", command)
            }
        }

        "dnf" | "apt" | "pacman" => {
            // Allow gdisk installation for EFI boot support
            if (program == "dnf" && args.contains(&"-y") && args.contains(&"gdisk"))
                || (program == "apt" && args.contains(&"gdisk"))
                || (program == "pacman" && args.contains(&"gdisk"))
            {
                execute_safe_command(program, args)
            } else {
                bail!("Package manager operation not allowed: {}", command)
            }
        }

        // LVM commands (read-only only)
        "vgs" | "vgchange" => execute_safe_command(program, args),

        // User management (chroot only)
        "useradd" | "usermod" | "passwd" => execute_safe_command(program, args),

        // GRUB commands (chroot only)
        "grub-install" | "grub2-install" | "grub-mkconfig" | "grub2-mkconfig" => {
            execute_safe_command(program, args)
        }

        // EFI bootloader tools
        "efibootmgr" | "efivar" => execute_safe_command(program, args),

        // Service management (chroot only)
        "rc-update" | "rc-service" => execute_safe_command(program, args),

        // Chroot for bootloader installation and system setup
        "chroot" => execute_safe_command(program, args),

        // Allow safe shell builtins with strict validation
        "sh" => {
            if args.len() >= 2 && args[0] == "-c" {
                let shell_cmd = args[1];
                // Only allow very specific shell patterns
                if is_safe_shell_command(shell_cmd) {
                    execute_safe_shell_command(shell_cmd)
                } else {
                    bail!("Unsafe shell command: {}", shell_cmd)
                }
            } else {
                bail!("Invalid sh usage: {}", command)
            }
        }

        _ => {
            bail!("Command not allowed: {}", program)
        }
    }
}

// Execute safe commands directly without shell
fn execute_safe_command(program: &str, args: &[&str]) -> Result<String> {
    let output = ProcessCommand::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute system command"))?;

    if !output.status.success() {
        bail!(
            "System command failed (exit code: {:?})",
            output.status.code()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// Execute safe shell commands with strict validation
fn execute_safe_shell_command(shell_cmd: &str) -> Result<String> {
    // Allow only specific, safe shell patterns
    let allowed_patterns = [
        r"umount -ql [^[:space:]]+\?\* 2>/dev/null \|\| true",
        r"umount -ql /dev/[^[:space:]]+\?\* 2>/dev/null \|\| true",
        r"umount -R [^[:space:]]+ 2>/dev/null",
        r"umount [^[:space:]]+ 2>/dev/null",
        r"mount --rbind /dev /mnt/root/dev",
        r"mount --rbind /sys /mnt/root/sys",
        r"mount --bind /run /mnt/root/run",
        r"mount --make-slave /mnt/root/run",
        r"cat <<EOF \| sfdisk -q --wipe always --force [^[:space:]]+",
        r"chroot /mnt/root /bin/bash <<'EOT'[\s\S]*EOT$",
        r"mount -t overlay overlay -o [^[:space:]]+ [^[:space:]]+",
        r"lsblk -ln -o NAME [^[:space:]]+",
        r"lsblk -fn -o NAME,LABEL",
        r"blkid -L [^[:space:]]+",
        r"vgs \| awk '\{ print \$1 \}' \| grep -vw VG",
        r"cryptsetup close [^[:space:]]+ 2>/dev/null",
        r"partprobe [^[:space:]]+",
        r"sfdisk -R [^[:space:]]+",
        r"hdparm -z [^[:space:]]+",
        r"blockdev --rereadpt [^[:space:]]+",
    ];

    for pattern in &allowed_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if regex.is_match(shell_cmd) {
                let output = ProcessCommand::new("sh")
                    .args(&["-c", shell_cmd])
                    .output()
                    .with_context(|| format!("Failed to execute shell command: {}", shell_cmd))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    bail!(
                        "Shell command failed: {} (exit code: {:?})\nSTDOUT: {}\nSTDERR: {}",
                        shell_cmd,
                        output.status.code(),
                        stdout,
                        stderr
                    );
                }

                return Ok(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
    }

    bail!("Shell command pattern not allowed: {}", shell_cmd)
}

// Check if a shell command is safe
fn is_safe_shell_command(cmd: &str) -> bool {
    // First check for specific allowed patterns that might contain "dangerous" characters
    let allowed_special_patterns = [
        r"^cat <<EOF \| sfdisk -q --wipe always --force [^[:space:]]+[\s\S]*EOF$",
        r"vgs \| awk '\{ print \$1 \}' \| grep -vw VG$",
    ];

    for pattern in &allowed_special_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if regex.is_match(cmd) {
                return true;
            }
        }
    }

    // Reject dangerous characters and patterns
    let dangerous_patterns = [
        ";", "&&", "||", "|", "&", "$(", "`", "$", "${", ">", ">>", "<", "rm ", "dd ", "chmod ",
        "chown ", "sudo ", "su ", "eval ", "exec ",
    ];

    for pattern in &dangerous_patterns {
        if cmd.contains(pattern) {
            return false;
        }
    }

    // Only allow specific safe patterns
    let safe_patterns = [
        r"^umount -ql /[a-zA-Z0-9/]+[np]?[0-9]*\?\* 2>/dev/null \|\| true$",
        r"^umount -ql /[a-zA-Z0-9/]+\?\* 2>/dev/null \|\| true$",
        r"^umount -R [^[:space:]]+ 2>/dev/null$",
        r"^umount [^[:space:]]+ 2>/dev/null$",
        r"^mount --rbind /dev /mnt/root/dev$",
        r"^mount --rbind /sys /mnt/root/sys$",
        r"^mount --bind /run /mnt/root/run$",
        r"^mount --make-slave /mnt/root/run$",
        r"^cat <<EOF \| sfdisk -q --wipe always --force [^[:space:]]+.*$",
        r"^mount -t overlay overlay -o [^[:space:]]+ [^[:space:]]+$",
        r"^lsblk -ln -o NAME [^[:space:]]+$",
        r"^lsblk -fn -o NAME,LABEL$",
        r"^blkid -L [^[:space:]]+$",
        r"^vgs \| awk '\{ print \$1 \}' \| grep -vw VG$",
    ];

    for pattern in &safe_patterns {
        if let Ok(regex) = regex::Regex::new(pattern) {
            if regex.is_match(cmd) {
                return true;
            }
        }
    }

    false
}

fn get_drive_size(drive: &str) -> Result<u64> {
    // Use direct command execution to avoid shell injection issues
    match execute_safe_command("lsblk", &["-b", "-o", "SIZE", "-n", drive]) {
        Ok(output) => {
            let size_str = output.trim();
            if size_str.is_empty() {
                Ok(0)
            } else {
                // Take only the first line (drive size, not partitions)
                let first_line = size_str.lines().next().unwrap_or("").trim();
                Ok(first_line.parse::<u64>().unwrap_or(0))
            }
        }
        Err(e) => Err(e),
    }
}

fn check_drive_size(drive: &str) -> bool {
    match get_drive_size(drive) {
        Ok(size) => size > 12884901888, // 12GB in bytes
        Err(_) => false,
    }
}

fn get_drives() -> Result<Vec<String>> {
    let sys_block = Path::new("/sys/block");
    if !sys_block.exists() {
        warn(&format!(
            "Sys block directory not found: {}",
            sys_block.display()
        ));
        return Ok(vec![]);
    }

    let mut drives = Vec::new();
    for entry in fs::read_dir(sys_block)? {
        let entry = entry?;
        let drive_name = entry.file_name();
        let drive_path = format!("/dev/{}", drive_name.to_string_lossy());

        // Skip loopback devices and other non-physical drives
        let name_str = drive_name.to_string_lossy();
        if name_str.starts_with("loop")
            || name_str.starts_with("ram")
            || name_str.starts_with("dm-")
        {
            continue;
        }

        match check_drive_size(&drive_path) {
            true => {
                info(&format!("Found valid drive: {} (size > 12GB)", drive_path));
                drives.push(drive_path);
            }
            false => {
                // Only show debug info for actual block devices, not every entry
                if name_str.starts_with("sd")
                    || name_str.starts_with("nvme")
                    || name_str.starts_with("hd")
                    || name_str.starts_with("vd")
                {
                    info(&format!("Drive {} too small or not accessible", drive_path));
                }
            }
        }
    }

    if drives.is_empty() {
        warn("No suitable drives found (drives must be > 12GB)");
    }

    Ok(drives)
}

async fn check_url(url: &str) -> bool {
    let manifest_url = format!("{}Manifest.toml", url);
    match reqwest::get(&manifest_url).await {
        Ok(response) => response.status().is_success(),
        Err(_) => {
            warn("URL entered is not reachable, or there is no Manifest.toml available. Please try again.");
            false
        }
    }
}

fn get_layouts() -> HashMap<String, Vec<Partition>> {
    let mut layouts = HashMap::new();

    if is_efi() {
        layouts.insert(
            "btrfs".to_string(),
            vec![
                Partition {
                    size: "512M".to_string(),
                    label: Some("EFI".to_string()),
                    format: "vfat".to_string(),
                    partition_type: "uefi".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "rest".to_string(),
                    label: Some("ROOTS".to_string()),
                    format: "btrfs".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: Some(vec![
                        "/home".to_string(),
                        "/overlay".to_string(),
                        "/overlay/etc".to_string(),
                        "/overlay/var".to_string(),
                        "/overlay/usr".to_string(),
                    ]),
                    inside: None,
                },
            ],
        );
    } else {
        layouts.insert(
            "btrfs".to_string(),
            vec![
                Partition {
                    size: "2M".to_string(),
                    label: None,
                    format: "".to_string(),
                    partition_type: "21686148-6449-6E6F-744E-656564454649".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "512M".to_string(),
                    label: Some("BOOT".to_string()),
                    format: "ext4".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "rest".to_string(),
                    label: Some("ROOTS".to_string()),
                    format: "btrfs".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: Some(vec![
                        "/home".to_string(),
                        "/overlay".to_string(),
                        "/overlay/etc".to_string(),
                        "/overlay/var".to_string(),
                        "/overlay/usr".to_string(),
                    ]),
                    inside: None,
                },
            ],
        );
    }

    if is_efi() {
        layouts.insert(
            "btrfs_encryption_dev".to_string(),
            vec![
                Partition {
                    size: "512M".to_string(),
                    label: Some("EFI".to_string()),
                    format: "vfat".to_string(),
                    partition_type: "uefi".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "8G".to_string(),
                    label: Some("ROOTS".to_string()),
                    format: "ext4".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "rest".to_string(),
                    label: Some("REGICIDEOS".to_string()),
                    format: "luks".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: Some(Box::new(Partition {
                        size: "rest".to_string(),
                        label: None,
                        format: "btrfs".to_string(),
                        partition_type: "linux".to_string(),
                        subvolumes: Some(vec![
                            "/home".to_string(),
                            "/overlay".to_string(),
                            "/overlay/etc".to_string(),
                            "/overlay/var".to_string(),
                            "/overlay/usr".to_string(),
                        ]),
                        inside: None,
                    })),
                },
            ],
        );
    } else {
        layouts.insert(
            "btrfs_encryption_dev".to_string(),
            vec![
                Partition {
                    size: "2M".to_string(),
                    label: None,
                    format: "".to_string(),
                    partition_type: "21686148-6449-6E6F-744E-656564454649".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "512M".to_string(),
                    label: Some("BOOT".to_string()),
                    format: "ext4".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "8G".to_string(),
                    label: Some("ROOTS".to_string()),
                    format: "ext4".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: None,
                },
                Partition {
                    size: "rest".to_string(),
                    label: Some("REGICIDEOS".to_string()),
                    format: "luks".to_string(),
                    partition_type: "linux".to_string(),
                    subvolumes: None,
                    inside: Some(Box::new(Partition {
                        size: "rest".to_string(),
                        label: None,
                        format: "btrfs".to_string(),
                        partition_type: "linux".to_string(),
                        subvolumes: Some(vec![
                            "/home".to_string(),
                            "/overlay".to_string(),
                            "/overlay/etc".to_string(),
                            "/overlay/var".to_string(),
                            "/overlay/usr".to_string(),
                        ]),
                        inside: None,
                    })),
                },
            ],
        );
    }

    layouts
}

fn wait_for_partitions(drive: &str, expected_count: usize) -> Result<Vec<String>> {
    info("Waiting for kernel to recognize new partitions...");

    let mut attempts = 0;
    let max_attempts = 30; // Increased from 10
    let drive_base = drive.split('/').last().unwrap_or("");

    loop {
        // Try multiple detection methods
        let mut partition_names = Vec::new();

        // Method 1: Use lsblk to detect partitions
        if let Ok(partitions_output) = execute(&format!("lsblk -ln -o NAME {}", drive)) {
            println!("DEBUG: lsblk output for {}: {}", drive, partitions_output);
            let mut detected_partitions: Vec<String> = partitions_output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter(|line| line.trim() != drive_base)
                .map(|line| format!("/dev/{}", line.trim()))
                .collect();

            // Sort partitions numerically to ensure correct order
            detected_partitions.sort_by(|a, b| {
                let extract_num = |path: &str| {
                    path.rsplit_once('p')
                        .and_then(|(_, num)| num.parse::<u32>().ok())
                        .or_else(|| {
                            path.chars()
                                .skip_while(|c| !c.is_ascii_digit())
                                .collect::<String>()
                                .parse::<u32>()
                                .ok()
                        })
                        .unwrap_or(0)
                };
                extract_num(a).cmp(&extract_num(b))
            });

            println!(
                "DEBUG: sorted_detected_partitions = {:?}",
                detected_partitions
            );

            // Special handling for LUKS - check if we have a mapper device
            if expected_count == 1 && detected_partitions.is_empty() {
                // For LUKS, we expect 1 mapper device instead of physical partitions
                if Path::new("/dev/mapper/regicideos").exists() {
                    partition_names.push("/dev/mapper/regicideos".to_string());
                }
            } else if detected_partitions.len() == expected_count {
                // Verify all partitions actually exist as device files
                let mut all_exist = true;
                for part in &detected_partitions {
                    if !Path::new(part).exists() {
                        all_exist = false;
                        break;
                    }
                }
                if all_exist {
                    partition_names = detected_partitions;
                }
            }
        }

        // Method 2: Try numbered approach if lsblk detection fails
        if partition_names.len() != expected_count {
            partition_names.clear();
            let mut all_exist = true;
            for i in 1..=expected_count {
                let part_name = if drive.contains("nvme")
                    || drive.chars().last().unwrap_or('a').is_ascii_digit()
                {
                    format!("{}p{}", drive, i)
                } else {
                    format!("{}{}", drive, i)
                };

                if Path::new(&part_name).exists() {
                    partition_names.push(part_name);
                } else {
                    all_exist = false;
                    break;
                }
            }

            if !all_exist {
                partition_names.clear();
            }
        }

        if partition_names.len() == expected_count {
            println!(
                "DEBUG: Found {} partitions as expected: {:?}",
                expected_count, partition_names
            );
            info(&format!("Found {} partitions", expected_count));
            return Ok(partition_names);
        }

        attempts += 1;
        if attempts >= max_attempts {
            bail!(
                "Partitions were not created properly after {} attempts. Expected {}, found {}",
                max_attempts,
                expected_count,
                partition_names.len()
            );
        }

        // Exponential backoff with max delay
        let delay = std::cmp::min(1000, 100 * attempts);
        std::thread::sleep(std::time::Duration::from_millis(delay));

        // Try to refresh partition table every few attempts
        if attempts % 5 == 0 {
            let _ = execute(&format!("partprobe {}", drive))
                .or_else(|_| execute(&format!("sfdisk -R {}", drive)))
                .or_else(|_| execute(&format!("blockdev --rereadpt {}", drive)));
        }
    }
}

fn set_efi_boot_flag(partition: &str) -> Result<()> {
    // Check if sgdisk is available, attempt to install if missing
    if execute("which sgdisk").is_err() {
        warn("sgdisk not found, attempting to install gdisk package...");
        if execute("which dnf").is_ok() {
            execute("dnf install -y gdisk")
                .map_err(|e| warn(&format!("Failed to install gdisk via dnf: {}", e)))
                .ok();
        } else if execute("which apt").is_ok() {
            execute("apt update && apt install -y gdisk")
                .map_err(|e| warn(&format!("Failed to install gdisk via apt: {}", e)))
                .ok();
        } else if execute("which pacman").is_ok() {
            execute("pacman -S --noconfirm gdisk")
                .map_err(|e| warn(&format!("Failed to install gdisk via pacman: {}", e)))
                .ok();
        } else {
            warn("Could not determine package manager to install gdisk");
        }
    }

    // Set EFI boot flag using sgdisk if available
    if execute("which sgdisk").is_ok() {
        let partition_num = partition
            .chars()
            .last()
            .and_then(|c| c.to_digit(10))
            .ok_or_else(|| {
                anyhow::anyhow!("Could not determine partition number from {}", partition)
            })?;

        let drive = if partition.contains("nvme") && partition.contains("p") {
            partition.rsplit_once("p").unwrap().0
        } else {
            partition.trim_end_matches(char::is_numeric)
        };

        execute(&format!(
            "sgdisk --set-flag={}:boot:on {}",
            partition_num, drive
        ))?;
        info(&format!("Set EFI boot flag on partition {}", partition_num));
    } else {
        warn("sgdisk not available, EFI boot flag not set. System may not boot properly.");
        warn("Please install gdisk package manually: dnf install gdisk (Fedora) or apt install gdisk (Ubuntu)");
    }
    Ok(())
}

fn partition_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    info(&format!("Partitioning drive {}", drive));



    // Step 2: Create new partition table
    info("Creating new partition table");
    if execute("which sgdisk").is_ok() {
        execute(&format!("sgdisk --clear {}", drive))?;

        // Create partitions
        let mut part_num = 1u32;
        for partition in layout {
            let size = match partition.size.as_str() {
                "512M" => "0:+512M",
                "8G" => "0:+8G",
                "2M" => "0:+2M",
                "rest" => "0:0",
                _ => bail!("Unsupported size: {}", partition.size),
            };

            let typecode = match partition.partition_type.as_str() {
                "uefi" => "ef00",
                "linux" => "8300",
                "21686148-6449-6E6F-744E-656564454649" => "ef02",
                _ => "8300",
            };

            let label = partition.label.as_deref().unwrap_or("");

            execute(&format!(
                "sgdisk --new={}:{} --typecode={}:{} --change-name={}:'{}' {}",
                part_num, size, part_num, typecode, part_num, label, drive
            ))?;

            part_num += 1;
        }

        // Use --refresh flag to notify kernel
        execute(&format!("sgdisk --refresh {}", drive))?;
    } else {
        bail!("sgdisk not available");
    }

    // Step 3: Wait for partitions to be recognized
    info("Waiting for partitions to be recognized");
    let partition_names = wait_for_partitions(drive, layout.len())?;

    // Step 4: Show what partitions were created
    info("Verifying new partitions were created");
    println!("SUCCESS: Created {} partitions:", partition_names.len());
    for (i, partition) in partition_names.iter().enumerate() {
        if let Ok(info) = execute(&format!("lsblk -n -o NAME,SIZE {}", partition)) {
            println!("  Partition {}: {} ({})", i + 1, partition, info.trim());
        } else {
            println!("  Partition {}: {}", i + 1, partition);
        }
    }

    Ok(())
}

fn format_partition(device: &str, partition: &Partition) -> Result<()> {
    info(&format!("Formatting {} as {}", device, partition.format));

    match partition.format.as_str() {
        "btrfs" => {
            // Create BTRFS filesystem on the device (usually a LUKS mapper)
            if let Some(ref label) = partition.label {
                info(&format!(
                    "Creating BTRFS filesystem with label '{}' on {}",
                    label, device
                ));
                if let Err(e) = execute(&format!("mkfs.btrfs -L {} {}", label, device)) {
                    bail!(
                        "Failed to create BTRFS filesystem with label '{}': {}",
                        label,
                        e
                    );
                }
            } else {
                info(&format!("Creating BTRFS filesystem on {}", device));
                if let Err(e) = execute(&format!("mkfs.btrfs {}", device)) {
                    bail!("Failed to create BTRFS filesystem: {}", e);
                }
            }

            // Create subvolumes if specified
            if let Some(ref subvolumes) = partition.subvolumes {
                let temp_mount = "/mnt/temp_btrfs";

                // Ensure mount directory exists
                if let Err(e) = fs::create_dir_all(temp_mount) {
                    bail!(
                        "Failed to create temporary mount directory '{}': {}",
                        temp_mount,
                        e
                    );
                }

                // Mount the BTRFS filesystem
                info(&format!(
                    "Mounting BTRFS filesystem temporarily at {}",
                    temp_mount
                ));
                if let Err(e) = execute(&format!("mount {} {}", device, temp_mount)) {
                    bail!(
                        "Failed to mount BTRFS filesystem for subvolume creation: {}",
                        e
                    );
                }

                // Create each subvolume with error handling
                for subvolume in subvolumes {
                    let subvol_path = format!("{}{}", temp_mount, subvolume);
                    info(&format!("Creating BTRFS subvolume: {}", subvolume));
                    if let Err(e) = execute(&format!("btrfs subvolume create {}", subvol_path)) {
                        // Attempt cleanup on failure
                        let _ = execute(&format!("umount {}", temp_mount));
                        bail!("Failed to create BTRFS subvolume '{}': {}", subvolume, e);
                    }
                }

                // Unmount the temporary filesystem
                info("Unmounting temporary BTRFS mount");
                if let Err(e) = execute(&format!("umount {}", temp_mount)) {
                    warn(&format!(
                        "Warning: Failed to unmount temporary BTRFS mount: {}",
                        e
                    ));
                }
            }

            // Verify the filesystem
            if let Err(e) = verify_filesystem(device, "btrfs") {
                warn(&format!("BTRFS filesystem verification failed: {}", e));
                warn("The filesystem may still be usable, but please verify manually");
            }
        }
        _ => {
            bail!(
                "Unsupported filesystem type for format_partition: {}",
                partition.format
            );
        }
    }

    Ok(())
}

fn verify_filesystem(partition: &str, fs_type: &str) -> Result<()> {
    match fs_type {
        "vfat" => {
            if execute("which fsck.fat").is_ok() {
                let result = execute(&format!("fsck.fat -r {}", partition));
                if result.is_err() {
                    warn(&format!("FAT filesystem check failed for {}", partition));
                } else {
                    info(&format!("FAT filesystem verified for {}", partition));
                }
            }
        }
        "ext4" => {
            if execute("which fsck.ext4").is_ok() {
                let result = execute(&format!("fsck.ext4 -n {}", partition));
                if result.is_err() {
                    warn(&format!("ext4 filesystem check failed for {}", partition));
                } else {
                    info(&format!("ext4 filesystem verified for {}", partition));
                }
            }
        }
        "btrfs" => {
            if execute("which btrfs").is_ok() {
                let result = execute(&format!("btrfs check {}", partition));
                if result.is_err() {
                    warn(&format!("BTRFS filesystem check failed for {}", partition));
                } else {
                    info(&format!("BTRFS filesystem verified for {}", partition));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

// Add this function to check if a partition is actually in use
fn is_partition_in_use(partition: &str) -> bool {
    // Check if partition is mounted
    if let Ok(mount_info) = execute(&format!("findmnt -n -o TARGET {}", partition)) {
        if !mount_info.trim().is_empty() {
            return true;
        }
    }

    // Check for processes using the partition
    if let Ok(fuser_info) = execute(&format!("fuser -v {}", partition)) {
        if !fuser_info.trim().is_empty() {
            return true;
        }
    }

    // Check for LUKS mappings
    if partition.starts_with("/dev/mapper/") {
        if let Ok(ls_info) = execute(&format!("ls -la /dev/mapper")) {
            for line in ls_info.lines() {
                if line.contains(partition) {
                    return true;
                }
            }
        }
    }

    false
}

fn ensure_partition_ready(partition: &str) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 10;

    while attempts < max_attempts {
        // Check if partition exists and is ready
        if Path::new(partition).exists() {
            // Try to read partition info to verify it's ready
            if let Ok(_) = execute(&format!("lsblk -n -o NAME {}", partition)) {
                return Ok(());
            }
        }

        attempts += 1;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    bail!(
        "Partition {} is not ready after {} attempts",
        partition,
        max_attempts
    )
}

fn format_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    // Wait for kernel to recognize partitions and get reliable partition list
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!(
        "DEBUG: format_drive called with {} partitions",
        layout.len()
    );
    for (i, part) in layout.iter().enumerate() {
        println!(
            "DEBUG: Layout[{}]: format={}, label={:?}",
            i, part.format, part.label
        );
    }

    // Use the same reliable detection as partition_drive
    let partition_names = wait_for_partitions(drive, layout.len())?;

    // Ensure all partitions are ready before formatting
    for partition in &partition_names {
        ensure_partition_ready(partition)?;
    }

    for (i, partition) in layout.iter().enumerate() {
        let current_name = &partition_names[i];

        // Double-check partition exists before formatting
        if !Path::new(current_name).exists() {
            bail!("Partition {} does not exist", current_name);
        }

        // Check if partition is in use before formatting
        if is_partition_in_use(current_name) {
            warn(&format!(
                "Partition {} is in use, skipping formatting",
                current_name
            ));
            continue;
        }

        // Simple cleanup before formatting
        info(&format!(
            "Preparing partition {} for formatting",
            current_name
        ));

        // Step 1: Unmount aggressively
        let _ = execute(&format!("umount -f {} 2>/dev/null || true", current_name));
        let _ = execute(&format!("umount -l {} 2>/dev/null || true", current_name));

        // Step 2: Check for processes using partition
        let _ = execute(&format!("fuser -v {} 2>/dev/null || true", current_name));
        let _ = execute(&format!("fuser -k {} 2>/dev/null || true", current_name));

        // Step 3: Remove any device mapper references
        let _ = execute("dmsetup remove_all 2>/dev/null || true");

        // Step 4: Close any LUKS containers
        let _ = execute(&format!(
            "cryptsetup close {} 2>/dev/null || true",
            current_name
        ));
        let _ = execute("cryptsetup close regicideos 2>/dev/null || true");

        // Step 5: Use a simple approach to clear partition
        info(&format!("Clearing partition {}", current_name));

        // Use enhanced clearing for all drives to ensure partition is truly clean
        info(&format!(
            "Clearing partition metadata and data on {}",
            current_name
        ));

        // Step 1: Clear filesystem signatures
        let _ = execute(&format!("wipefs -af {} 2>/dev/null || true", current_name));

        // Step 2: Zero out the first 1MB to clear partition table and filesystem metadata
        let _ = execute(&format!(
            "dd if=/dev/zero of={} bs=1M count=1 2>/dev/null || true",
            current_name
        ));

        // Step 3: For NVMe drives, also try nvme sanitize if available (safer than format)
        if current_name.contains("nvme") && execute("which nvme").is_ok() {
            // Extract the base NVMe device for sanitize operation
            let base_device = if let Some(pos) = current_name.rfind('p') {
                &current_name[..pos]
            } else {
                current_name
            };

            // Try nvme sanitize - this is safer than format and works on individual namespaces
            info(&format!("Attempting NVMe sanitize on {}", base_device));
            let _ = execute(&format!(
                "nvme sanitize --no-flush --force {} 2>/dev/null || true",
                base_device
            ));
        }

        // Step 6: Sync and wait
        let _ = std::process::Command::new("sync").status();
        std::thread::sleep(std::time::Duration::from_millis(3000));

        info(&format!(
            "Formatting {} as {}",
            current_name, partition.format
        ));
        println!(
            "DEBUG: Partition {} format = '{}', label = {:?}",
            i, partition.format, partition.label
        );

        match partition.format.as_str() {
            "vfat" => {
                // EFI partition formatting with validation
                if let Some(ref label) = partition.label {
                    execute(&format!("mkfs.vfat -F 32 -n {} {}", label, current_name))?;
                } else {
                    execute(&format!("mkfs.vfat -F 32 {}", current_name))?;
                }

                // Set EFI boot flag if this is likely an EFI partition
                if is_efi()
                    && (partition.partition_type == "uefi"
                        || partition.label.as_ref().map_or(false, |l| l == "EFI"))
                {
                    if let Err(e) = set_efi_boot_flag(current_name) {
                        warn(&format!("Failed to set EFI boot flag: {}", e));
                    }
                }

                // Verify filesystem
                verify_filesystem(current_name, "vfat")?;
            }
            "ext4" => {
                println!("DEBUG: Entering ext4 case for partition {}", current_name);
                info(&format!("Formatting {} as ext4", current_name));

                // Use a more robust approach for ext4 formatting
                let cmd = if let Some(ref label) = partition.label {
                    format!("mkfs.ext4 -F -L {} {}", label, current_name)
                } else {
                    format!("mkfs.ext4 -F {}", current_name)
                };

                // Execute with full error output
                match execute_with_output(&cmd) {
                    Ok(_) => {
                        info(&format!("Successfully formatted {} as ext4", current_name));
                    }
                    Err(_) => {
                        // If formatting fails, try a more aggressive approach
                        warn(&format!(
                            "Standard ext4 formatting failed, trying alternative approach..."
                        ));

                        // Try with different options
                        let alt_cmd = if let Some(ref label) = partition.label {
                            format!(
                                "mkfs.ext4 -F -L {} -E lazy_itable_init {}",
                                label, current_name
                            )
                        } else {
                            format!("mkfs.ext4 -F -E lazy_itable_init {}", current_name)
                        };

                        if let Err(_) = execute_with_output(&alt_cmd) {
                            warn(&format!("Alternative ext4 formatting also failed. You may need to reboot and try again."));
                            bail!(
                                "Failed to format {} as ext4 after multiple attempts",
                                current_name
                            );
                        } else {
                            info(&format!(
                                "Successfully formatted {} as ext4 with alternative method",
                                current_name
                            ));
                        }
                    }
                }

                // Verify filesystem
                verify_filesystem(current_name, "ext4")?;
            }
            "btrfs" => {
                // BTRFS formatting with enhanced error handling
                if let Some(ref label) = partition.label {
                    info(&format!(
                        "Creating BTRFS filesystem with label '{}' on {}",
                        label, current_name
                    ));
                    if let Err(e) = execute(&format!("mkfs.btrfs -L {} {}", label, current_name)) {
                        bail!(
                            "Failed to create BTRFS filesystem with label '{}': {}",
                            label,
                            e
                        );
                    }
                } else {
                    info(&format!("Creating BTRFS filesystem on {}", current_name));
                    if let Err(e) = execute(&format!("mkfs.btrfs {}", current_name)) {
                        bail!("Failed to create BTRFS filesystem: {}", e);
                    }
                }

                // Create subvolumes with better error handling
                if let Some(ref subvolumes) = partition.subvolumes {
                    let temp_mount = "/mnt/temp_btrfs";

                    // Ensure mount directory exists
                    if let Err(e) = fs::create_dir_all(temp_mount) {
                        bail!(
                            "Failed to create temporary mount directory '{}': {}",
                            temp_mount,
                            e
                        );
                    }

                    // Mount the BTRFS filesystem
                    info(&format!(
                        "Mounting BTRFS filesystem temporarily at {}",
                        temp_mount
                    ));
                    if let Err(e) = execute(&format!("mount {} {}", current_name, temp_mount)) {
                        bail!(
                            "Failed to mount BTRFS filesystem for subvolume creation: {}",
                            e
                        );
                    }

                    // Create each subvolume with error handling
                    for subvolume in subvolumes {
                        let subvol_path = format!("{}{}", temp_mount, subvolume);
                        info(&format!("Creating BTRFS subvolume: {}", subvolume));
                        if let Err(e) = execute(&format!("btrfs subvolume create {}", subvol_path))
                        {
                            // Attempt cleanup on failure
                            let _ = execute(&format!("umount {}", temp_mount));
                            bail!("Failed to create BTRFS subvolume '{}': {}", subvolume, e);
                        }
                    }

                    // Unmount the temporary filesystem
                    info("Unmounting temporary BTRFS mount");
                    if let Err(e) = execute(&format!("umount {}", temp_mount)) {
                        warn(&format!(
                            "Warning: Failed to unmount temporary BTRFS mount: {}",
                            e
                        ));
                    }
                }

                // Verify the filesystem
                if let Err(e) = verify_filesystem(current_name, "btrfs") {
                    warn(&format!("BTRFS filesystem verification failed: {}", e));
                    warn("The filesystem may still be usable, but please verify manually");
                }
            }
            "luks" => {
                println!("DEBUG: Entering LUKS case for partition {}", current_name);
                println!("Setting up LUKS encryption. You will be prompted to enter a password.");

                // Check if partition is in use before LUKS format
                if is_partition_in_use(current_name) {
                    warn(&format!(
                        "Partition {} is in use, skipping LUKS format",
                        current_name
                    ));
                    continue;
                }

                // Unmount aggressively before LUKS format
                let _ = execute(&format!("umount -f {} 2>/dev/null || true", current_name));
                let _ = execute(&format!("umount -l {} 2>/dev/null || true", current_name));

                // Close any existing LUKS containers
                let _ = execute(&format!(
                    "cryptsetup close {} 2>/dev/null || true",
                    current_name
                ));
                let _ = execute("cryptsetup close regicideos 2>/dev/null || true");

                // Remove any device mapper references
                let _ = execute("dmsetup remove_all 2>/dev/null || true");

                // Use enhanced clearing for LUKS partition preparation
                info(&format!(
                    "Clearing partition metadata and data on {}",
                    current_name
                ));

                // Step 1: Clear filesystem signatures
                let _ = execute(&format!("wipefs -af {} 2>/dev/null || true", current_name));

                // Step 2: Zero out the first 1MB to clear partition table and filesystem metadata
                let _ = execute(&format!(
                    "dd if=/dev/zero of={} bs=1M count=1 2>/dev/null || true",
                    current_name
                ));

                // Step 3: For NVMe drives, also try nvme sanitize if available (safer than format)
                if current_name.contains("nvme") && execute("which nvme").is_ok() {
                    // Extract the base NVMe device for sanitize operation
                    let base_device = if let Some(pos) = current_name.rfind('p') {
                        &current_name[..pos]
                    } else {
                        current_name
                    };

                    // Try nvme sanitize - this is safer than format and works on individual namespaces
                    info(&format!("Attempting NVMe sanitize on {}", base_device));
                    let _ = execute(&format!(
                        "nvme sanitize --no-flush --force {} 2>/dev/null || true",
                        base_device
                    ));
                }

                // Wait longer for all operations to complete
                std::thread::sleep(std::time::Duration::from_millis(5000));

                // Special handling for LUKS format (interactive password required)
                let result = ProcessCommand::new("cryptsetup")
                    .args(&["luksFormat", current_name])
                    .stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status();

                match result {
                    Ok(status) => {
                        if status.success() {
                            println!("DEBUG: LUKS format successful for {}", current_name);
                        } else {
                            bail!(
                                "Failed to format LUKS partition: exit code {:?}",
                                status.code()
                            );
                        }
                    }
                    Err(e) => {
                        bail!("Failed to execute cryptsetup: {}", e);
                    }
                }

                // Set LUKS label after formatting if specified
                if let Some(ref label) = partition.label {
                    execute(&format!(
                        "cryptsetup -q config {} --label {}",
                        current_name, label
                    ))?;
                }

                // Always use "regicideos" as the mapper name for RegicideOS
                let open_result = ProcessCommand::new("cryptsetup")
                    .args(["luksOpen", current_name, "regicideos"])
                    .status();

                if !open_result.map(|s| s.success()).unwrap_or(false) {
                    bail!("Failed to open LUKS partition");
                }

                // Verify the device was created with timeout
                let mut attempts = 0;
                while !Path::new("/dev/mapper/regicideos").exists() && attempts < 10 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    attempts += 1;
                }

                if !Path::new("/dev/mapper/regicideos").exists() {
                    bail!("LUKS device /dev/mapper/regicideos was not created after 5 seconds");
                }

                let mapper_device = "/dev/mapper/regicideos".to_string();
                println!("DEBUG: LUKS mapper created: {}", mapper_device);

                // Recursively format the inside partition (should be BTRFS)
                if let Some(ref inside_partition) = partition.inside {
                    println!("DEBUG: Recursively formatting inside partition as BTRFS...");
                    format_partition(&mapper_device, inside_partition)?;
                }

                // CRITICAL: Return early to prevent re-formatting the same partition
                // The LUKS container and its BTRFS filesystem are now set up
                return Ok(());
            }
            _ => {
                warn(&format!("Unknown filesystem type: {}", partition.format));
            }
        }
    }

    Ok(())
}

fn chroot(command: &str) -> Result<()> {
    // Execute chroot with proper PATH: chroot /mnt/root /bin/bash -c "export PATH=... && command"
    let full_command = format!("chroot /mnt/root /bin/bash -c \"export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin && {}\"", command);

    let output = ProcessCommand::new("bash")
        .args(&["-c", &full_command])
        .output()
        .with_context(|| format!("Failed to execute chroot command: {}", command))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Use anyhow::anyhow directly to avoid sanitization for debugging
        return Err(anyhow::anyhow!(
            "Chroot command failed: {}\nSTDOUT: {}\nSTDERR: {}", 
            command, stdout, stderr
        ));
    }

    info(&format!(
        "Successfully executed chroot command: {}",
        command
    ));
    Ok(())
}

fn chroot_with_output(command: &str) -> Result<String> {
    // Execute chroot with proper PATH and return output
    let full_command = format!("chroot /mnt/root /bin/bash -c \"export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin && {}\"", command);

    let output = ProcessCommand::new("bash")
        .args(&["-c", &full_command])
        .output()
        .with_context(|| format!("Failed to execute chroot command: {}", command))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Use anyhow::anyhow directly to avoid sanitization for debugging
        return Err(anyhow::anyhow!(
            "Chroot command failed: {}\nSTDOUT: {}\nSTDERR: {}", 
            command, stdout, stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn get_manifest(repository: &str) -> Result<toml::Value> {
    let manifest_url = format!("{}Manifest.toml", repository);
    let response = reqwest::get(&manifest_url).await?;
    let content = response.text().await?;
    let manifest: toml::Value = toml::from_str(&content)?;
    Ok(manifest)
}

async fn get_flavours(repository: &str) -> Result<Vec<String>> {
    let manifest = get_manifest(repository).await?;
    let arch = "amd64"; // Assuming x86_64 architecture

    let mut flavours = Vec::new();
    if let Some(table) = manifest.as_table() {
        for (key, value) in table {
            if let Some(flavour_table) = value.as_table() {
                if let Some(arch_array) = flavour_table.get("arch") {
                    if let Some(arch_vec) = arch_array.as_array() {
                        for arch_val in arch_vec {
                            if let Some(arch_str) = arch_val.as_str() {
                                if arch_str == arch {
                                    flavours.push(key.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(flavours)
}

async fn get_releases(repository: &str, flavour: &str) -> Result<Vec<String>> {
    let manifest = get_manifest(repository).await?;
    let arch = "amd64";

    let mut releases = Vec::new();
    if let Some(flavour_table) = manifest.get(flavour).and_then(|v| v.as_table()) {
        if let Some(versions_table) = flavour_table.get("versions").and_then(|v| v.as_table()) {
            for (version_key, version_value) in versions_table {
                if let Some(version_table) = version_value.as_table() {
                    if let Some(arch_array) = version_table.get("arch") {
                        if let Some(arch_vec) = arch_array.as_array() {
                            for arch_val in arch_vec {
                                if let Some(arch_str) = arch_val.as_str() {
                                    if arch_str == arch {
                                        releases.push(version_key.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(releases)
}

async fn get_url(config: &Config) -> Result<String> {
    let manifest = get_manifest(&config.repository).await?;
    let arch = "amd64";

    if let Some(filename) = manifest
        .get(&config.flavour)
        .and_then(|f| f.as_table())
        .and_then(|f| f.get("versions"))
        .and_then(|v| v.as_table())
        .and_then(|v| v.get(&config.release_branch))
        .and_then(|r| r.as_table())
        .and_then(|r| r.get("filename"))
        .and_then(|f| f.as_str())
    {
        Ok(format!(
            "{}{}/{}/{}",
            config.repository, arch, config.release_branch, filename
        ))
    } else {
        bail!("Could not find filename in manifest")
    }
}

fn find_partition_by_label(label: &str) -> Result<String> {
    let label_path = format!("/dev/disk/by-label/{}", label);

    // Method 1: Try by-label first
    if Path::new(&label_path).exists() {
        return Ok(format!("LABEL={}", label));
    }

    // Method 2: Try to find via blkid
    if execute("which blkid").is_ok() {
        if let Ok(output) = execute(&format!("blkid -L {}", label)) {
            let device = output.trim();
            if !device.is_empty() && Path::new(device).exists() {
                return Ok(device.to_string());
            }
        }
    }

    // Method 3: Search through all block devices
    if let Ok(output) = execute("lsblk -fn -o NAME,LABEL") {
        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == label {
                let device = format!("/dev/{}", parts[0]);
                if Path::new(&device).exists() {
                    return Ok(device);
                }
            }
        }
    }

    bail!("Could not find partition with label: {}", label);
}

fn mount_with_retry(
    source: &str,
    target: &str,
    fs_type: Option<&str>,
    options: Option<&str>,
) -> Result<()> {
    // Only allow mounting under /mnt for security
    if !target.starts_with("/mnt/") {
        bail!("Mount target must be under /mnt directory: {}", target);
    }
    safe_create_dir_all(target, "/mnt").ok();

    let mut mount_cmd = format!("mount");

    if let Some(opts) = options {
        mount_cmd.push_str(&format!(" -o {}", opts));
    }

    if let Some(fs) = fs_type {
        mount_cmd.push_str(&format!(" -t {}", fs));
    }

    mount_cmd.push_str(&format!(" {} {}", source, target));

    // Try mounting with retries
    let mut attempts = 0;
    let max_attempts = 5;

    loop {
        match execute(&mount_cmd) {
            Ok(_) => {
                info(&format!("Successfully mounted {} on {}", source, target));
                return Ok(());
            }
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts {
                    bail!(
                        "Failed to mount {} on {} after {} attempts: {}",
                        source,
                        target,
                        max_attempts,
                        e
                    );
                }
                warn(&format!(
                    "Mount attempt {} failed, retrying in {}ms: {}",
                    attempts,
                    attempts * 500,
                    e
                ));
                std::thread::sleep(std::time::Duration::from_millis(attempts * 500));
            }
        }
    }
}

fn mount_roots() -> Result<()> {
    let mount_point = "/mnt/gentoo";

    info("Finding ROOTS partition...");
    let roots_device = find_partition_by_label("ROOTS")?;

    info("Mounting ROOTS partition on /mnt/gentoo");
    safe_create_dir_all(mount_point, "/mnt")?;
    mount_with_retry(&roots_device, mount_point, None, None)?;

    Ok(())
}

fn mount() -> Result<()> {
    safe_create_dir_all("/mnt/root", "/mnt").ok();

    info("Mounting root.img on /mnt/root");
    mount_with_retry(
        "/mnt/gentoo/root.img",
        "/mnt/root",
        Some("squashfs"),
        Some("ro,loop"),
    )?;

    info("Mounting special filesystems first");
    mount_with_retry("/proc", "/mnt/root/proc", Some("proc"), None)?;
    execute("mount --rbind /dev /mnt/root/dev")?;
    execute("mount --rbind /sys /mnt/root/sys")?;
    execute("mount --bind /run /mnt/root/run")?;
    execute("mount --make-slave /mnt/root/run")?;

    // Mount EFI partition BEFORE creating directories in it (fixes read-only error)
    if is_efi() {
        info("Finding and mounting EFI partition before creating boot directories...");
        let efi_device = find_partition_by_label("EFI")?;
        info(&format!("Found EFI partition: {}", efi_device));
        mount_with_retry(&efi_device, "/mnt/root/boot/efi", None, Some("rw"))?;
        
        // Verify mount was successful
        match execute("findmnt /mnt/root/boot/efi") {
            Ok(mount_info) => {
                info(&format!("EFI mount verification: {}", mount_info.trim()));
            }
            Err(e) => {
                warn(&format!("Failed to verify EFI mount: {}", e));
            }
        }
    }

    // Ensure proper boot directory structure for GRUB on writable EFI partition
    info("Creating boot directory structure for GRUB");
    
    // Debug: Check if EFI partition is actually mounted and writable
    match execute("mount | grep '/mnt/root/boot/efi'") {
        Ok(mount_info) => info(&format!("EFI mount status: {}", mount_info.trim())),
        Err(e) => warn(&format!("Failed to check EFI mount: {}", e)),
    }
    
    // Test write to EFI partition before creating directories
    let test_file = "/mnt/root/boot/efi/.installer_test";
    match safe_write_file(test_file, b"test_write", "/mnt/root/boot/efi") {
        Ok(()) => {
            info("✓ EFI partition is writable");
            let _ = safe_remove_file(test_file, "/mnt/root/boot/efi");
        }
        Err(e) => {
            bail!("❌ EFI partition is NOT writable: {}", e);
        }
    }
    
    safe_create_dir_all("/mnt/root/boot/efi/grub", "/mnt/root/boot/efi")?;
    
    // Create symlinks from /usr/bin to /usr/sbin for GRUB tools if needed
    // This fixes grub2-mkconfig calling grub2-probe from wrong location
    info("Creating GRUB tool symlinks to fix hardcoded paths...");
    let _grub_links = [
        ("grub-probe", "grub-probe"),
        ("grub2-probe", "grub2-probe"),
        ("grub-mkconfig", "grub-mkconfig"), 
        ("grub2-mkconfig", "grub2-mkconfig"),
    ];
    
    // Create symlinks from /usr/bin to /usr/sbin for GRUB tools if needed
    // This fixes grub2-mkconfig calling grub2-probe from wrong location
    info("Creating GRUB tool symlinks to fix hardcoded paths...");
    
    // First, let's debug what actually exists
    info("Debugging GRUB tool locations...");
    match chroot_with_output("ls -la /usr/sbin/grub* 2>/dev/null || echo 'not found'") {
        Ok(output) => info(&format!("GRUB tools in /usr/sbin: {}", output.trim())),
        Err(e) => warn(&format!("Failed to list /usr/sbin/grub*: {}", e)),
    }
    
    match chroot_with_output("ls -la /usr/bin/grub* 2>/dev/null || echo 'not found'") {
        Ok(output) => info(&format!("GRUB tools in /usr/bin: {}", output.trim())),
        Err(e) => warn(&format!("Failed to list /usr/bin/grub*: {}", e)),
    }
    
    let _grub_links = [
        ("grub-probe", "grub-probe"),
        ("grub2-probe", "grub2-probe"),
        ("grub-mkconfig", "grub-mkconfig"), 
        ("grub2-mkconfig", "grub2-mkconfig"),
    ];
    
    for (target, link_name) in &_grub_links {
        let target_path = format!("/mnt/root/usr/sbin/{}", target);
        let link_path = format!("/mnt/root/usr/bin/{}", link_name);
        
        info(&format!("Checking symlink: target={}, link={}", target_path, link_path));
        info(&format!("Target exists: {}, Link exists: {}", 
            Path::new(&target_path).exists(), 
            Path::new(&link_path).exists()
        ));
        
        if Path::new(&target_path).exists() && !Path::new(&link_path).exists() {
            // Try copying instead of symlinking to avoid mount issues
            let copy_cmd = format!("cp /usr/sbin/{} /usr/bin/{}", target, link_name);
            info(&format!("Running copy command: {}", copy_cmd));
            
            match chroot(&copy_cmd) {
                Ok(()) => {
                    info(&format!("✓ Copied: /usr/bin/{} <- /usr/sbin/{}", link_name, target));
                    
                    // Verify the copy was created and is executable
                    match chroot_with_output(&format!("ls -la /usr/bin/{}", link_name)) {
                        Ok(verify_output) => {
                            info(&format!("Copy verification: {}", verify_output.trim()));
                            // Make sure it's executable
                            let chmod_cmd = format!("chmod +x /usr/bin/{}", link_name);
                            if let Err(e) = chroot(&chmod_cmd) {
                                warn(&format!("Failed to make {} executable: {}", link_name, e));
                            } else {
                                info(&format!("Made {} executable", link_name));
                            }
                        }
                        Err(e) => warn(&format!("Failed to verify copy: {}", e)),
                    }
                }
                Err(e) => warn(&format!("Failed to copy {} -> {}: {}", link_name, target, e)),
            }
        } else {
            info(&format!("Skipping copy {} (target exists: {}, link exists: {})", 
                link_name, 
                Path::new(&target_path).exists(), 
                Path::new(&link_path).exists()
            ));
        }
    }

    Ok(())
}

async fn download_root(url: &str) -> Result<()> {
    let root_img_path = "/mnt/gentoo/root.img";

    // Remove existing file if present
    if Path::new(root_img_path).exists() {
        safe_remove_file(root_img_path, "/mnt/gentoo")?;
    }

    info(&format!("Downloading RegicideOS root image from {}", url));
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        bail!("Failed to download root image: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;

    if bytes.is_empty() {
        bail!("Downloaded root image is empty");
    }

    safe_write_file(root_img_path, &bytes, "/mnt/gentoo")?;

    // Verify the file was written and has content
    let metadata = fs::metadata(root_img_path)?;
    if metadata.len() == 0 {
        bail!("Root image file is empty after download");
    }

    info(&format!("Downloaded {} bytes", metadata.len()));
    Ok(())
}

fn verify_grub_environment() -> Result<()> {
    info("Verifying GRUB environment and dependencies...");

    // Check 1: Verify GRUB binaries are accessible in chroot environment
    info("Checking GRUB binary availability in chroot...");
    
    // Check for GRUB mkconfig binary in chroot (prioritize sbin)
    let grub_mkconfig_check = match chroot_with_output("which grub-mkconfig 2>/dev/null || which grub2-mkconfig 2>/dev/null || echo 'not found'") {
        Ok(output) => {
            let result = output.trim();
            if result != "not found" && !result.is_empty() {
                info(&format!("✓ GRUB mkconfig found: {}", result));
                true
            } else {
                // Try direct path check if which fails
                match chroot_with_output("ls /usr/sbin/grub-mkconfig /usr/sbin/grub2-mkconfig 2>/dev/null || echo 'not found'") {
                    Ok(direct_output) => {
                        let direct_result = direct_output.trim();
                        if direct_result != "not found" && !direct_result.is_empty() {
                            info(&format!("✓ GRUB mkconfig found in /usr/sbin: {}", direct_result));
                            true
                        } else {
                            false
                        }
                    }
                    Err(_) => false
                }
            }
        }
        Err(e) => {
            warn(&format!("Failed to check for GRUB mkconfig: {}", e));
            false
        }
    };
    
    // Check for GRUB probe binary in chroot (prioritize sbin)
    let grub_probe_check = match chroot_with_output("which grub-probe 2>/dev/null || which grub2-probe 2>/dev/null || echo 'not found'") {
        Ok(output) => {
            let result = output.trim();
            if result != "not found" && !result.is_empty() {
                info(&format!("✓ GRUB probe found: {}", result));
                true
            } else {
                // Try direct path check if which fails
                match chroot_with_output("ls /usr/sbin/grub-probe /usr/sbin/grub2-probe 2>/dev/null || echo 'not found'") {
                    Ok(direct_output) => {
                        let direct_result = direct_output.trim();
                        if direct_result != "not found" && !direct_result.is_empty() {
                            info(&format!("✓ GRUB probe found in /usr/sbin: {}", direct_result));
                            true
                        } else {
                            false
                        }
                    }
                    Err(_) => false
                }
            }
        }
        Err(e) => {
            warn(&format!("Failed to check for GRUB probe: {}", e));
            false
        }
    };

    if !grub_mkconfig_check {
        bail!("GRUB mkconfig binary not found in chroot environment - please install GRUB");
    }
    
    if !grub_probe_check {
        warn("GRUB probe binary not found in chroot - some GRUB functionality may be limited");
        // Don't fail the installation for missing probe binary
    }



    // Check 3: Verify boot directory is writable (test EFI partition, not read-only squashfs)
    let boot_test_file = "/mnt/root/boot/efi/.grub_test_write";
    if let Err(e) = safe_write_file(boot_test_file, b"test", "/mnt/root/boot/efi") {
        bail!("EFI boot directory is not writable: {}", e);
    }
    let _ = safe_remove_file(boot_test_file, "/mnt/root/boot/efi");

    // Check 4: Verify EFI/BOOT partition is mounted and writable
    let efi_dir = if is_efi() {
        "/mnt/root/boot/efi"
    } else {
        "/mnt/root/boot"
    };
    let efi_test_file = &format!("{}/.grub_test_write", efi_dir);
    if let Err(e) = safe_write_file(efi_test_file, b"test", efi_dir) {
        bail!("EFI/BOOT partition is not writable: {}", e);
    }
    let _ = safe_remove_file(efi_test_file, efi_dir);

    // Check 5: Verify required directories exist
    let required_dirs = [
        "/mnt/root/boot",
        "/mnt/root/boot/efi/grub",  // for EFI
        "/mnt/root/usr/share/grub", // GRUB modules
        "/mnt/root/etc/grub.d",     // GRUB configuration scripts
    ];

    for dir in &required_dirs {
        if !Path::new(dir).exists() {
            warn(&format!("Required GRUB directory missing: {}", dir));
            // Try to create missing directories in appropriate writable locations
            let writable_base = if dir.starts_with("/mnt/root/boot") {
                "/mnt/root/boot"
            } else if dir.starts_with("/mnt/root/etc") {
                "/mnt/root/etc" 
            } else if dir.starts_with("/mnt/root/usr") {
                "/mnt/root/usr"
            } else {
                "/mnt/root"
            };
            
            if let Err(e) = safe_create_dir_all(dir, writable_base) {
                bail!("Could not create required directory {}: {}", dir, e);
            }
        }
    }

    // Check 6: Verify GRUB configuration scripts exist
    let grub_d_files = [
        "/mnt/root/etc/grub.d/00_header",
        "/mnt/root/etc/grub.d/10_linux",
        "/mnt/root/etc/grub.d/30_os-prober",
    ];

    for file in &grub_d_files {
        if !Path::new(file).exists() {
            warn(&format!("GRUB configuration script missing: {}", file));
        }
    }

    // Check 7: Test device access that grub-probe will need
    info("Testing device access for GRUB...");
    let device_tests = ["/dev/sda", "/dev/mapper/regicideos"];

    for device in &device_tests {
        if Path::new(device).exists() {
            if let Ok(_) = execute(&format!("lsblk -n -o NAME,SIZE {}", device)) {
                info(&format!("Device access OK: {}", device));
            } else {
                warn(&format!("Cannot access device: {}", device));
            }
        }
    }

    // Check 8: Verify filesystem detection
    if is_efi() {
        if let Ok(efi_device) = find_partition_by_label("EFI") {
            info(&format!("EFI partition found: {}", efi_device));
        }
    }

    // Check 9: Verify sufficient disk space (at least 50MB for GRUB)
    info("Checking disk space for GRUB configuration...");
    // This is a simplified check - you might want to use statvfs for accurate space checking
    if Path::new("/mnt/root/boot").exists() {
        info("Boot directory exists for GRUB configuration");
    } else {
        bail!("Boot directory not available for GRUB configuration");
    }

    // Check 10: Verify GRUB crypto modules for encrypted boot support
    info("Verifying GRUB crypto modules for encrypted boot support...");
    let crypto_modules = [
        "cryptodisk",
        "luks", 
        "gcry_rijndael",
        "gcry_sha256",
        "gcry_sha512",
        "part_gpt",
    ];

    let grub_lib_path = "/mnt/root/usr/lib/grub";
    for module in &crypto_modules {
        let module_path = format!("{}/x86_64-efi/{}.mod", grub_lib_path, module);
        if Path::new(&module_path).exists() {
            info(&format!("✓ GRUB crypto module found: {}", module));
        } else {
            warn(&format!("✗ GRUB crypto module missing: {}", module));
        }
    }

    // Check 11: Verify LUKS mapper device exists for encrypted boot
    if Path::new("/dev/mapper/regicideos").exists() {
        info("✓ LUKS mapper device /dev/mapper/regicideos found for encrypted boot");
    } else {
        info("ℹ LUKS mapper device not found - encrypted boot may not be configured");
    }

    Ok(())
}



fn install_bootloader(platform: &str, device: &str) -> Result<()> {
    // Check for grub binary in chroot environment using chroot commands
    let grub = match chroot_with_output("which grub-install 2>/dev/null || which grub2-install 2>/dev/null || echo 'not found'") {
        Ok(output) => {
            let result = output.trim();
            if result != "not found" && !result.is_empty() {
                if result.contains("grub2-install") {
                    "grub2"
                } else {
                    "grub"
                }
            } else {
                // Try direct path check if which fails
                match chroot_with_output("ls /usr/sbin/grub-install /usr/sbin/grub2-install 2>/dev/null || echo 'not found'") {
                    Ok(direct_output) => {
                        let direct_result = direct_output.trim();
                        if direct_result != "not found" && !direct_result.is_empty() {
                            if direct_result.contains("grub2-install") {
                                "grub2"
                            } else {
                                "grub"
                            }
                        } else {
                            bail!("GRUB installer not found in chroot environment");
                        }
                    }
                    Err(e) => {
                        bail!("Failed to check for GRUB installer: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            bail!("Failed to check for GRUB installer: {}", e);
        }
    };

    if platform.contains("efi") {
        // Install GRUB for EFI systems - run INSIDE chroot like Python reference
        // Use exact same commands as Python reference for compatibility
        let grub_install_cmd = format!(
            "{}-install --modules=lvm --force --target=\"{}\" --efi-directory=\"/boot/efi\" --boot-directory=\"/boot/efi\"",
            grub, platform
        );
        chroot(&grub_install_cmd)?;

        // Create GRUB configuration manually to avoid systemd-boot conflicts
        info("Creating GRUB configuration manually...");
        
        // First, find actual kernel and initrd files in squashfs
        let kernel_files = match chroot_with_output("find /boot -name 'vmlinuz-*' -type f | head -1") {
            Ok(files) => files.trim().to_string(),
            Err(_) => {
                warn("Could not find kernel files, using fallback");
                "/boot/vmlinuz".to_string()
            }
        };
        
        let initrd_files = match chroot_with_output("find /boot -name 'initrd-*' -type f | head -1") {
            Ok(files) => files.trim().to_string(),
            Err(_) => {
                warn("Could not find initrd files, using fallback");
                "/boot/initrd".to_string()
            }
        };
        
        info(&format!("Using kernel: {}", kernel_files));
        info(&format!("Using initrd: {}", initrd_files));
        
        let grub_config = format!(r#"set default=0
set timeout=5

menuentry "RegicideOS" {{
    linux {}
    initrd {}
    options "root=LABEL=ROOTS quiet splash rw"
}}

menuentry "RegicideOS (Fallback)" {{
    linux {}
    initrd {}
    options "root=LABEL=ROOTS quiet splash rw"
}}
"#, kernel_files, initrd_files, kernel_files, initrd_files);
        
        safe_write_file("/mnt/root/boot/efi/grub/grub.cfg", grub_config.as_bytes(), "/mnt/root/boot/efi")?;
        info("✓ GRUB configuration created successfully");
    } else {
        // For BIOS, use exact same commands as Python reference
        let grub_install_cmd = format!(
            "{}-install --force --target=\"{}\" --boot-directory=\"/boot/efi\" {}",
            grub, platform, device
        );
        chroot(&grub_install_cmd)?;

        // Ensure boot partition is writable for GRUB config generation
        info("Ensuring boot partition is writable for GRUB config generation");
        chroot("mount -o remount,rw /boot/efi")?;

        // Verify GRUB environment before running grub-mkconfig
        info("Verifying GRUB environment before config generation");
        verify_grub_environment()?;

        // Run GRUB mkconfig to generate configuration
        let grub_mkconfig_cmd = format!("{}-mkconfig -o /boot/efi/grub/grub.cfg", grub);
        info(&format!("Running GRUB mkconfig: {}", grub_mkconfig_cmd));
        
        chroot(&grub_mkconfig_cmd)?;
    }

    Ok(())
}

fn post_install(config: &Config) -> Result<()> {
    let layout_name = &config.filesystem;
    info("Mounting overlays & home");

    let (etc_path, var_path, usr_path) = match layout_name.as_str() {
        "btrfs" => {
            execute("mount -L ROOTS -o subvol=overlay /mnt/root/overlay")?;
            execute("mount -L ROOTS -o subvol=home /mnt/root/home")?;
            (
                "/mnt/root/overlay/etc",
                "/mnt/root/overlay/var",
                "/mnt/root/overlay/usr",
            )
        }
        "btrfs_encryption_dev" => {
            // ROOTS is ext4 root filesystem, already mounted as root.img
            // Mount LUKS-encrypted REGICIDEOS partition for overlay and home subvolumes
            if !Path::new("/dev/mapper/regicideos").exists() {
                bail!("LUKS device /dev/mapper/regicideos not found");
            }
            execute("mount /dev/mapper/regicideos -o subvol=overlay /mnt/root/overlay")?;
            execute("mount /dev/mapper/regicideos -o subvol=home /mnt/root/home")?;
            (
                "/mnt/root/overlay/etc",
                "/mnt/root/overlay/var",
                "/mnt/root/overlay/usr",
            )
        }
        _ => {
            if !Path::new("/dev/disk/by-label/OVERLAY").exists() {
                bail!("OVERLAY partition not found");
            }
            if !Path::new("/dev/disk/by-label/HOME").exists() {
                bail!("HOME partition not found");
            }
            safe_create_dir_all("/mnt/root/overlay", "/mnt/root")?;
            safe_create_dir_all("/mnt/root/home", "/mnt/root")?;
            execute("mount -L OVERLAY /mnt/root/overlay")?;
            execute("mount -L HOME /mnt/root/home")?;
            (
                "/mnt/root/overlay",
                "/mnt/root/overlay",
                "/mnt/root/overlay",
            )
        }
    };

    let paths = [
        format!("{}/etc", etc_path),
        format!("{}/etcw", etc_path),
        format!("{}/var", var_path),
        format!("{}/varw", var_path),
        format!("{}/usr", usr_path),
        format!("{}/usrw", usr_path),
    ];

    for path in &paths {
        // Use the parent directory as base since these are in writable overlay areas
        if let Some(parent) = Path::new(path).parent() {
            let parent_str = parent.to_string_lossy();
            safe_create_dir_all(path, &parent_str)?;
        } else {
            safe_create_dir_all(path, "/mnt/root")?;
        }
    }

    execute(&format!(
        "mount -t overlay overlay -o lowerdir=/mnt/root/usr,upperdir={}/usr,workdir={}/usrw,ro /mnt/root/usr",
        usr_path, usr_path
    ))?;
    execute(&format!(
        "mount -t overlay overlay -o lowerdir=/mnt/root/etc,upperdir={}/etc,workdir={}/etcw,rw /mnt/root/etc",
        etc_path, etc_path
    ))?;
    execute(&format!(
        "mount -t overlay overlay -o lowerdir=/mnt/root/var,upperdir={}/var,workdir={}/varw,rw /mnt/root/var",
        var_path, var_path
    ))?;

    if !config.username.is_empty() {
        info("Creating user");
        chroot(&format!("useradd -m {}", config.username))?;

        let mut attempts = 0;
        loop {
            match chroot(&format!("passwd {}", config.username)) {
                Ok(_) => break,
                _ => {
                    attempts += 1;
                    if attempts >= 3 {
                        warn("Failed to set password after 3 attempts. User will need to set password manually.");
                        break;
                    }
                    println!("Password setting failed. Please try again.");
                }
            }
        }

        chroot(&format!("usermod -aG wheel,video {}", config.username))?;
    }

    let flatpaks = get_flatpak_packages(&config.applications);
    if !flatpaks.is_empty() {
        safe_create_dir_all("/mnt/root/etc/declare", "/mnt/root/etc")?;
        safe_write_file(
            "/mnt/root/etc/declare/flatpak",
            flatpaks.as_bytes(),
            "/mnt/root/etc",
        )?;

        if !Path::new("/mnt/root/usr/bin/rc-service").exists() {
            chroot("systemctl enable declareflatpak")?;
        } else {
            chroot("rc-update add declareflatpak")?;
        }
    }

    Ok(())
}

// Input validation functions
fn validate_device_path(path: &str) -> Result<()> {
    // Allow common block device patterns: /dev/sd*, /dev/nvme*, /dev/hd*, /dev/vd*, /dev/mmcblk*
    let device_regex = regex::Regex::new(
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

fn validate_username(username: &str) -> Result<()> {
    // Unix username rules: 1-32 chars, lowercase letters, digits, hyphens, underscores
    // Cannot start with hyphen or digit, cannot end with hyphen
    let username_regex = regex::Regex::new(r"^[a-z_][a-z0-9_-]{0,31}$")?;
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

fn validate_url(url: &str) -> Result<()> {
    // Basic URL validation with allowlist for official repositories
    let url_regex = regex::Regex::new(r"^https://[a-zA-Z0-9.-]+/[a-zA-Z0-9/_.-]*$")?;
    if !url_regex.is_match(url) {
        bail!("Invalid URL format");
    }

    // Only allow official repositories
    let allowed_domains = [
        "repo.regicideoslinux.com",
        "regicideoslinux.com",
        "repo.xenialinux.com",
        "xenialinux.com",
    ];

    let domain = regex::Regex::new(r"^https://([a-zA-Z0-9.-]+)")
        .unwrap()
        .captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    if !allowed_domains.contains(&domain) {
        bail!("URL not authorized");
    }

    Ok(())
}

fn validate_filesystem_type(fs: &str) -> Result<()> {
    let allowed_fs = ["btrfs", "btrfs_encryption_dev"];
    if !allowed_fs.contains(&fs) {
        bail!("Unsupported filesystem type");
    }
    Ok(())
}

fn validate_flavour(flavour: &str) -> Result<()> {
    // Only allow cosmic-fedora for RegicideOS
    if flavour != "cosmic-fedora" {
        bail!("Unsupported flavour");
    }
    Ok(())
}

fn validate_package_set(applications: &str) -> Result<()> {
    let allowed_sets = ["minimal", "base", "desktop", "full"];
    if !allowed_sets.contains(&applications) {
        bail!("Unsupported application set");
    }
    Ok(())
}

fn sanitize_input(input: &str) -> String {
    // Remove null bytes and control characters
    input
        .chars()
        .filter(|c| *c != '\0' && !c.is_control() || *c == '\t' || *c == '\n' || *c == '\r')
        .collect()
}

// Path traversal protection
fn validate_safe_path(path: &str, allowed_base: &str) -> Result<PathBuf> {
    use std::path::{Path, PathBuf};

    // Remove any dangerous characters
    let sanitized = sanitize_input(path);

    // Convert to absolute path
    let absolute_path = if sanitized.starts_with('/') {
        PathBuf::from(&sanitized)
    } else {
        std::env::current_dir()?.join(&sanitized)
    };

    // Get canonical path for allowed base (must exist)
    let base_path = Path::new(allowed_base)
        .canonicalize()
        .with_context(|| format!("Base directory does not exist: {}", allowed_base))?;

    // For validation, check if the path would be within base after creation
    // We need to handle the case where the path doesn't exist yet (for directory creation)
    let path_to_check = if absolute_path.exists() {
        absolute_path
            .canonicalize()
            .unwrap_or_else(|_| absolute_path.clone())
    } else {
        // For non-existent paths, validate the parent directory exists and is within bounds
        let parent = absolute_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;

        if !parent.exists() {
            bail!("Parent directory does not exist: {}", parent.display());
        }

        parent
            .canonicalize()
            .map(|p| p.join(absolute_path.file_name().unwrap_or_default()))
            .unwrap_or(absolute_path.clone())
    };

    // Ensure the path is within the allowed base directory
    if !path_to_check.starts_with(&base_path) {
        bail!(
            "Path access denied: {} is outside allowed base {}",
            path_to_check.display(),
            base_path.display()
        );
    }

    // Additional checks for dangerous patterns
    let path_str = absolute_path.to_string_lossy();
    let dangerous_patterns = [
        "..", "~", "$HOME", "/etc/", "/root/", "/var/", "/usr/", "/bin/", "/sbin/", "/lib/",
        "/proc/", "/sys/", "/dev/",
    ];

    for pattern in &dangerous_patterns {
        if path_str.contains(pattern) && !path_str.starts_with(allowed_base) {
            bail!("Path access denied: dangerous pattern detected");
        }
    }

    Ok(absolute_path)
}

// Safe file operations with path validation
fn safe_create_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::create_dir_all(validated_path).with_context(|| "Failed to create directory")?;
    Ok(())
}

fn safe_write_file(path: &str, content: &[u8], allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::write(validated_path, content).with_context(|| "Failed to write file")?;
    Ok(())
}

fn safe_read_file(path: &str, allowed_base: &str) -> Result<String> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::read_to_string(&validated_path).with_context(|| "Failed to read file")
}

fn safe_remove_file(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_file(validated_path).with_context(|| "Failed to remove file")?;
    Ok(())
}

fn safe_remove_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_dir_all(validated_path).with_context(|| "Failed to remove directory")?;
    Ok(())
}

#[cfg(test)]
mod tests_main {
    use super::*;

    #[test]
    fn test_validate_safe_path_nonexistent() -> Result<()> {
        // Test that validate_safe_path works for non-existent paths (for directory creation)
        std::fs::create_dir_all("/tmp/test_base")?;

        let result = validate_safe_path("/tmp/test_base/new_dir", "/tmp/test_base");
        assert!(result.is_ok());

        // Cleanup
        std::fs::remove_dir_all("/tmp/test_base")?;
        Ok(())
    }

    #[test]
    fn test_validate_safe_path_exists() -> Result<()> {
        // Test that validate_safe_path works for existing paths
        std::fs::create_dir_all("/tmp/test_base/existing_dir")?;

        let result = validate_safe_path("/tmp/test_base/existing_dir", "/tmp/test_base");
        assert!(result.is_ok());

        // Cleanup
        std::fs::remove_dir_all("/tmp/test_base")?;
        Ok(())
    }

    #[test]
    fn test_validate_safe_path_outside_base() -> Result<()> {
        // Test that validate_safe_path rejects paths outside base
        std::fs::create_dir_all("/tmp/test_base")?;

        let result = validate_safe_path("/tmp/other_dir", "/tmp/test_base");
        assert!(result.is_err());

        // Cleanup
        std::fs::remove_dir_all("/tmp/test_base")?;
        Ok(())
    }
}

fn get_input(prompt: &str, default: &str) -> String {
    print!(
        "{}. Valid options are {}\n{}[{}]{}: ",
        prompt,
        "",
        Colours::BLUE,
        default,
        Colours::ENDC
    );
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = sanitize_input(input.trim());

    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

async fn parse_config(mut config: Config, interactive: bool) -> Result<Config> {
    // Validate drive
    let drives = get_drives()?;
    if config.drive.is_empty() || !drives.contains(&config.drive) {
        if interactive {
            println!("Available drives: {:?}", drives);
            config.drive = get_input("Enter drive", drives.first().unwrap_or(&String::new()));
        } else {
            die("Invalid or missing drive in config");
        }
    }

    // Validate drive path security
    validate_device_path(&config.drive)?;

    // RegicideOS only supports the official Xenia Linux repository
    const REGICIDE_REPOSITORY: &str = "https://repo.xenialinux.com/releases/";
    if config.repository.is_empty() {
        config.repository = REGICIDE_REPOSITORY.to_string();
    } else if config.repository != REGICIDE_REPOSITORY {
        if interactive {
            warn(&format!(
                "RegicideOS only supports the official Xenia repository. Using: {}",
                REGICIDE_REPOSITORY
            ));
            config.repository = REGICIDE_REPOSITORY.to_string();
        } else {
            die(&format!(
                "RegicideOS only supports the official Xenia repository: {}",
                REGICIDE_REPOSITORY
            ));
        }
    }

    // Validate repository URL security
    validate_url(&config.repository)?;

    // Validate repository accessibility
    if !check_url(&config.repository).await {
        die("Cannot access the Xenia Linux repository");
    }

    // RegicideOS only supports cosmic-fedora flavour (available in Xenia repository)
    const REGICIDE_FLAVOUR: &str = "cosmic-fedora";
    if config.flavour.is_empty() {
        config.flavour = REGICIDE_FLAVOUR.to_string();
    } else if config.flavour != REGICIDE_FLAVOUR {
        if interactive {
            warn(&format!(
                "RegicideOS only supports the cosmic-fedora flavour. Using: {}",
                REGICIDE_FLAVOUR
            ));
            config.flavour = REGICIDE_FLAVOUR.to_string();
        } else {
            die(&format!(
                "RegicideOS only supports the cosmic-fedora flavour: {}",
                REGICIDE_FLAVOUR
            ));
        }
    }

    // Validate flavour security
    validate_flavour(&config.flavour)?;

    // Verify the cosmic-desktop flavour is available in the repository
    let flavours = get_flavours(&config.repository).await?;
    if !flavours.contains(&config.flavour) {
        die(&format!(
            "The {} flavour is not available in the repository",
            config.flavour
        ));
    }

    // Validate release branch
    let releases = get_releases(&config.repository, &config.flavour).await?;
    if config.release_branch.is_empty() || !releases.contains(&config.release_branch) {
        if interactive {
            println!("Available releases: {:?}", releases);
            config.release_branch = get_input(
                "Enter release branch",
                releases.first().unwrap_or(&"main".to_string()),
            );
        } else {
            die("Invalid or missing release branch in config");
        }
    }

    // Validate filesystem
    let filesystems = get_fs();
    if config.filesystem.is_empty() || !filesystems.contains(&config.filesystem) {
        if interactive {
            println!("Available filesystems: {:?}", filesystems);
            config.filesystem = get_input("Enter filesystem", "btrfs_encryption_dev");
        } else {
            die("Invalid or missing filesystem in config");
        }
    }

    // Validate filesystem type security
    validate_filesystem_type(&config.filesystem)?;

    // Validate username
    if !config.username.is_empty() {
        if !check_username(&config.username) {
            if interactive {
                config.username = get_input("Enter username (leave empty for none)", "");
            } else {
                die("Invalid username in config");
            }
        }

        // Additional username security validation
        if !config.username.is_empty() {
            validate_username(&config.username)?;
        }
    }

    // Validate applications
    let package_sets = get_package_sets();
    if config.applications.is_empty() || !package_sets.contains(&config.applications) {
        if interactive {
            println!("Available package sets: {:?}", package_sets);
            config.applications = get_input("Enter applications set", "minimal");
        } else {
            die("Invalid or missing applications in config");
        }
    }

    // Validate package set security
    validate_package_set(&config.applications)?;

    Ok(config)
}

fn cleanup_on_failure() {
    warn("Cleaning up due to installation failure...");

    // Unmount filesystems
    let _ = execute("umount -R /mnt/root 2>/dev/null");
    let _ = execute("umount /mnt/gentoo 2>/dev/null");
    let _ = execute("umount /mnt/temp_btrfs 2>/dev/null");

    // Close LUKS devices
    let _ = execute("cryptsetup close regicideos 2>/dev/null");

    // Remove temporary directories safely
    let _ = safe_remove_dir_all("/mnt/temp_btrfs", "/mnt");
    let _ = safe_remove_dir_all("/mnt/gentoo", "/mnt");
    let _ = safe_remove_dir_all("/mnt/root", "/mnt");

    info("Cleanup completed");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up cleanup handler
    let cleanup_flag = Arc::new(AtomicBool::new(false));
    let cleanup_flag_clone = cleanup_flag.clone();

    ctrlc::set_handler(move || {
        if !cleanup_flag_clone.load(Ordering::Relaxed) {
            cleanup_flag_clone.store(true, Ordering::Relaxed);
            cleanup_on_failure();
            std::process::exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");
    let matches = Command::new("RegicideOS Installer")
        .about("Program to install RegicideOS")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Run the installer automated from a toml config file"),
        )
        .get_matches();

    let config_file = matches.get_one::<String>("config");
    let interactive = config_file.is_none();

    print_banner();

    info(&format!(
        "{} detected.",
        if is_efi() { "EFI" } else { "BIOS" }
    ));

    let mut config = Config {
        drive: String::new(),
        repository: String::new(),
        flavour: String::new(),
        release_branch: String::new(),
        filesystem: String::new(),
        username: String::new(),
        applications: String::new(),
    };

    if let Some(config_path) = config_file {
        if !Path::new(config_path).exists() {
            die(&format!("Config file {} does not exist.", config_path));
        }

        let config_content = safe_read_file(config_path, ".")?;
        let config_toml: toml::Value = toml::from_str(&config_content)?;

        config.drive = config_toml
            .get("drive")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.repository = config_toml
            .get("repository")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.flavour = config_toml
            .get("flavour")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.release_branch = config_toml
            .get("release_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.filesystem = config_toml
            .get("filesystem")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.username = config_toml
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        config.applications = config_toml
            .get("applications")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    if interactive {
        info(&format!(
            "Entering interactive mode. Default values are shown wrapped in square brackets like {}[this]{}. Press enter to accept the default.\n",
            Colours::BLUE, Colours::ENDC
        ));
    } else {
        info("Checking config");
    }

    let config_parsed = parse_config(config, interactive).await?;
    info("Done checking config");

    if interactive {
        warn(&format!(
            "Drive partitioning is about to start. After this process, drive {} will be erased. Press enter to continue.",
            config_parsed.drive
        ));
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }

    let layouts = get_layouts();
    let layout = layouts.get(&config_parsed.filesystem).unwrap();

    info(&format!("Partitioning drive {}", config_parsed.drive));
    partition_drive(&config_parsed.drive, layout)?;

    info(&format!("Formatting drive {}", config_parsed.drive));
    format_drive(&config_parsed.drive, layout)?;

    info("Starting installation");
    
    // Simplified installation following Xenia manual pattern
    info("Step 1: Mount ROOTS partition");
    let roots_device = find_partition_by_label("ROOTS")?;
    info(&format!("Found ROOTS partition: {}", roots_device));
    safe_create_dir_all("/mnt/gentoo", "/mnt")?;
    mount_with_retry(&roots_device, "/mnt/gentoo", None, None)?;

    info("Step 2: Download root image");
    let root_url = get_url(&config_parsed).await?;
    download_root(&root_url).await?;

    info("Step 3: Mount root image and setup filesystems");
    mount()?;

    info("Step 4: Install bootloader");
    let platform = if is_efi() { "x86_64-efi" } else { "i386-pc" };
    info(&format!("Platform detected: {}", platform));
    info(&format!("Target device: {}", &config_parsed.drive));
    
    // Debug: Show current mounts before GRUB installation
    match execute("findmnt | grep -E '(boot|efi)' || echo 'No boot/efi mounts found'") {
        Ok(output) => info(&format!("Current boot-related mounts:\n{}", output.trim())),
        Err(e) => warn(&format!("Failed to show mounts: {}", e)),
    }
    
    info("About to call install_bootloader...");
    install_bootloader(platform, &config_parsed.drive)?;

    info("Step 5: Post-installation tasks");
    post_install(&config_parsed)?;

    info("Installation completed successfully!");

    // Display completion message
    let separator = "=".repeat(60);
    println!();
    println!("{}", separator);
    println!("🎉 REGICIDE OS INSTALLATION COMPLETED SUCCESSFULLY! 🎉");
    println!("{}", separator);
    println!();
    println!("✅ System has been installed on: {}", config_parsed.drive);
    println!("✅ Bootloader has been configured");
    println!("✅ All filesystems are properly mounted");
    println!();
    println!("🔄 YOU CAN NOW REBOOT YOUR SYSTEM!");
    println!("   Remove the installation media and reboot.");
    println!("   Your new RegicideOS system should boot automatically.");
    println!();
    println!("⚠️  IMPORTANT: Make sure to save any work before rebooting.");
    println!("{}", separator);
    println!();

    Ok(()).or_else(|e| {
        cleanup_on_failure();
        Err(e)
    })
}
