#!/usr/bin/env bash
# RegicideOS Dependency Installation Script
# Installs all build and runtime dependencies for the RegicideOS installer
#
# Usage: ./scripts/install-dependencies.sh [--dry-run]

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

DRY_RUN=false

# Function to log messages
log() {
    local level=$1
    shift
    local message="$*"
    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    case $level in
        "INFO")  echo -e "${BLUE}[INFO]${NC}  $timestamp - $message" ;;
        "WARN")  echo -e "${YELLOW}[WARN]${NC}  $timestamp - $message" ;;
        "ERROR") echo -e "${RED}[ERROR]${NC} $timestamp - $message" ;;
        "SUCCESS") echo -e "${GREEN}[SUCCESS]${NC} $timestamp - $message" ;;
        *)      echo "[UNKNOWN] $timestamp - $message" ;;
    esac
}

# Detect Linux distribution
detect_distro() {
    if [[ -f /etc/os-release ]]; then
        # shellcheck source=/dev/null
        source /etc/os-release
        echo "$ID"
    elif command -v lsb_release &> /dev/null; then
        lsb_release -si | tr '[:upper:]' '[:lower:]'
    else
        echo "unknown"
    fi
}

# Detect package manager
detect_package_manager() {
    if command -v dnf &> /dev/null; then
        echo "dnf"
    elif command -v apt-get &> /dev/null; then
        echo "apt"
    elif command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v zypper &> /dev/null; then
        echo "zypper"
    elif command -v emerge &> /dev/null; then
        echo "emerge"
    else
        echo "unknown"
    fi
}

# Install packages using the detected package manager
install_packages() {
    local pm=$1
    shift
    local packages=("$@")

    if [[ "$DRY_RUN" == "true" ]]; then
        log "INFO" "[DRY-RUN] Would install: ${packages[*]}"
        return 0
    fi

    case "$pm" in
        dnf)
            log "INFO" "Installing packages with dnf..."
            sudo dnf install -y "${packages[@]}"
            ;;
        apt)
            log "INFO" "Installing packages with apt..."
            sudo apt-get update
            sudo apt-get install -y "${packages[@]}"
            ;;
        pacman)
            log "INFO" "Installing packages with pacman..."
            sudo pacman -S --noconfirm "${packages[@]}"
            ;;
        zypper)
            log "INFO" "Installing packages with zypper..."
            sudo zypper install -y "${packages[@]}"
            ;;
        emerge)
            log "INFO" "Installing packages with emerge..."
            sudo emerge -av "${packages[@]}"
            ;;
        *)
            log "ERROR" "Unknown package manager. Please install manually: ${packages[*]}"
            return 1
            ;;
    esac
}

# Get package names for a specific distro
get_packages_for_distro() {
    local distro=$1
    local pm=$2

    # Distro-specific package names
    case "$distro" in
        fedora|rhel|centos|rocky|alma)
            echo "git curl gcc gdisk btrfs-progs cryptsetup grub2-tools make python3"
            ;;
        debian|ubuntu|linuxmint|pop)
            echo "git curl gcc gdisk btrfs-progs cryptsetup-bin grub-common make python3"
            ;;
        arch|manjaro|endeavouros)
            echo "git curl gcc gptfdisk btrfs-progs cryptsetup grub make python"
            ;;
        opensuse*|suse*)
            echo "git curl gcc gdisk btrfsprogs cryptsetup grub2 make python3"
            ;;
        gentoo)
            echo "dev-vcs/git net-misc/curl sys-devel/gcc sys-apps/gptfdisk sys-fs/btrfs-progs sys-fs/cryptsetup sys-boot/grub dev-lang/python"
            ;;
        *)
            # Fallback: try common package names
            log "WARN" "Unknown distro '$distro'. Attempting with generic package names."
            case "$pm" in
                dnf)
                    echo "git curl gcc gdisk btrfs-progs cryptsetup grub2-tools make python3"
                    ;;
                apt)
                    echo "git curl gcc gdisk btrfs-progs cryptsetup-bin grub-common make python3"
                    ;;
                pacman)
                    echo "git curl gcc gptfdisk btrfs-progs cryptsetup grub make python"
                    ;;
                *)
                    log "ERROR" "Cannot determine package names for distro: $distro"
                    return 1
                    ;;
            esac
            ;;
    esac
}

