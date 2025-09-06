#!/bin/bash
# BtrMind Installation Script

set -euo pipefail

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/btrmind"
DATA_DIR="/var/lib/btrmind"
SERVICE_FILE="/etc/systemd/system/btrmind.service"
USER="btrmind"
GROUP="btrmind"

echo "Installing BtrMind AI Storage Agent..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root" 
   exit 1
fi

# Build the project
echo "Building BtrMind..."
cargo build --release

# Create system user
if ! id "$USER" &>/dev/null; then
    echo "Creating user $USER..."
    useradd --system --no-create-home --shell /bin/false "$USER"
fi

# Create directories
echo "Creating directories..."
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR"
chown "$USER:$GROUP" "$DATA_DIR"
chmod 750 "$DATA_DIR"

# Install binary
echo "Installing binary..."
cp target/release/btrmind "$INSTALL_DIR/"
chmod 755 "$INSTALL_DIR/btrmind"

# Install configuration
echo "Installing configuration..."
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    cp config/btrmind.toml "$CONFIG_DIR/config.toml"
    chmod 644 "$CONFIG_DIR/config.toml"
else
    echo "Configuration already exists, not overwriting"
fi

# Install systemd service
echo "Installing systemd service..."
cp systemd/btrmind.service "$SERVICE_FILE"
chmod 644 "$SERVICE_FILE"

# Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload

echo "Installation complete!"
echo ""
echo "To start BtrMind:"
echo "  sudo systemctl enable btrmind"
echo "  sudo systemctl start btrmind"
echo ""
echo "To check status:"
echo "  sudo systemctl status btrmind"
echo "  sudo journalctl -u btrmind -f"
echo ""
echo "To test configuration:"
echo "  btrmind config"
echo ""
echo "To run manual analysis:"
echo "  btrmind analyze"
