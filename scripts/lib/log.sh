#!/bin/bash
# Shared logging helpers for RegicideOS scripts.
# Source this file: source "${SCRIPT_DIR}/lib/log.sh"

REGICIDE_RED='\033[0;31m'
REGICIDE_GREEN='\033[0;32m'
REGICIDE_YELLOW='\033[1;33m'
REGICIDE_BLUE='\033[0;34m'
REGICIDE_NC='\033[0m'

regicide_log() {
    local level=$1
    shift
    printf '%b[%s]%b %s\n' "${REGICIDE_BLUE}" "$level" "${REGICIDE_NC}" "$*"
}

regicide_success() {
    printf '%b[SUCCESS]%b %s\n' "${REGICIDE_GREEN}" "${REGICIDE_NC}" "$*"
}

regicide_warn() {
    printf '%b[WARN]%b %s\n' "${REGICIDE_YELLOW}" "${REGICIDE_NC}" "$*"
}

regicide_error() {
    printf '%b[ERROR]%b %s\n' "${REGICIDE_RED}" "${REGICIDE_NC}" "$*"
}