# Install Rust toolchain via rustup
install_rust() {
    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        log "INFO" "Rust already installed: $(rustc --version)"
        return 0
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log "INFO" "[DRY-RUN] Would install Rust via rustup"
        return 0
    fi

    log "INFO" "Installing Rust via rustup..."

    if ! command -v rustup &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    fi

    rustup default stable
    rustup update stable

    log "SUCCESS" "Rust installed: $(rustc --version)"
}

# Validate that critical tools are available after installation
validate_installation() {
    log "INFO" "Validating installation..."

    local tools=(
        "git:Version control"
        "curl:Download tool"
        "gcc:C compiler"
        "sgdisk:Partitioning tool"
        "btrfs:BTRFS filesystem tools"
        "cryptsetup:LUKS encryption"
        "grub-install:Bootloader"
        "make:Build system"
        "python3:Python runtime"
        "cargo:Rust package manager"
        "rustc:Rust compiler"
    )

    local missing=()

    for tool_info in "${tools[@]}"; do
        local tool="${tool_info%%:*}"
        local desc="${tool_info##*:}"

        if ! command -v "$tool" &> /dev/null; then
            log "WARN" "Missing: $tool ($desc)"
            missing+=("$tool ($desc)")
        else
            log "INFO" "Found: $tool ($desc)"
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log "WARN" "Some tools are missing. You may need to install them manually:"
        for item in "${missing[@]}"; do
            log "WARN" "  - $item"
        done
        return 1
    fi

    log "SUCCESS" "All critical dependencies are installed"
    return 0
}

# Main function
main() {
    log "INFO" "RegicideOS Dependency Installer"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --help|-h)
                cat << 'EOF'
Usage: ./scripts/install-dependencies.sh [OPTIONS]

OPTIONS:
    --dry-run    Show what would be installed without actually installing
    --help       Show this help message

DESCRIPTION:
    Installs all build and runtime dependencies required to build
    and run the RegicideOS installer from source.

SUPPORTED DISTRIBUTIONS:
    Fedora, RHEL, CentOS, Rocky Linux, AlmaLinux
    Debian, Ubuntu, Linux Mint, Pop!_OS
    Arch Linux, Manjaro, EndeavourOS
    openSUSE, SUSE Linux Enterprise
    Gentoo

EXAMPLES:
    ./scripts/install-dependencies.sh
    ./scripts/install-dependencies.sh --dry-run
EOF
                exit 0
                ;;
            *)
                log "ERROR" "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Detect environment
    local distro
    distro=$(detect_distro)
    local pm
    pm=$(detect_package_manager)

    log "INFO" "Detected distribution: $distro"
    log "INFO" "Detected package manager: $pm"

    if [[ "$pm" == "unknown" ]]; then
        log "ERROR" "No supported package manager found (tried: dnf, apt, pacman, zypper, emerge)"
        log "ERROR" "Please install dependencies manually."
        exit 1
    fi

    # Get packages for this distro
    local packages
    packages=$(get_packages_for_distro "$distro" "$pm")
    if [[ $? -ne 0 ]]; then
        exit 1
    fi

    # Install system packages
    log "INFO" "Installing system packages..."
    if ! install_packages "$pm" $packages; then
        log "ERROR" "Failed to install system packages"
        exit 1
    fi

    # Install Rust toolchain
    install_rust

    # Validate
    if validate_installation; then
        log "SUCCESS" "All dependencies installed successfully!"
        log "INFO" ""
        log "INFO" "Next steps:"
        log "INFO" "  1. Clone the repository: git clone https://github.com/awdemos/RegicideOS.git"
        log "INFO" "  2. cd RegicideOS/installer"
        log "INFO" "  3. Build: cargo build --release"
        log "INFO" "  4. Run: sudo ./target/release/installer"
    else
        log "WARN" "Installation completed with warnings. Some tools may need manual installation."
        exit 1
    fi
}

# Error handling
trap 'log "ERROR" "Installation interrupted"; exit 1' INT TERM

# Run main
main "$@"
