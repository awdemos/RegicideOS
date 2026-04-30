# RegicideOS Overlay Installation

The RegicideOS overlay provides AI-powered system management tools and enhanced development support for RegicideOS systems.

**Warning**: The overlay is only available on Gentoo-based systems (including RegicideOS and Xenia Linux).

## RegicideOS Specific Steps

If RegicideOS is being used, first sync the Gentoo repositories:

> **Note**: The command `emerge-webrsync` is used here as it will fix issues with using emerge in the next step.

```bash
emerge-webrsync
```

Next, mount `/usr` as read-write:

```bash
mount -o rw,remount /usr
```

## Installing eselect-repository

Install `eselect-repository`:

```bash
emerge -va eselect-repository
```

## Add the Overlay

First, add the RegicideOS overlay:

```bash
eselect repository add regicide-overlay git https://github.com/awdemos/regicide-overlay
```

And sync it:

```bash
emaint sync --repo regicide-overlay
```

Done!

## Allowing Live Ebuilds

Most RegicideOS tools are currently available as live ebuilds.

To allow `emerge` to use these, add the following to `/etc/portage/package.accept_keywords/regicide`:

```bash
regicide-tools/* **
```

## Installing RegicideOS Tools

### BtrMind AI Storage Agent

```bash
# Install BtrMind
emerge regicide-tools/btrmind

# Enable and start the service
systemctl enable btrmind
systemctl start btrmind

# Test the installation
btrmind analyze
```

### RegicideOS Installer

```bash
# Install the installer (for system deployment)
emerge regicide-tools/regicide-installer

# The installer will be available as 'regicide-installer'
```

### Future AI Tools

```bash
# PortCL will be available when implemented
# emerge regicide-tools/portcl
```

## Configuration

### BtrMind Configuration

Edit `/etc/btrmind/config.toml`:

```toml
[monitoring]
target_path = "/"
poll_interval = 60

[thresholds]
warning_level = 85.0
critical_level = 95.0
emergency_level = 98.0

[actions]
enable_temp_cleanup = true
enable_compression = true
enable_snapshot_cleanup = true
```

### System Integration

The AI agents integrate with RegicideOS's immutable system:

```bash
# Check AI agent status
systemctl status btrmind

# View learning progress
btrmind stats

# Manual operations
btrmind cleanup --aggressive
```

## Verification

Verify the overlay is working:

```bash
# Check overlay is active
eselect repository list | grep regicide

# Verify package sources
equery which regicide-tools/btrmind

# Test package functionality
btrmind --help
```

## Troubleshooting

### Overlay not found
```bash
# Re-add overlay
eselect repository remove regicide-overlay
eselect repository add regicide-overlay git https://github.com/awdemos/regicide-overlay

# Force sync
emaint sync --repo regicide-overlay
```

### Package installation fails
```bash
# Check dependencies
emerge --pretend regicide-tools/btrmind

# Check USE flags
emerge --info | grep USE

# Enable required keywords
echo "regicide-tools/btrmind **" >> /etc/portage/package.accept_keywords/regicide
```

### AI service issues
```bash
# Check service logs
journalctl -u btrmind -f

# Reset AI learning
systemctl stop btrmind  
rm /var/lib/btrmind/model.json
systemctl start btrmind

# Validate configuration
btrmind config
```

## Development

### Testing Local Changes

```bash
# Create local overlay
mkdir -p /usr/local/portage/regicide-dev

# Copy modified ebuilds
cp regicide-tools/btrmind/btrmind-9999.ebuild /usr/local/portage/regicide-dev/

# Test installation
PORTDIR_OVERLAY="/usr/local/portage/regicide-dev" emerge btrmind
```

### Contributing

1. Fork the overlay repository
2. Make changes following Gentoo ebuild guidelines
3. Test changes locally
4. Submit pull request

See [Development Guide](https://github.com/awdemos/RegicideOS/blob/main/DEVELOPMENT_ROADMAP.md) for details.
