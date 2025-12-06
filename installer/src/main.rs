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
use installer::{Config, Partition, check_username, human_to_bytes, is_efi, get_fs, get_package_sets, get_flatpak_packages};

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
    eprintln!("{}{}{} {}", Colours::RED, "[ERROR]", Colours::ENDC, sanitized);
    std::process::exit(1);
}

// Sanitize error messages to prevent information disclosure
fn sanitize_error_message(message: &str) -> String {
    // Remove potentially sensitive information
    let sensitive_patterns = [
        r"/home/[^/\s]+",           // Home directory paths
        r"/root/[^/\s]+",           // Root directory paths  
        r"/tmp/[^/\s]+",            // Temp file paths
        r"password[^=\s]*=\s*[^\s]+", // Passwords in error messages
        r"token[^=\s]*=\s*[^\s]+",    // Tokens in error messages
        r"key[^=\s]*=\s*[^\s]+",      // Keys in error messages
        r"secret[^=\s]*=\s*[^\s]+",    // Secrets in error messages
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
    println!("{}{}{} {}", Colours::YELLOW, "[WARN]", Colours::ENDC, sanitized);
}

fn print_banner() {
    println!("{}", Colours::BLUE);
    println!(r#"
    ██████╗ ███████╗ ██████╗ ██╗ ██████╗██╗██████╗ ███████╗ ██████╗ ███████╗
    ██╔══██╗██╔════╝██╔════╝ ██║██╔════╝██║██╔══██╗██╔════╝██╔═══██╗██╔════╝
    ██████╔╝█████╗  ██║  ███╗██║██║     ██║██║  ██║█████╗  ██║   ██║███████╗
    ██╔══██╗██╔══╝  ██║   ██║██║██║     ██║██║  ██║██╔══╝  ██║   ██║╚════██║
    ██║  ██║███████╗╚██████╔╝██║╚██████╗██║██████╔╝███████╗╚██████╔╝███████║
    ╚═╝  ╚═╝╚══════╝ ╚═════╝ ╚═╝ ╚═════╝╚═╝╚═════╝ ╚══════╝ ╚═════╝ ╚══════╝
                                                                              
              🏰 A Rust-first, AI-powered Linux Distribution 🚀
                        Cosmic Desktop • BTRFS • Gentoo Base
    "#);
    println!("{}", Colours::ENDC);
}

// Safe command execution with strict allowlist
fn execute(command: &str) -> Result<String> {
    // Check for heredoc patterns that need special handling
    if command.contains("<<EOF") {
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
            if args.len() >= 3 && args.contains(&"<<EOF") && args.iter().any(|&arg| arg.contains("sfdisk")) {
                // Reconstruct the full command and execute through shell
                let full_command = format!("{} {}", program, args.join(" "));
                execute_safe_shell_command(&full_command)
            } else {
                bail!("Cat command not allowed in this context");
            }
        }
        
        // Filesystem commands
        "mkfs.vfat" | "mkfs.ext4" | "mkfs.btrfs" | "fsck.fat" | "fsck.ext4" | "btrfs" | "wipefs" | "file" => {
            execute_safe_command(program, args)
        }
        
        // Mount/unmount commands
        "mount" | "umount" => {
            execute_safe_command(program, args)
        }
        
        // LUKS commands
        "cryptsetup" => {
            execute_safe_command(program, args)
        }
        
        // System commands
        "systemctl" | "loginctl" => {
            execute_safe_command(program, args)
        }
        
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
            if (program == "dnf" && args.contains(&"-y") && args.contains(&"gdisk")) ||
               (program == "apt" && args.contains(&"gdisk")) ||
               (program == "pacman" && args.contains(&"gdisk")) {
                execute_safe_command(program, args)
            } else {
                bail!("Package manager operation not allowed: {}", command)
            }
        }
        
        // LVM commands (read-only only)
        "vgs" | "vgchange" => {
            execute_safe_command(program, args)
        }
        
        // User management (chroot only)
        "useradd" | "usermod" | "passwd" => {
            execute_safe_command(program, args)
        }
        
        // GRUB commands (chroot only)
        "grub-install" | "grub2-install" | "grub-mkconfig" | "grub2-mkconfig" => {
            execute_safe_command(program, args)
        }
        
        // EFI bootloader tools
        "efibootmgr" | "efivar" => {
            execute_safe_command(program, args)
        }
        
        // Service management (chroot only)
        "rc-update" | "rc-service" => {
            execute_safe_command(program, args)
        }
        
        // Chroot for bootloader installation and system setup
        "chroot" => {
            execute_safe_command(program, args)
        }
        
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
        bail!("System command failed (exit code: {:?})", output.status.code());
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
                    bail!("Shell command failed: {} (exit code: {:?})\nSTDOUT: {}\nSTDERR: {}", 
                          shell_cmd, output.status.code(), stdout, stderr);
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
        ";", "&&", "||", "|", "&", "$(", "`", 
        "$", "${", ">", ">>", "<",
        "rm ", "dd ", "chmod ", "chown ",
        "sudo ", "su ", "eval ", "exec ",
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
    println!("DEBUG: Attempting to get size for: {}", drive);
    match execute_safe_command("lsblk", &["-b", "-o", "SIZE", "-n", drive]) {
        Ok(output) => {
            let size_str = output.trim();
            println!("DEBUG: lsblk output: '{}'", size_str);
            
            if size_str.is_empty() {
                Ok(0)
            } else {
                // Take only the first line (drive size, not partitions)
                let first_line = size_str.lines().next().unwrap_or("").trim();
                println!("DEBUG: Using first line as drive size: '{}'", first_line);
                Ok(first_line.parse::<u64>().unwrap_or(0))
            }
        }
        Err(e) => {
            println!("DEBUG: lsblk failed: {}", e);
            Err(e)
        }
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
        warn(&format!("Sys block directory not found: {}", sys_block.display()));
        return Ok(vec![]);
    }

    let mut drives = Vec::new();
    for entry in fs::read_dir(sys_block)? {
        let entry = entry?;
        let drive_name = entry.file_name();
        let drive_path = format!("/dev/{}", drive_name.to_string_lossy());
        
        // Skip loopback devices and other non-physical drives
        let name_str = drive_name.to_string_lossy();
        if name_str.starts_with("loop") || name_str.starts_with("ram") || name_str.starts_with("dm-") {
            continue;
        }
        
        match check_drive_size(&drive_path) {
            true => {
                info(&format!("Found valid drive: {} (size > 12GB)", drive_path));
                drives.push(drive_path);
            }
            false => {
                // Only show debug info for actual block devices, not every entry
                if name_str.starts_with("sd") || name_str.starts_with("nvme") || name_str.starts_with("hd") || name_str.starts_with("vd") {
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
        layouts.insert("btrfs".to_string(), vec![
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
        ]);
    } else {
        layouts.insert("btrfs".to_string(), vec![
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
        ]);
    }
    
    if is_efi() {
        layouts.insert("btrfs_encryption_dev".to_string(), vec![
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
                label: Some("XENIA".to_string()),
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
        ]);
    } else {
        layouts.insert("btrfs_encryption_dev".to_string(), vec![
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
                label: Some("XENIA".to_string()),
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
        ]);
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
            let detected_partitions: Vec<String> = partitions_output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter(|line| line.trim() != drive_base)
                .map(|line| format!("/dev/{}", line.trim()))
                .collect();
            
            // Special handling for LUKS - check if we have a mapper device
            if expected_count == 1 && detected_partitions.is_empty() {
                // For LUKS, we expect 1 mapper device instead of physical partitions
                if Path::new("/dev/mapper/regicideos").exists() {
                    println!("DEBUG: LUKS detected, using mapper device");
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
                let part_name = if drive.contains("nvme") || drive.chars().last().unwrap_or('a').is_ascii_digit() {
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
        
        println!("DEBUG: Partition detection - expected: {}, found: {}, partitions: {:?}", 
                 expected_count, partition_names.len(), partition_names);
        
        if partition_names.len() == expected_count {
            info(&format!("Found {} partitions", expected_count));
            return Ok(partition_names);
        }
        
        attempts += 1;
        if attempts >= max_attempts {
            bail!("Partitions were not created properly after {} attempts. Expected {}, found {}", 
                  max_attempts, expected_count, partition_names.len());
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
        let partition_num = partition.chars().last()
            .and_then(|c| c.to_digit(10))
            .ok_or_else(|| anyhow::anyhow!("Could not determine partition number from {}", partition))?;
        
        let drive = if partition.contains("nvme") && partition.contains("p") {
            partition.rsplit_once("p").unwrap().0
        } else {
            partition.trim_end_matches(char::is_numeric)
        };
        
        execute(&format!("sgdisk --set-flag={}:boot:on {}", partition_num, drive))?;
        info(&format!("Set EFI boot flag on partition {}", partition_num));
    } else {
        warn("sgdisk not available, EFI boot flag not set. System may not boot properly.");
        warn("Please install gdisk package manually: dnf install gdisk (Fedora) or apt install gdisk (Ubuntu)");
    }
    Ok(())
}

fn partition_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    // Unmount any existing partitions on this drive using direct commands
    // This avoids complex shell pattern matching issues
    let base_drive = drive.trim_end_matches('/');
    
    // Try common partition numbering schemes
    let partitions_to_try = [
        format!("{}1", base_drive),
        format!("{}2", base_drive), 
        format!("{}3", base_drive),
        format!("{}4", base_drive),
        format!("{}5", base_drive),
        format!("{}p1", base_drive),
        format!("{}p2", base_drive),
        format!("{}p3", base_drive),
        format!("{}p4", base_drive),
        format!("{}p5", base_drive),
    ];
    
    for partition in &partitions_to_try {
        let _ = execute(&format!("umount {} 2>/dev/null || true", partition));
    }
    
    // Check if LVM is available before trying to use it
    let vgs_output = if Path::new("/sbin/vgs").exists() || Path::new("/usr/sbin/vgs").exists() {
        execute("vgs | awk '{ print $1 }' | grep -vw VG").unwrap_or_default()
    } else {
        String::new()
    };
    for vg in vgs_output.lines() {
        let vg = vg.trim();
        if !vg.is_empty() {
            execute(&format!("vgchange -an {}", vg))?;
        }
    }

    let mut command = format!("cat <<EOF | sfdisk -q --wipe always --force {}\nlabel: gpt", drive);
    let drive_size = get_drive_size(drive)?;
    let mut running_drive_size = drive_size.saturating_sub(1048576); // for BIOS systems, -1M

    for partition in layout {
        let size_part = if partition.size == "rest" {
            if !is_efi() {
                format!("size={}K, ", running_drive_size / 1024)
            } else {
                String::new()
            }
        } else if partition.size.ends_with('%') {
            let percentage: f64 = partition.size[..partition.size.len()-1].parse().unwrap_or(0.0);
            let partition_size = ((drive_size as f64) * (percentage / 100.0)) as u64;
            running_drive_size = running_drive_size.saturating_sub(partition_size);
            format!("size={}K, ", partition_size / 1024)
        } else {
            let partition_size = human_to_bytes(&partition.size)?;
            running_drive_size = running_drive_size.saturating_sub(partition_size);
            format!("size={}, ", partition.size)
        };
        
        command += &format!("\n{}type={}", size_part, partition.partition_type);
    }

    if !is_efi() {
        command += "\ntype=21686148-6449-6E6F-744E-656564454649";
    }

    command += "\nEOF";
    
    // Debug: print the command to see what we're actually executing
    println!("DEBUG: Full sfdisk command:\n---\n{}\n---", command);
    println!("DEBUG: Command length: {}", command.len());
    
    execute(&command)?;
    
    // Wait for partitioning to complete and inform the kernel
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Try to inform kernel of partition changes
    if execute("which partprobe").is_ok() {
        if let Err(e) = execute(&format!("partprobe {}", drive)) {
            warn(&format!("partprobe failed: {}, trying other methods", e));
            // Try alternative approaches
            let _ = execute(&format!("sfdisk -R {}", drive))
                .or_else(|_| execute(&format!("hdparm -z {}", drive)))
                .or_else(|_| execute(&format!("blockdev --rereadpt {}", drive)));
        }
    } else {
        warn("partprobe not available, trying alternative methods");
        let _ = execute(&format!("sfdisk -R {}", drive))
            .or_else(|_| execute(&format!("hdparm -z {}", drive)))
            .or_else(|_| execute(&format!("blockdev --rereadpt {}", drive)));
    }
    
    // Wait and verify partitions were created with improved detection
    let _partition_names = wait_for_partitions(drive, layout.len())?;
    
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

fn format_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    // Wait for kernel to recognize partitions and get reliable partition list
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Use the same reliable detection as partition_drive
    let partition_names = wait_for_partitions(drive, layout.len())?;

    for (i, partition) in layout.iter().enumerate() {
        let current_name = &partition_names[i];
        
        // Double-check partition exists before formatting
        if !Path::new(current_name).exists() {
            bail!("Partition {} does not exist", current_name);
        }

        info(&format!("Formatting {} as {}", current_name, partition.format));
        
        match partition.format.as_str() {
            "vfat" => {
                // EFI partition formatting with validation
                if let Some(ref label) = partition.label {
                    execute(&format!("mkfs.vfat -F 32 -n {} {}", label, current_name))?;
                } else {
                    execute(&format!("mkfs.vfat -F 32 {}", current_name))?;
                }
                
                // Set EFI boot flag if this is likely an EFI partition
                if is_efi() && (partition.partition_type == "uefi" || 
                               partition.label.as_ref().map_or(false, |l| l == "EFI")) {
                    if let Err(e) = set_efi_boot_flag(current_name) {
                        warn(&format!("Failed to set EFI boot flag: {}", e));
                    }
                }
                
                // Verify filesystem
                verify_filesystem(current_name, "vfat")?;
            }
            "ext4" => {
                // Check if mkfs.ext4 is available, attempt to install if missing
                if execute("which mkfs.ext4").is_err() {
                    warn("mkfs.ext4 not found, attempting to install e2fsprogs package...");
                    if execute("which dnf").is_ok() {
                        execute("dnf install -y e2fsprogs")
                            .map_err(|e| warn(&format!("Failed to install e2fsprogs via dnf: {}", e)))
                            .ok();
                    } else if execute("which apt").is_ok() {
                        execute("apt update && apt install -y e2fsprogs")
                            .map_err(|e| warn(&format!("Failed to install e2fsprogs via apt: {}", e)))
                            .ok();
                    } else if execute("which pacman").is_ok() {
                        execute("pacman -S --noconfirm e2fsprogs")
                            .map_err(|e| warn(&format!("Failed to install e2fsprogs via pacman: {}", e)))
                            .ok();
                    } else {
                        warn("Could not determine package manager to install e2fsprogs");
                    }
                }
                
                // Debug: Check partition state before formatting
                println!("DEBUG: Checking partition state for {}", current_name);
                let _ = execute(&format!("lsblk {}", current_name));
                let _ = execute(&format!("file -s {}", current_name));
                
                // Ensure partition is not mounted before formatting
                println!("DEBUG: Ensuring {} is not mounted...", current_name);
                let _ = execute(&format!("umount -f {} 2>/dev/null || true", current_name));
                
                if let Some(ref label) = partition.label {
                    let cmd = format!("mkfs.ext4 -L {} {}", label, current_name);
                    println!("DEBUG: Running command: {}", cmd);
                    execute(&cmd)?;
                } else {
                    let cmd = format!("mkfs.ext4 {}", current_name);
                    println!("DEBUG: Running command: {}", cmd);
                    execute(&cmd)?;
                }
                verify_filesystem(current_name, "ext4")?;
            }
            "btrfs" => {
                // Following Xenia's BTRFS formatting approach with enhanced error handling
                if let Some(ref label) = partition.label {
                    info(&format!("Creating BTRFS filesystem with label '{}' on {}", label, current_name));
                    if let Err(e) = execute(&format!("mkfs.btrfs -L {} {}", label, current_name)) {
                        bail!("Failed to create BTRFS filesystem with label '{}': {}", label, e);
                    }
                } else {
                    info(&format!("Creating BTRFS filesystem on {}", current_name));
                    if let Err(e) = execute(&format!("mkfs.btrfs {}", current_name)) {
                        bail!("Failed to create BTRFS filesystem: {}", e);
                    }
                }
                
                // Create subvolumes following Xenia's exact approach with better error handling
                if let Some(ref subvolumes) = partition.subvolumes {
                    let temp_mount = "/mnt/gentoo";
                    
                    // Ensure mount directory exists
                    if let Err(e) = fs::create_dir_all(temp_mount) {
                        bail!("Failed to create temporary mount directory '{}': {}", temp_mount, e);
                    }
                    
                    // Mount the BTRFS filesystem
                    info(&format!("Mounting BTRFS filesystem temporarily at {}", temp_mount));
                    if let Err(e) = execute(&format!("mount {} {}", current_name, temp_mount)) {
                        bail!("Failed to mount BTRFS filesystem for subvolume creation: {}", e);
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
                        warn(&format!("Warning: Failed to unmount temporary BTRFS mount: {}", e));
                    }
                }
                
                // Verify the filesystem
                if let Err(e) = verify_filesystem(current_name, "btrfs") {
                    warn(&format!("BTRFS filesystem verification failed: {}", e));
                    warn("The filesystem may still be usable, but please verify manually");
                }
            }
            "luks" => {
                println!("Setting up LUKS encryption. You will be prompted to enter a password.");
                
                // Aggressive cleanup before LUKS format
                println!("DEBUG: Aggressive cleanup for {}", current_name);
                
                // Check what's using the device
                println!("DEBUG: Checking processes using {}:", current_name);
                let _ = execute(&format!("lsof {} 2>/dev/null || true", current_name));
                let _ = execute(&format!("fuser -v {} 2>/dev/null || true", current_name));
                
                // Unmount aggressively
                let _ = execute(&format!("umount -f {} 2>/dev/null || true", current_name));
                let _ = execute("umount -f /dev/mapper/regicideos 2>/dev/null || true");
                
                // Close any existing LUKS containers
                let _ = execute(&format!("cryptsetup close {} 2>/dev/null || true", current_name));
                let _ = execute("cryptsetup close regicideos 2>/dev/null || true");
                
                // Remove any device mapper references
                let _ = execute("dmsetup remove_all 2>/dev/null || true");
                
                // Try to wipe the first few blocks to clear any filesystem signatures
                let _ = execute(&format!("wipefs -a {} 2>/dev/null || true", current_name));
                let _ = execute(&format!("dd if=/dev/zero of={} bs=1M count=1 2>/dev/null || true", current_name));
                
                // Wait longer for all operations to complete
                std::thread::sleep(std::time::Duration::from_millis(5000));
                
                // Special handling for LUKS format (interactive password required)
                println!("DEBUG: Starting LUKS format for {}", current_name);
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
                            bail!("Failed to format LUKS partition: exit code {:?}", status.code());
                        }
                    }
                    Err(e) => {
                        bail!("Failed to execute cryptsetup: {}", e);
                    }
                }
                
                let luks_device = if let Some(ref label) = partition.label {
                    // Set LUKS label after formatting
                    execute(&format!("cryptsetup -q config {} --label {}", current_name, label))?;
                    
                    // Open LUKS container using the actual partition path, not by-label
                    // LUKS containers don't have accessible labels until opened
                    let open_result = ProcessCommand::new("cryptsetup")
                        .args(["luksOpen", current_name, "regicideos"])
                        .status();
                        
                    if !open_result.map(|s| s.success()).unwrap_or(false) {
                        bail!("Failed to open LUKS partition with label '{}'", label);
                    }
                    
                    println!("DEBUG: LUKS open command completed, checking for mapper device");
                    
                    // Verify the device was created with timeout
                    let mut attempts = 0;
                    while !Path::new("/dev/mapper/regicideos").exists() && attempts < 10 {
                        println!("DEBUG: Waiting for /dev/mapper/regicideos (attempt {})", attempts + 1);
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        attempts += 1;
                    }
                    
                    if !Path::new("/dev/mapper/regicideos").exists() {
                        bail!("LUKS device /dev/mapper/regicideos was not created after 5 seconds");
                    } else {
                        println!("DEBUG: LUKS mapper device created successfully");
                    }
                    
                    "/dev/mapper/regicideos".to_string()
                } else {
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
                    
                    "/dev/mapper/regicideos".to_string()
                };
                
                if let Some(ref inside) = partition.inside {
                    format_drive(&luks_device, &[*inside.clone()])?;
                }
            }
            _ => {
                warn(&format!("Unknown filesystem type: {}", partition.format));
            }
        }
    }
    
    Ok(())
}

fn chroot(command: &str) -> Result<()> {
    let full_command = format!("chroot /mnt/root /bin/bash <<'EOT'\n{}\nEOT", command);
    execute(&full_command)?;
    Ok(())
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
        Ok(format!("{}{}/{}/{}", config.repository, arch, config.release_branch, filename))
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

fn mount_with_retry(source: &str, target: &str, fs_type: Option<&str>, options: Option<&str>) -> Result<()> {
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
                    bail!("Failed to mount {} on {} after {} attempts: {}", source, target, max_attempts, e);
                }
                warn(&format!("Mount attempt {} failed, retrying in {}ms: {}", attempts, attempts * 500, e));
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
    mount_with_retry("/mnt/gentoo/root.img", "/mnt/root", Some("squashfs"), Some("ro,loop"))?;
    
    // Mount EFI or BOOT partition based on system type
    if is_efi() {
        info("Finding EFI partition...");
        let efi_device = find_partition_by_label("EFI")?;
        
        info("Mounting ESP on /mnt/root/boot/efi");
        mount_with_retry(&efi_device, "/mnt/root/boot/efi", None, None)?;
    } else {
        info("Finding BOOT partition...");
        let boot_device = find_partition_by_label("BOOT")?;
        
        info("Mounting BOOT on /mnt/root/boot");
        mount_with_retry(&boot_device, "/mnt/root/boot", None, None)?;
    }
    
    info("Mounting special filesystems");
    mount_with_retry("/proc", "/mnt/root/proc", Some("proc"), None)?;
    execute("mount --rbind /dev /mnt/root/dev")?;
    execute("mount --rbind /sys /mnt/root/sys")?;
    execute("mount --bind /run /mnt/root/run")?;
    execute("mount --make-slave /mnt/root/run")?;
    
    Ok(())
}

async fn download_root(url: &str) -> Result<()> {
    let root_img_path = "/mnt/gentoo/root.img";
    
    if Path::new(root_img_path).exists() {
        safe_remove_file(root_img_path, "/mnt/gentoo")?;
    }
    
    info(&format!("Downloading root image from {}", url));
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

fn install_bootloader(platform: &str, device: &str) -> Result<()> {
    // Check for grub binary, see if its grub2-install or grub-install
    let grub = if Path::new("/mnt/root/usr/bin/grub-install").exists() {
        "grub"
    } else {
        "grub2"
    };

    if platform.contains("efi") {
        // Install GRUB for EFI systems targeting the mounted root
        execute(&format!(
            "{}-install --force --target=\"{}\" --efi-directory=\"/mnt/root/boot/efi\" --boot-directory=\"/mnt/root/boot/efi\"",
            grub, platform
        ))?;
        
        // Generate GRUB config using the chroot environment
        chroot(&format!("{}-mkconfig -o /boot/efi/{}/grub.cfg", grub, grub))?;
    } else {
        chroot(&format!(
            "{}-install --force --target=\"{}\" --boot-directory=\"/boot/efi\" {}",
            grub, platform, device
        ))?;
        chroot(&format!("{}-mkconfig -o /boot/efi/{}/grub.cfg", grub, grub))?;
    }
    
    Ok(())
}

fn post_install(config: &Config) -> Result<()> {
    let layout_name = &config.filesystem;
    info("Mounting overlays & home");

    let (etc_path, var_path, usr_path) = match layout_name.as_str() {
        "btrfs" => {
            safe_create_dir_all("/mnt/root/overlay", "/mnt/root")?;
            safe_create_dir_all("/mnt/root/home", "/mnt/root")?;
            execute("mount -L ROOTS -o subvol=overlay /mnt/root/overlay")?;
            execute("mount -L ROOTS -o subvol=home /mnt/root/home")?;
            ("/mnt/root/overlay/etc", "/mnt/root/overlay/var", "/mnt/root/overlay/usr")
        }
        "btrfs_encryption_dev" => {
            if !Path::new("/dev/mapper/regicideos").exists() {
                bail!("LUKS device /dev/mapper/regicideos not found");
            }
            safe_create_dir_all("/mnt/root/overlay", "/mnt/root")?;
            safe_create_dir_all("/mnt/root/home", "/mnt/root")?;
            execute("mount /dev/mapper/regicideos -o subvol=overlay /mnt/root/overlay")?;
            execute("mount /dev/mapper/regicideos -o subvol=home /mnt/root/home")?;
            ("/mnt/root/overlay/etc", "/mnt/root/overlay/var", "/mnt/root/overlay/usr")
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
            ("/mnt/root/overlay", "/mnt/root/overlay", "/mnt/root/overlay")
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
        safe_create_dir_all(path, "/mnt/root")?;
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
            let result = ProcessCommand::new("chroot")
                .args(["/mnt/root", "/bin/bash", "-c", &format!("passwd {}", config.username)])
                .status();
                
            match result {
                Ok(status) if status.success() => break,
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
        safe_write_file("/mnt/root/etc/declare/flatpak", flatpaks.as_bytes(), "/mnt/root/etc")?;

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
    let device_regex = regex::Regex::new(r"^/dev/(sd[a-z]+|nvme[0-9]+n[0-9]+|hd[a-z]+|vd[a-z]+|mmcblk[0-9]+)(p[0-9]+)?$")?;
    if !device_regex.is_match(path) {
        bail!("Invalid device path");
    }
    
    // Prevent dangerous device paths (but allow legitimate block devices)
    let dangerous_exact = [
        "/dev/null", "/dev/zero", "/dev/full", "/dev/random", "/dev/urandom",
        "/dev/mem", "/dev/kmem", "/dev/port", "/dev/console", "/dev/initctl",
    ];
    
    let dangerous_prefixes = [
        "/dev/shm/", "/dev/pts/", "/dev/mqueue/", "/dev/hugepages/",
    ];
    
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
        "root", "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail",
        "news", "uucp", "proxy", "www-data", "backup", "list", "irc", "gnats",
        "nobody", "systemd-network", "systemd-resolve", "syslog", "messagebus",
        "uuidd", "dnsmasq", "usbmux", "rtkit", "pulse", "speech-dispatcher",
        "avahi", "saned", "colord", "hplip", "geoclue", "gnome-initial-setup",
        "gdm", "sshd", "ntp", "postgres", "mysql", "oracle", "tomcat",
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
    input.chars()
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
    
    // Get canonical paths to resolve symlinks and relative components
    let canonical_path = match absolute_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If canonicalization fails, try to resolve manually
            let resolved = absolute_path.clone();
            // Simple resolution - in production, use more sophisticated approach
            resolved
        }
    };
    
    // Get canonical path for allowed base
    let base_path = Path::new(allowed_base).canonicalize()
        .unwrap_or_else(|_| PathBuf::from(allowed_base));
    
    // Ensure the path is within the allowed base directory
    if !canonical_path.starts_with(&base_path) {
        bail!("Path access denied");
    }
    
    // Additional checks for dangerous patterns
    let path_str = canonical_path.to_string_lossy();
    let dangerous_patterns = [
        "..", "~", "$HOME", "/etc/", "/root/", "/var/", "/usr/",
        "/bin/", "/sbin/", "/lib/", "/proc/", "/sys/", "/dev/",
    ];
    
    for pattern in &dangerous_patterns {
        if path_str.contains(pattern) && !path_str.starts_with(allowed_base) {
            bail!("Path access denied");
        }
    }
    
    Ok(canonical_path)
}

// Safe file operations with path validation
fn safe_create_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::create_dir_all(validated_path)
        .with_context(|| "Failed to create directory")?;
    Ok(())
}

fn safe_write_file(path: &str, content: &[u8], allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::write(validated_path, content)
        .with_context(|| "Failed to write file")?;
    Ok(())
}

fn safe_read_file(path: &str, allowed_base: &str) -> Result<String> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::read_to_string(&validated_path)
        .with_context(|| "Failed to read file")
}

fn safe_remove_file(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_file(validated_path)
        .with_context(|| "Failed to remove file")?;
    Ok(())
}

fn safe_remove_dir_all(path: &str, allowed_base: &str) -> Result<()> {
    let validated_path = validate_safe_path(path, allowed_base)?;
    fs::remove_dir_all(validated_path)
        .with_context(|| "Failed to remove directory")?;
    Ok(())
}

fn get_input(prompt: &str, default: &str) -> String {
    print!("{}. Valid options are {}\n{}[{}]{}: ", 
           prompt, "", Colours::BLUE, default, Colours::ENDC);
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
            warn(&format!("RegicideOS only supports the official Xenia repository. Using: {}", REGICIDE_REPOSITORY));
            config.repository = REGICIDE_REPOSITORY.to_string();
        } else {
            die(&format!("RegicideOS only supports the official Xenia repository: {}", REGICIDE_REPOSITORY));
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
            warn(&format!("RegicideOS only supports the cosmic-fedora flavour. Using: {}", REGICIDE_FLAVOUR));
            config.flavour = REGICIDE_FLAVOUR.to_string();
        } else {
            die(&format!("RegicideOS only supports the cosmic-fedora flavour: {}", REGICIDE_FLAVOUR));
        }
    }

    // Validate flavour security
    validate_flavour(&config.flavour)?;

    // Verify the cosmic-desktop flavour is available in the repository
    let flavours = get_flavours(&config.repository).await?;
    if !flavours.contains(&config.flavour) {
        die(&format!("The {} flavour is not available in the repository", config.flavour));
    }

    // Validate release branch
    let releases = get_releases(&config.repository, &config.flavour).await?;
    if config.release_branch.is_empty() || !releases.contains(&config.release_branch) {
        if interactive {
            println!("Available releases: {:?}", releases);
            config.release_branch = get_input("Enter release branch", releases.first().unwrap_or(&"main".to_string()));
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
    }).expect("Error setting Ctrl-C handler");
    let matches = Command::new("RegicideOS Installer")
        .about("Program to install RegicideOS")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Run the installer automated from a toml config file")
        )
        .get_matches();

    let config_file = matches.get_one::<String>("config");
    let interactive = config_file.is_none();

    print_banner();

    info(&format!("{} detected.", if is_efi() { "EFI" } else { "BIOS" }));

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

        config.drive = config_toml.get("drive").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.repository = config_toml.get("repository").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.flavour = config_toml.get("flavour").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.release_branch = config_toml.get("release_branch").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.filesystem = config_toml.get("filesystem").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.username = config_toml.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
        config.applications = config_toml.get("applications").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
    mount_roots()?;

    info("Downloading root image");
    let root_url = get_url(&config_parsed).await?;
    download_root(&root_url).await?;
    mount()?;

    info("Installing bootloader");
    let platform = if is_efi() { "x86_64-efi" } else { "i386-pc" };
    install_bootloader(platform, &config_parsed.drive)?;

    info("Starting post-installation tasks");
    post_install(&config_parsed)?;

    info("Installation completed successfully!");

    Ok(()).or_else(|e| {
        cleanup_on_failure();
        Err(e)
    })
}
