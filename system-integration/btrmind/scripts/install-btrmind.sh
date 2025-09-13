#!/bin/bash
# RegicideOS BtrMind Installation Script
# Installs and configures BtrMind AI storage optimization agent

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BTRMIND_USER="btrmind"
BTRMIND_GROUP="btrmind"
BTRMIND_HOME="/var/lib/btrmind"
BTRMIND_LOG_DIR="/var/log/btrmind"
BTRMIND_CONFIG_DIR="/etc/btrmind"
BTRMIND_SOCKET_DIR="/run/btrmind"

log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
    exit 1
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

check_requirements() {
    log "Checking requirements..."

    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        error "This script must be run as root"
    fi

    # Check for BTRFS support
    if ! command -v btrfs >/dev/null 2>&1; then
        error "btrfs-progs is required but not installed"
    fi

    # Check for systemd
    if ! command -v systemctl >/dev/null 2>&1; then
        error "systemd is required but not available"
    fi

    # Check if BTRFS filesystem is mounted
    if ! findmnt -t btrfs / >/dev/null 2>&1; then
        warn "No BTRFS filesystem found at root. BtrMind will still install but may not be useful."
    fi

    success "Requirements check passed"
}

create_user_and_directories() {
    log "Creating user and directories..."

    # Create system user and group
    if ! getent group "$BTRMIND_GROUP" >/dev/null; then
        groupadd -r "$BTRMIND_GROUP"
        success "Created group: $BTRMIND_GROUP"
    fi

    if ! getent passwd "$BTRMIND_USER" >/dev/null; then
        useradd -r -g "$BTRMIND_GROUP" -d "$BTRMIND_HOME" -s /sbin/nologin \
            -c "RegicideOS BtrMind AI Agent" "$BTRMIND_USER"
        success "Created user: $BTRMIND_USER"
    fi

    # Create directories
    for dir in "$BTRMIND_HOME" "$BTRMIND_LOG_DIR" "$BTRMIND_CONFIG_DIR" "$BTRMIND_SOCKET_DIR"; do
        mkdir -p "$dir"
        chown -R "$BTRMIND_USER:$BTRMIND_GROUP" "$dir"
        chmod 750 "$dir"
    done

    # Create tmpfiles.d configuration for socket directory
    cat > /usr/lib/tmpfiles.d/btrmind.conf << EOF
d /run/btrmind 0755 $BTRMIND_USER $BTRMIND_GROUP -
EOF

    success "Created directories and permissions"
}

