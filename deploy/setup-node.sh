#!/usr/bin/env bash
# deploy/setup-node.sh — ClawChain PoA Validator Node Setup
#
# Creates the system user, directories, and installs the binary and systemd
# service for a ClawChain mainnet validator node.
#
# Usage:
#   sudo bash deploy/setup-node.sh [OPTIONS]
#
# Options:
#   --binary PATH       Path to the clawchain-node binary (required if not in $PATH)
#   --authorities PATH  Path to authorities.json (required for mainnet)
#   --name NAME         Validator node name (default: hostname)
#   --skip-service      Install binary only; do not install or enable the
#                       systemd service
#   --dry-run           Print what would be done without making changes
#
# Examples:
#   # Typical validator setup on a fresh VPS:
#   sudo bash deploy/setup-node.sh \
#     --binary ./target/release/clawchain-node \
#     --authorities ./authorities.json \
#     --name "my-validator-1"
#
#   # Dry-run to preview changes:
#   sudo bash deploy/setup-node.sh --dry-run

set -euo pipefail

# ─── Defaults ────────────────────────────────────────────────────────────────

CLAWCHAIN_USER="clawchain"
CLAWCHAIN_GROUP="clawchain"
BINARY_DEST="/usr/local/bin/clawchain-node"
DATA_DIR="/var/lib/clawchain"
KEYSTORE_DIR="${DATA_DIR}/keystore"
CONFIG_DIR="/etc/clawchain"
ENV_FILE="${CONFIG_DIR}/env"
SERVICE_NAME="clawchain"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

BINARY_SRC=""
AUTHORITIES_SRC=""
NODE_NAME="$(hostname -s)"
SKIP_SERVICE=false
DRY_RUN=false

# ─── Colours ─────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Colour

info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
die()   { error "$*"; exit 1; }

run() {
    if [[ "${DRY_RUN}" == "true" ]]; then
        echo -e "${YELLOW}[DRY-RUN]${NC} $*"
    else
        eval "$@"
    fi
}

# ─── Argument parsing ─────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)        BINARY_SRC="$2"; shift 2 ;;
        --authorities)   AUTHORITIES_SRC="$2"; shift 2 ;;
        --name)          NODE_NAME="$2"; shift 2 ;;
        --skip-service)  SKIP_SERVICE=true; shift ;;
        --dry-run)       DRY_RUN=true; shift ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \?//'
            exit 0
            ;;
        *)
            die "Unknown option: $1 (try --help)"
            ;;
    esac
done

# ─── Pre-flight checks ────────────────────────────────────────────────────────

info "ClawChain PoA Node Setup"
info "========================"

if [[ "${DRY_RUN}" == "false" ]] && [[ "${EUID}" -ne 0 ]]; then
    die "This script must be run as root. Try: sudo bash $0 $*"
fi

# Resolve binary
if [[ -z "${BINARY_SRC}" ]]; then
    if command -v clawchain-node &>/dev/null; then
        BINARY_SRC="$(command -v clawchain-node)"
        info "Using clawchain-node from PATH: ${BINARY_SRC}"
    else
        die "--binary is required (clawchain-node not found in PATH)"
    fi
fi

if [[ ! -f "${BINARY_SRC}" ]]; then
    die "Binary not found: ${BINARY_SRC}"
fi

info "Binary:       ${BINARY_SRC}"
info "Node name:    ${NODE_NAME}"
info "Data dir:     ${DATA_DIR}"
info "Config dir:   ${CONFIG_DIR}"
info "Service:      ${SERVICE_NAME}"
if [[ -n "${AUTHORITIES_SRC}" ]]; then
    info "Authorities:  ${AUTHORITIES_SRC}"
fi
echo

# ─── Create system user ───────────────────────────────────────────────────────

if id "${CLAWCHAIN_USER}" &>/dev/null; then
    info "User '${CLAWCHAIN_USER}' already exists"
else
    info "Creating system user '${CLAWCHAIN_USER}'..."
    run useradd \
        --system \
        --no-create-home \
        --shell /usr/sbin/nologin \
        --comment "ClawChain Validator Node" \
        "${CLAWCHAIN_USER}"
    ok "User '${CLAWCHAIN_USER}' created"
fi

# ─── Create directories ───────────────────────────────────────────────────────

for dir in "${DATA_DIR}" "${KEYSTORE_DIR}" "${CONFIG_DIR}"; do
    if [[ -d "${dir}" ]]; then
        info "Directory exists: ${dir}"
    else
        info "Creating: ${dir}"
        run mkdir -p "${dir}"
    fi
done

# Permissions: only the clawchain user can read the data and keystore dirs
run chown -R "${CLAWCHAIN_USER}:${CLAWCHAIN_GROUP}" "${DATA_DIR}"
run chmod 750 "${DATA_DIR}" "${KEYSTORE_DIR}"
run chown root:root "${CONFIG_DIR}"
run chmod 755 "${CONFIG_DIR}"
ok "Directories configured"

