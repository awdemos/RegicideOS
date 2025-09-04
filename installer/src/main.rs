use anyhow::{bail, Context, Result};
use clap::{Arg, Command};
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
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
    eprintln!("{}{}{} {}", Colours::RED, "[ERROR]", Colours::ENDC, message);
    std::process::exit(1);
}

fn info(message: &str) {
    println!("{}{}{} {}", Colours::BLUE, "[INFO]", Colours::ENDC, message);
}

fn warn(message: &str) {
    println!("{}{}{} {}", Colours::YELLOW, "[WARN]", Colours::ENDC, message);
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
                          Cosmic Fedora Desktop • BTRFS
    "#);
    println!("{}", Colours::ENDC);
}

fn execute(command: &str) -> Result<String> {
    let output = ProcessCommand::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .context("Failed to execute command")?;

    if !output.status.success() {
        bail!("Command failed: {}", command);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_drive_size(drive: &str) -> Result<u64> {
    let command = format!("lsblk -bo SIZE {} | grep -v SIZE | head -1", drive);
    let output = execute(&command)?;
    let size_str = output.trim();
    
    if size_str.is_empty() {
        Ok(0)
    } else {
        Ok(size_str.parse::<u64>().unwrap_or(0))
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
        return Ok(vec![]);
    }

    let mut drives = Vec::new();
    for entry in fs::read_dir(sys_block)? {
        let entry = entry?;
        let drive_name = entry.file_name();
        let drive_path = format!("/dev/{}", drive_name.to_string_lossy());
        
        if check_drive_size(&drive_path) {
            drives.push(drive_path);
        }
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

fn partition_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    // Use sh -c to properly expand the glob pattern
    execute(&format!("sh -c 'umount -ql {}?* 2>/dev/null || true'", drive))?;
    
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
    execute(&command)?;
    
    // Wait for partitioning to complete and inform the kernel
    std::thread::sleep(std::time::Duration::from_secs(2));
    execute(&format!("partprobe {}", drive))?;
    
    // Wait for kernel to recognize new partitions
    std::thread::sleep(std::time::Duration::from_secs(3));
    
    // Verify partitions were created
    let mut attempts = 0;
    loop {
        let result = execute(&format!("lsblk {} | wc -l", drive));
        if result.is_ok() && result.unwrap().trim().parse::<i32>().unwrap_or(0) > 1 {
            break;
        }
        attempts += 1;
        if attempts >= 10 {
            warn("Partitions may not have been created properly");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    Ok(())
}

fn format_drive(drive: &str, layout: &[Partition]) -> Result<()> {
    let name_output = execute(&format!("lsblk -o NAME --list | grep '{}.' | head -1", drive.split('/').last().unwrap_or("")))?;
    let mut name = format!("/dev/{}", name_output.trim());
    
    let no_num = name == "/dev/";
    if no_num {
        name = drive.to_string();
    } else {
        name = name.replace("-", "/");
    }
    
    let mut number = if !no_num && !name.is_empty() {
        name.chars().last().unwrap_or('1').to_digit(10).unwrap_or(1)
    } else {
        1
    };

    for partition in layout {
        let current_name = if !no_num {
            format!("{}{}", &name[..name.len()-1], number)
        } else {
            name.clone()
        };
        
        if !no_num {
            number += 1;
        }

        match partition.format.as_str() {
            "vfat" => {
                if let Some(ref label) = partition.label {
                    execute(&format!("mkfs.vfat -I -F 32 -n {} {}", label, current_name))?;
                } else {
                    execute(&format!("mkfs.vfat -I -F 32 {}", current_name))?;
                }
            }
            "ext4" => {
                if let Some(ref label) = partition.label {
                    execute(&format!("mkfs.ext4 -q -L {} {}", label, current_name))?;
                } else {
                    execute(&format!("mkfs.ext4 -q {}", current_name))?;
                }
            }
            "btrfs" => {
                if let Some(ref label) = partition.label {
                    execute(&format!("mkfs.btrfs -q -f -L {} {}", label, current_name))?;
                } else {
                    execute(&format!("mkfs.btrfs -q -f {}", current_name))?;
                }
                
                if let Some(ref subvolumes) = partition.subvolumes {
                    let temp_mount = "/mnt/temp_btrfs";
                    fs::create_dir_all(temp_mount).ok();
                    execute(&format!("mount {} {}", current_name, temp_mount))?;
                    
                    for subvolume in subvolumes {
                        execute(&format!("btrfs subvolume create {}{}", temp_mount, subvolume))?;
                    }
                    
                    execute(&format!("umount {}", temp_mount))?;
                    fs::remove_dir(temp_mount).ok();
                }
            }
            "luks" => {
                println!("Setting up LUKS encryption. You will be prompted to enter a password.");
                
                // Use ProcessCommand for interactive password input
                let format_result = ProcessCommand::new("cryptsetup")
                    .args(["-q", "luksFormat", &current_name])
                    .status();
                    
                if !format_result.map(|s| s.success()).unwrap_or(false) {
                    bail!("Failed to format LUKS partition");
                }
                
                let luks_device = if let Some(ref label) = partition.label {
                    execute(&format!("cryptsetup -q config {} --label {}", current_name, label))?;
                    
                    let open_result = ProcessCommand::new("cryptsetup")
                        .args(["luksOpen", &format!("/dev/disk/by-label/{}", label), "xenia"])
                        .status();
                        
                    if !open_result.map(|s| s.success()).unwrap_or(false) {
                        bail!("Failed to open LUKS partition");
                    }
                    
                    // Verify the device was created
                    if !Path::new("/dev/mapper/xenia").exists() {
                        bail!("LUKS device /dev/mapper/xenia was not created");
                    }
                    
                    "/dev/mapper/xenia".to_string()
                } else {
                    let open_result = ProcessCommand::new("cryptsetup")
                        .args(["luksOpen", &current_name, "xenia"])
                        .status();
                        
                    if !open_result.map(|s| s.success()).unwrap_or(false) {
                        bail!("Failed to open LUKS partition");
                    }
                    
                    // Verify the device was created
                    if !Path::new("/dev/mapper/xenia").exists() {
                        bail!("LUKS device /dev/mapper/xenia was not created");
                    }
                    
                    "/dev/mapper/xenia".to_string()
                };
                
                if let Some(ref inside) = partition.inside {
                    format_drive(&luks_device, &[*inside.clone()])?;
                }
            }
            _ => {}
        }
    }
    
    Ok(())
}

fn chroot(command: &str) -> Result<()> {
    let full_command = format!("chroot /mnt/root /bin/bash -c '{}'", command);
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

fn mount_roots() -> Result<()> {
    let mount_point = "/mnt/gentoo";
    fs::create_dir_all(mount_point).ok();
    
    // Check if ROOTS label exists
    if !Path::new("/dev/disk/by-label/ROOTS").exists() {
        bail!("ROOTS partition not found. Partitioning may have failed.");
    }
    
    info("Mounting roots on /mnt/gentoo");
    execute(&format!("mount -L ROOTS {}", mount_point))?;
    Ok(())
}

fn mount() -> Result<()> {
    fs::create_dir_all("/mnt/root").ok();
    
    info("Mounting root.img on /mnt/root");
    execute("mount -o ro,loop -t squashfs /mnt/gentoo/root.img /mnt/root")?;
    
    // Check if EFI label exists
    if !Path::new("/dev/disk/by-label/EFI").exists() {
        bail!("EFI partition not found. Partitioning may have failed.");
    }
    
    info("Mounting ESP on /mnt/root/boot/efi");
    fs::create_dir_all("/mnt/root/boot/efi").ok();
    execute("mount -L EFI /mnt/root/boot/efi")?;
    
    info("Mounting special filesystems");
    execute("mount -t proc /proc /mnt/root/proc")?;
    execute("mount --rbind /dev /mnt/root/dev")?;
    execute("mount --rbind /sys /mnt/root/sys")?;
    execute("mount --bind /run /mnt/root/run")?;
    execute("mount --make-slave /mnt/root/run")?;
    
    Ok(())
}

async fn download_root(url: &str) -> Result<()> {
    let root_img_path = "/mnt/gentoo/root.img";
    
    if Path::new(root_img_path).exists() {
        fs::remove_file(root_img_path)?;
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
    
    fs::write(root_img_path, bytes)?;
    
    // Verify the file was written and has content
    let metadata = fs::metadata(root_img_path)?;
    if metadata.len() == 0 {
        bail!("Root image file is empty after download");
    }
    
    info(&format!("Downloaded {} bytes", metadata.len()));
    Ok(())
}

fn install_bootloader(platform: &str, device: &str) -> Result<()> {
    let grub = if Path::new("/mnt/root/usr/bin/grub-install").exists() {
        "grub"
    } else {
        "grub2"
    };

    if platform.contains("efi") {
        chroot(&format!(
            "{}-install --force --target=\"{}\" --efi-directory=\"/boot/efi\" --boot-directory=\"/boot/efi\"",
            grub, platform
        ))?;
        chroot(&format!("{}-mkconfig -o /boot/efi/{}/grub.cfg", grub, grub))?;
    } else {
        chroot(&format!(
            "{}-install --force --target=\"{}\" --boot-directory=\"/boot\" {}",
            grub, platform, device
        ))?;
        chroot(&format!("{}-mkconfig -o /boot/{}/grub.cfg", grub, grub))?;
    }
    
    Ok(())
}

fn post_install(config: &Config) -> Result<()> {
    let layout_name = &config.filesystem;
    info("Mounting overlays & home");

    let (etc_path, var_path, usr_path) = match layout_name.as_str() {
        "btrfs" => {
            fs::create_dir_all("/mnt/root/overlay").ok();
            fs::create_dir_all("/mnt/root/home").ok();
            execute("mount -L ROOTS -o subvol=overlay /mnt/root/overlay")?;
            execute("mount -L ROOTS -o subvol=home /mnt/root/home")?;
            ("/mnt/root/overlay/etc", "/mnt/root/overlay/var", "/mnt/root/overlay/usr")
        }
        "btrfs_encryption_dev" => {
            if !Path::new("/dev/mapper/xenia").exists() {
                bail!("LUKS device /dev/mapper/xenia not found");
            }
            fs::create_dir_all("/mnt/root/overlay").ok();
            fs::create_dir_all("/mnt/root/home").ok();
            execute("mount /dev/mapper/xenia -o subvol=overlay /mnt/root/overlay")?;
            execute("mount /dev/mapper/xenia -o subvol=home /mnt/root/home")?;
            ("/mnt/root/overlay/etc", "/mnt/root/overlay/var", "/mnt/root/overlay/usr")
        }
        _ => {
            if !Path::new("/dev/disk/by-label/OVERLAY").exists() {
                bail!("OVERLAY partition not found");
            }
            if !Path::new("/dev/disk/by-label/HOME").exists() {
                bail!("HOME partition not found");
            }
            fs::create_dir_all("/mnt/root/overlay").ok();
            fs::create_dir_all("/mnt/root/home").ok();
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
        fs::create_dir_all(path).ok();
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
        fs::create_dir_all("/mnt/root/etc/declare").ok();
        fs::write("/mnt/root/etc/declare/flatpak", flatpaks)?;

        if !Path::new("/mnt/root/usr/bin/rc-service").exists() {
            chroot("systemctl enable declareflatpak")?;
        } else {
            chroot("rc-update add declareflatpak")?;
        }
    }

    Ok(())
}

fn get_input(prompt: &str, default: &str) -> String {
    print!("{}. Valid options are {}\n{}[{}]{}: ", 
           prompt, "", Colours::BLUE, default, Colours::ENDC);
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    
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

    // Validate repository accessibility
    if !check_url(&config.repository).await {
        die("Cannot access the Xenia Linux repository");
    }

    // RegicideOS only supports cosmic-desktop flavour
    const REGICIDE_FLAVOUR: &str = "cosmic-fedora";
    if config.flavour.is_empty() {
        config.flavour = REGICIDE_FLAVOUR.to_string();
    } else if config.flavour != REGICIDE_FLAVOUR {
        if interactive {
            warn(&format!("RegicideOS only supports the cosmic-desktop flavour. Using: {}", REGICIDE_FLAVOUR));
            config.flavour = REGICIDE_FLAVOUR.to_string();
        } else {
            die(&format!("RegicideOS only supports the cosmic-desktop flavour: {}", REGICIDE_FLAVOUR));
        }
    }

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
            config.filesystem = get_input("Enter filesystem", "btrfs");
        } else {
            die("Invalid or missing filesystem in config");
        }
    }

    // Validate username
    if !check_username(&config.username) {
        if interactive {
            config.username = get_input("Enter username (leave empty for none)", "");
        } else {
            die("Invalid username in config");
        }
    }

    // Validate applications
    let package_sets = get_package_sets();
    if config.applications.is_empty() || !package_sets.contains(&config.applications) {
        if interactive {
            println!("Available package sets: {:?}", package_sets);
            config.applications = get_input("Enter applications set", "recommended");
        } else {
            die("Invalid or missing applications in config");
        }
    }

    Ok(config)
}

fn cleanup_on_failure() {
    warn("Cleaning up due to installation failure...");
    
    // Unmount filesystems
    let _ = execute("umount -R /mnt/root 2>/dev/null");
    let _ = execute("umount /mnt/gentoo 2>/dev/null");
    let _ = execute("umount /mnt/temp_btrfs 2>/dev/null");
    
    // Close LUKS devices
    let _ = execute("cryptsetup close xenia 2>/dev/null");
    
    // Remove temporary directories
    let _ = fs::remove_dir_all("/mnt/temp_btrfs");
    let _ = fs::remove_dir_all("/mnt/gentoo");
    let _ = fs::remove_dir_all("/mnt/root");
    
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

        let config_content = fs::read_to_string(config_path)?;
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
