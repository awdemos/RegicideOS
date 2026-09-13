use anyhow::{bail, Context, Result};
use std::process::{Command as ProcessCommand, Output};

/// Run an external command with an explicit argv array (no shell interpolation).
///
/// This is the preferred way to invoke binaries from the installer: every
/// argument is passed verbatim, so values containing spaces, quotes, or shell
/// metacharacters cannot alter the command semantics.
pub fn run_cmd(program: &str, args: &[&str]) -> Result<Output> {
    ProcessCommand::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Failed to spawn {program}"))
}

/// Run a command and return its stdout as a string, failing on non-zero exit.
#[cfg_attr(not(test), allow(dead_code))]
pub fn run_cmd_output(program: &str, args: &[&str]) -> Result<String> {
    let output = run_cmd(program, args)?;
    if !output.status.success() {
        bail!(
            "{} failed ({}): {}",
            program,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a command inside /mnt/root with explicit argv arrays.
pub fn chroot_cmd(program: &str, args: &[&str]) -> Result<()> {
    let mut chroot_args = vec!["/mnt/root", program];
    chroot_args.extend(args);
    let output = run_cmd("chroot", &chroot_args)?;
    if !output.status.success() {
        bail!(
            "chroot {} failed ({}): {}",
            program,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Run a command inside /mnt/root and return its stdout, failing on non-zero exit.
#[cfg_attr(not(test), allow(dead_code))]
pub fn chroot_cmd_output(program: &str, args: &[&str]) -> Result<String> {
    let mut chroot_args = vec!["/mnt/root", program];
    chroot_args.extend(args);
    let output = run_cmd("chroot", &chroot_args)?;
    if !output.status.success() {
        bail!(
            "chroot {} failed ({}): {}",
            program,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg_attr(not(test), allow(dead_code))]
/// Escape a value for safe use as a filesystem label.
///
/// Labels are restricted to a small safe character set to avoid injection into
/// mkfs.*, cryptsetup, and other tools that parse shell-style strings.
pub fn safe_label(label: &str) -> Result<String> {
    if label.is_empty() {
        bail!("Label cannot be empty");
    }
    if label.len() > 64 {
        bail!("Label too long (max 64 characters)");
    }
    if label.contains('\0') || label.starts_with('-') {
        bail!("Invalid label: contains unsafe characters");
    }
    if !label
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ' ')
    {
        bail!("Label contains unsafe characters: {}", label);
    }
    Ok(label.to_string())
}

/// Build a sgdisk argument vector for creating a partition.
///
/// All values are passed as separate argv entries so partition labels with
/// spaces or punctuation are handled safely by sgdisk itself.
pub fn sgdisk_new_args(
    part_num: u32,
    size: &str,
    typecode: &str,
    label: Option<&str>,
    drive: &str,
) -> Vec<String> {
    let mut args = vec![
        format!("--new={part_num}:{size}"),
        format!("--typecode={part_num}:{typecode}"),
    ];
    if let Some(l) = label {
        if !l.is_empty() {
            args.push(format!("--change-name={part_num}:{l}"));
        }
    }
    args.push(drive.to_string());
    args
}

/// Build an mkfs argument vector with an optional label.
pub fn mkfs_args(program: &str, device: &str, label: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();
    match program {
        "mkfs.vfat" => args.push("-F".to_string()),
        "mkfs.ext4" => args.push("-F".to_string()),
        "mkfs.btrfs" => {}
        _ => {}
    }
    if let Some(l) = label {
        if !l.is_empty() {
            match program {
                "mkfs.vfat" => {
                    args.push("-n".to_string());
                    args.push(l.to_string());
                }
                "mkfs.ext4" | "mkfs.btrfs" => {
                    args.push("-L".to_string());
                    args.push(l.to_string());
                }
                _ => {}
            }
        }
    }
    args.push(device.to_string());
    args
}