# ─── Install binary ───────────────────────────────────────────────────────────

info "Installing binary to ${BINARY_DEST}..."
run cp "${BINARY_SRC}" "${BINARY_DEST}"
run chmod 755 "${BINARY_DEST}"
ok "Binary installed: ${BINARY_DEST}"

# ─── Install authorities file ─────────────────────────────────────────────────

if [[ -n "${AUTHORITIES_SRC}" ]]; then
    if [[ ! -f "${AUTHORITIES_SRC}" ]]; then
        die "Authorities file not found: ${AUTHORITIES_SRC}"
    fi
    AUTH_DEST="${DATA_DIR}/authorities.json"
    info "Installing authorities file to ${AUTH_DEST}..."
    run cp "${AUTHORITIES_SRC}" "${AUTH_DEST}"
    run chown "${CLAWCHAIN_USER}:${CLAWCHAIN_GROUP}" "${AUTH_DEST}"
    run chmod 640 "${AUTH_DEST}"
    ok "Authorities file installed: ${AUTH_DEST}"
fi

# ─── Write environment file ───────────────────────────────────────────────────

if [[ -f "${ENV_FILE}" ]]; then
    warn "Environment file already exists: ${ENV_FILE} (not overwriting)"
else
    info "Writing environment file: ${ENV_FILE}..."
    run bash -c "cat > '${ENV_FILE}' <<'EOF'
# ClawChain validator environment
# Edit these values before starting the node.
# This file is NOT tracked in git — keep it safe.

# Path to the authorities.json file.
CLAWCHAIN_AUTHORITIES_FILE=${DATA_DIR}/authorities.json

# Human-readable name shown in telemetry and logs.
CLAWCHAIN_NODE_NAME=${NODE_NAME}

# Network ports
CLAWCHAIN_P2P_PORT=30333
CLAWCHAIN_RPC_PORT=9944
CLAWCHAIN_PROMETHEUS_PORT=9615

# Space-separated list of boot-node multiaddresses.
# Example: /ip4/1.2.3.4/tcp/30333/p2p/12D3Koo...
CLAWCHAIN_BOOTNODES=
EOF"
    run chown root:"${CLAWCHAIN_GROUP}" "${ENV_FILE}"
    run chmod 640 "${ENV_FILE}"
    ok "Environment file written: ${ENV_FILE}"
fi

# ─── Install systemd service ──────────────────────────────────────────────────

if [[ "${SKIP_SERVICE}" == "true" ]]; then
    info "Skipping service installation (--skip-service)"
else
    info "Installing systemd service..."

    UNIT_SRC="${SCRIPT_DIR}/clawchain.service"
    if [[ ! -f "${UNIT_SRC}" ]]; then
        die "Service unit not found: ${UNIT_SRC}"
    fi

    run cp "${UNIT_SRC}" "${SERVICE_FILE}"
    run chmod 644 "${SERVICE_FILE}"
    ok "Service file installed: ${SERVICE_FILE}"

    info "Reloading systemd daemon..."
    run systemctl daemon-reload

    info "Enabling service to start on boot..."
    run systemctl enable "${SERVICE_NAME}"
    ok "Service enabled"

    # Print instructions rather than starting automatically
    echo
    echo -e "${GREEN}================================================================${NC}"
    echo -e "${GREEN}Setup complete!${NC}"
    echo
    echo "Next steps:"
    echo
    echo "  1. Edit the environment file:"
    echo "       sudo nano ${ENV_FILE}"
    echo
    echo "  2. Insert your validator keys into the keystore:"
    echo "       # Generate keys (if you haven't already):"
    echo "       clawchain-node generate-keys --output /tmp/validator-keys.json"
    echo "       # Insert Aura key:"
    echo "       clawchain-node key insert \\"
    echo "         --base-path ${DATA_DIR}/data \\"
    echo "         --keystore-path ${KEYSTORE_DIR} \\"
    echo "         --chain mainnet \\"
    echo "         --scheme Sr25519 \\"
    echo "         --key-type aura \\"
    echo "         --suri '<your Aura secret phrase>'"
    echo "       # Insert GRANDPA key:"
    echo "       clawchain-node key insert \\"
    echo "         --base-path ${DATA_DIR}/data \\"
    echo "         --keystore-path ${KEYSTORE_DIR} \\"
    echo "         --chain mainnet \\"
    echo "         --scheme Ed25519 \\"
    echo "         --key-type gran \\"
    echo "         --suri '<your GRANDPA secret phrase>'"
    echo
    echo "  3. Start the node:"
    echo "       sudo systemctl start ${SERVICE_NAME}"
    echo
    echo "  4. Check the logs:"
    echo "       journalctl -u ${SERVICE_NAME} -f"
    echo
    echo "  5. Check Prometheus metrics:"
    echo "       curl http://localhost:9615/metrics"
    echo -e "${GREEN}================================================================${NC}"
fi

if [[ "${DRY_RUN}" == "true" ]]; then
    echo
    warn "Dry-run complete — no changes were made."
fi