install_files() {
    log "Installing BtrMind files..."

    # Install binary (assuming it's built and available)
    BTRMIND_BIN="/usr/bin/btrmind"
    if [[ -f "target/release/btrmind" ]]; then
        cp target/release/btrmind "$BTRMIND_BIN"
        chmod 755 "$BTRMIND_BIN"
        success "Installed BtrMind binary"
    else
        warn "BtrMind binary not found. Please build it first with: cargo build --release"
    fi

    # Install systemd service files
    if [[ -f "systemd/btrmind.service" ]]; then
        cp systemd/btrmind.service /etc/systemd/system/
        cp systemd/btrmind.socket /etc/systemd/system/
        systemctl daemon-reload
        success "Installed systemd service files"
    fi

    # Install configuration
    if [[ -f "config/btrmind.toml" ]]; then
        cp config/btrmind.toml "$BTRMIND_CONFIG_DIR/"
        chown "$BTRMIND_USER:$BTRMIND_GROUP" "$BTRMIND_CONFIG_DIR/btrmind.toml"
        chmod 640 "$BTRMIND_CONFIG_DIR/btrmind.toml"
        success "Installed configuration file"
    fi

    # Install logrotate configuration
    cat > /etc/logrotate.d/btrmind << EOF
$BTRMIND_LOG_DIR/*.log {
    daily
    missingok
    rotate 7
    compress
    delaycompress
    notifempty
    create 0640 $BTRMIND_USER $BTRMIND_GROUP
    postrotate
        systemctl reload btrmind || true
    endscript
}
EOF

    success "Installed logrotate configuration"
}

configure_system() {
    log "Configuring system integration..."

    # Create sysctl configuration for BTRFS optimization
    cat > /etc/sysctl.d/99-btrmind.conf << EOF
# BtrMind BTRFS optimization settings
vm.swappiness = 10
vm.vfs_cache_pressure = 50
EOF

    # Apply sysctl settings
    sysctl -p /etc/sysctl.d/99-btrmind.conf >/dev/null || true

    # Create udev rules for BTRFS devices
    cat > /etc/udev/rules.d/99-btrfs.rules << EOF
# BtrMind BTRFS optimization rules
ACTION=="add|change", SUBSYSTEM=="block", ENV{ID_FS_TYPE}=="btrfs", RUN+="/usr/bin/btrfs filesystem resize max /"
EOF

    # Reload udev rules
    udevadm control --reload-rules

    # Add BtrMind user to required groups
    usermod -a -G disk "$BTRMIND_USER"
    usermod -a -G systemd-journal "$BTRMIND_USER"

    success "System configuration complete"
}

setup_monitoring() {
    log "Setting up monitoring..."

    # Create systemd monitoring timer
    cat > /etc/systemd/system/btrmind-monitor.timer << EOF
[Unit]
Description=RegicideOS BtrMind Monitoring Timer
Documentation=man:btrmind(1)
Requires=btrmind.service

[Timer]
OnBootSec=5min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
EOF

    # Create monitoring service
    cat > /etc/systemd/system/btrmind-monitor.service << EOF
[Unit]
Description=RegicideOS BtrMind Monitoring Service
Documentation=man:btrmind(1)
After=btrmind.service
Requires=btrmind.service

[Service]
Type=oneshot
User=$BTRMIND_USER
Group=$BTRMIND_GROUP
ExecStart=/usr/bin/btrmind monitor
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload

    success "Monitoring setup complete"
}

install_manpage() {
    log "Installing manpage..."

    # Create manpage directory
    mkdir -p /usr/share/man/man1

    # Install manpage (create a basic one if it doesn't exist)
    cat > /usr/share/man/man1/btrmind.1 << 'EOF'
.TH BTRMIND 1 "January 2025" "RegicideOS" "System Administration"
.SH NAME
btrmind \- AI-powered BTRFS storage optimization agent
.SH SYNOPSIS
.B btrmind
.RI [COMMAND]
.RI [OPTIONS]
.SH DESCRIPTION
BtrMind is an AI-powered storage optimization agent designed for RegicideOS.
It monitors BTRFS filesystems and performs intelligent cleanup and optimization
using reinforcement learning.
.SH COMMANDS
.TP
.B run
Start the BtrMind daemon in the background.
.TP
.B analyze
Analyze current storage state and provide recommendations.
.TP
.B cleanup
Perform cleanup operations to free up disk space.
.TP
.B stats
Display learning statistics and performance metrics.
.TP
.B config
Validate configuration and show current settings.
.SH OPTIONS
.TP
.B \-\-config PATH
Use configuration file at PATH instead of the default.
.TP
.B \-\-dry\-run
Show what would be done without actually doing it.
.TP
.B \-\-verbose
Enable verbose logging output.
.TP
.B \-\-help
Show help message and exit.
.SH FILES
.TP
.I /etc/btrmind/config.toml
Main configuration file.
.TP
.I /var/lib/btrmind/
Data directory for AI models and learning data.
.TP
.I /var/log/btrmind/
Log file directory.
.TP
.I /run/btrmind/
Socket directory for communication.
.SH EXAMPLES
Start BtrMind daemon:
.RS
.EX
btrmind run
.EE
.RE
Analyze current storage:
.RS
.EX
btrmind analyze
.EE
.RE
Perform aggressive cleanup:
.RS
.EX
btrmind cleanup --aggressive
.EE
.RE
.SH "SEE ALSO"
.BR btrfs (8),
.BR systemd (1)
.SH AUTHOR
RegicideOS Team <https://regicideos.com>
EOF

    gzip /usr/share/man/man1/btrmind.1

    success "Manpage installed"
}

enable_service() {
    log "Enabling BtrMind service..."

    # Enable and start the service
    systemctl enable btrmind.service
    systemctl enable btrmind.socket
    systemctl enable btrmind-monitor.timer

    # Start the service
    systemctl start btrmind.socket
    systemctl start btrmind.service

    # Wait a moment for service to start
    sleep 2

    # Check if service started successfully
    if systemctl is-active --quiet btrmind.service; then
        success "BtrMind service started successfully"
    else
        warn "BtrMind service failed to start. Check logs with: journalctl -u btrmind"
    fi
}

verify_installation() {
    log "Verifying installation..."

    # Check if binary exists
    if [[ -x "/usr/bin/btrmind" ]]; then
        success "BtrMind binary is installed"
    else
        error "BtrMind binary not found"
    fi

    # Check if service is running
    if systemctl is-active --quiet btrmind.service; then
        success "BtrMind service is running"
    else
        warn "BtrMind service is not running"
    fi

    # Check configuration
    if [[ -f "$BTRMIND_CONFIG_DIR/btrmind.toml" ]]; then
        success "Configuration file exists"

        # Validate configuration
        if /usr/bin/btrmind config >/dev/null 2>&1; then
            success "Configuration is valid"
        else
            warn "Configuration validation failed"
        fi
    else
        error "Configuration file not found"
    fi

    # Test basic functionality
    if /usr/bin/btrmind analyze >/dev/null 2>&1; then
        success "BtrMind is functional"
    else
        warn "BtrMind basic functionality test failed"
    fi
}

main() {
    log "Starting RegicideOS BtrMind installation..."

    check_requirements
    create_user_and_directories
    install_files
    configure_system
    setup_monitoring
    install_manpage
    enable_service
    verify_installation

    success "BtrMind installation completed!"
    echo
    echo "🎉 BtrMind AI storage optimization agent has been installed successfully!"
    echo
    echo "Next steps:"
    echo "  1. Monitor the service: systemctl status btrmind"
    echo "  2. View logs: journalctl -u btrmind -f"
    echo "  3. Check storage: btrmind analyze"
    echo "  4. View stats: btrmind stats"
    echo
    echo "Configuration file: $BTRMIND_CONFIG_DIR/btrmind.toml"
    echo "Log files: $BTRMIND_LOG_DIR/"
    echo "Data directory: $BTRMIND_HOME/"
    echo
}

# Run main function
main "$@"