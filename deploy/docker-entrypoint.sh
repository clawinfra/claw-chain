#!/bin/sh
# docker-entrypoint.sh — ClawChain Validator Node Entrypoint
#
# Handles:
#   1. Chain spec validation
#   2. Network key generation (libp2p peer identity)
#   3. Automatic session key generation (if AUTO_KEY_GEN=true and keystore empty)
#   4. Bootnode construction from BOOTNODES or TESTNET_BOOTNODE
#   5. exec into clawchain-node (tini stays as PID 1)
#
# Environment variables (all have defaults via Dockerfile ENV):
#   NODE_NAME        Human-readable node name                  [ClawChain-Validator]
#   CHAIN_SPEC       Built-in name (dev|local) or file path    [dev]
#   BASE_PATH        Data directory                            [/data]
#   P2P_PORT         P2P networking port                       [30333]
#   RPC_PORT         RPC/WebSocket port                        [9944]
#   METRICS_PORT     Prometheus metrics port                   [9615]
#   RUST_LOG         Log level                                 [info]
#   AUTO_KEY_GEN     Auto-generate session keys if empty       [false]
#   BOOTNODES        Comma-separated multiaddr bootnode list   []
#   TESTNET_BOOTNODE Pre-configured testnet bootnode           [see below]
#   EXTRA_ARGS       Extra CLI flags passed verbatim           []

set -e

# ── Defaults ────────────────────────────────────────────────────────────────
NODE_NAME="${NODE_NAME:-ClawChain-Validator}"
CHAIN_SPEC="${CHAIN_SPEC:-dev}"
BASE_PATH="${BASE_PATH:-/data}"
P2P_PORT="${P2P_PORT:-30333}"
RPC_PORT="${RPC_PORT:-9944}"
METRICS_PORT="${METRICS_PORT:-9615}"
RUST_LOG="${RUST_LOG:-info}"
AUTO_KEY_GEN="${AUTO_KEY_GEN:-false}"
BOOTNODES="${BOOTNODES:-}"
# Default testnet bootnode (Alice's peer ID on 135.181.157.121)
TESTNET_BOOTNODE="${TESTNET_BOOTNODE:-/ip4/135.181.157.121/tcp/30333/p2p/12D3KooWAkZ8Jmv13cdhogi6ESdZ5QVYrVyzosF2oSHMiW8ymZj6}"
EXTRA_ARGS="${EXTRA_ARGS:-}"

# ── Logging helpers ──────────────────────────────────────────────────────────
info()  { printf '[entrypoint] INFO:  %s\n' "$*" >&2; }
warn()  { printf '[entrypoint] WARN:  %s\n' "$*" >&2; }
error() { printf '[entrypoint] ERROR: %s\n' "$*" >&2; }

info "Starting ClawChain validator entrypoint..."
info "  NODE_NAME   = ${NODE_NAME}"
info "  CHAIN_SPEC  = ${CHAIN_SPEC}"
info "  BASE_PATH   = ${BASE_PATH}"
info "  AUTO_KEY_GEN= ${AUTO_KEY_GEN}"

# ── Step 1: Validate chain spec ──────────────────────────────────────────────
# If CHAIN_SPEC looks like a file path (starts with / or ./), verify it exists
case "${CHAIN_SPEC}" in
  /*|./*)
    if [ ! -f "${CHAIN_SPEC}" ]; then
      error "Chain spec file not found: ${CHAIN_SPEC}"
      error "Mount the spec file or use a built-in name (dev|local)."
      exit 1
    fi
    info "Chain spec file verified: ${CHAIN_SPEC}"
    ;;
  dev|local)
    info "Using built-in chain spec: ${CHAIN_SPEC}"
    ;;
  *)
    # Could be a named chain spec — pass through, node will validate
    info "Using chain spec: ${CHAIN_SPEC}"
    ;;
esac

# ── Step 2: Generate network key (libp2p peer identity) if missing ─────────
# The network key is stored at: <BASE_PATH>/chains/<chain_name>/network/secret_ed25519
# Without it, the node will fail to start with NetworkKeyNotFound error.
# Derive chain-specific directory name from CHAIN_SPEC (dev -> clawchain_dev, local -> clawchain_local)
CHAIN_DIR_NAME=""
if [ "${CHAIN_SPEC}" = "dev" ]; then
  CHAIN_DIR_NAME="clawchain_dev"
elif [ "${CHAIN_SPEC}" = "local" ]; then
  CHAIN_DIR_NAME="clawchain_local"
elif [ "${CHAIN_SPEC}" = "testnet" ]; then
  CHAIN_DIR_NAME="clawchain_testnet"
else
  # For custom chain specs, use the basename without extension
  CHAIN_DIR_NAME=$(basename "${CHAIN_SPEC}" .json)
fi

NETWORK_KEY_PATH="${BASE_PATH}/chains/${CHAIN_DIR_NAME}/network/secret_ed25519"

if [ ! -f "${NETWORK_KEY_PATH}" ]; then
  info "Network key not found at ${NETWORK_KEY_PATH} — generating new libp2p peer identity..."

  # Create network directory
  NETWORK_DIR="${BASE_PATH}/chains/${CHAIN_DIR_NAME}/network"
  mkdir -p "${NETWORK_DIR}"

  # Use the node binary to generate a properly-formatted ed25519 network key.
  # 'key generate-node-key' writes the secret key to --file and prints the peer ID to stdout.
  # This is the only reliable way to produce a key Substrate accepts — raw openssl/urandom
  # bytes do NOT match Substrate's expected key format and cause NetworkKeyNotFound errors.
  if clawchain-node key generate-node-key --file "${NETWORK_KEY_PATH}" >/dev/null 2>&1; then
    chmod 600 "${NETWORK_KEY_PATH}"
    info "Generated new network key at ${NETWORK_KEY_PATH}"
  else
    # Fallback for older node builds that don't support 'key generate-node-key --file'
    # Try writing to stdout and capturing
    PEER_ID=$(clawchain-node key generate-node-key 2>"${NETWORK_KEY_PATH}" || true)
    if [ -s "${NETWORK_KEY_PATH}" ]; then
      chmod 600 "${NETWORK_KEY_PATH}"
      info "Generated new network key at ${NETWORK_KEY_PATH} (peer ID: ${PEER_ID})"
    else
      error "Failed to generate network key — clawchain-node key generate-node-key not supported"
      error "Try passing --node-key via EXTRA_ARGS or mounting a pre-generated key"
      exit 1
    fi
  fi
else
  info "Network key exists at ${NETWORK_KEY_PATH}"
fi

# ── Step 3: Auto session key generation ─────────────────────────────────────
if [ "${AUTO_KEY_GEN}" = "true" ]; then
  # Determine keystore path — Substrate stores keys under:
  # <BASE_PATH>/chains/<chain_name>/keystore/
  # We search for any keystore directory under BASE_PATH
  KEYSTORE_DIR=""
  # Look for existing keystore directories
  for candidate in "${BASE_PATH}/chains/"*/keystore; do
    if [ -d "${candidate}" ]; then
      KEYSTORE_DIR="${candidate}"
      break
    fi
  done

  if [ -n "${KEYSTORE_DIR}" ] && [ "$(ls -A "${KEYSTORE_DIR}" 2>/dev/null)" ]; then
    info "Keystore already populated at ${KEYSTORE_DIR} — skipping key generation."
  else
    info "Keystore is empty — generating session keys..."

    # Generate Aura key (sr25519)
    info "Generating Aura sr25519 key..."
    AURA_OUTPUT=$(clawchain-node key generate --scheme sr25519 --words 24 2>&1)
    AURA_SECRET=$(printf '%s\n' "${AURA_OUTPUT}" | grep "Secret seed" | awk '{print $NF}')
    AURA_PUBLIC=$(printf '%s\n' "${AURA_OUTPUT}" | grep "SS58 Address" | awk '{print $NF}')

    if [ -z "${AURA_SECRET}" ]; then
      error "Failed to generate Aura key. Output: ${AURA_OUTPUT}"
      exit 1
    fi

    # Generate GRANDPA key (ed25519)
    info "Generating GRANDPA ed25519 key..."
    GRAN_OUTPUT=$(clawchain-node key generate --scheme ed25519 --words 24 2>&1)
    GRAN_SECRET=$(printf '%s\n' "${GRAN_OUTPUT}" | grep "Secret seed" | awk '{print $NF}')
    GRAN_PUBLIC=$(printf '%s\n' "${GRAN_OUTPUT}" | grep "SS58 Address" | awk '{print $NF}')

    if [ -z "${GRAN_SECRET}" ]; then
      error "Failed to generate GRANDPA key. Output: ${GRAN_OUTPUT}"
      exit 1
    fi

    # Insert Aura key into keystore
    info "Inserting Aura key into keystore..."
    clawchain-node key insert \
      --base-path "${BASE_PATH}" \
      --chain "${CHAIN_SPEC}" \
      --scheme sr25519 \
      --suri "${AURA_SECRET}" \
      --key-type aura

    # Insert GRANDPA key into keystore
    info "Inserting GRANDPA key into keystore..."
    clawchain-node key insert \
      --base-path "${BASE_PATH}" \
      --chain "${CHAIN_SPEC}" \
      --scheme ed25519 \
      --suri "${GRAN_SECRET}" \
      --key-type gran

    # Print public keys — operator needs these to register as validator
    info "═══════════════════════════════════════════════════════════════"
    info "  SESSION KEYS GENERATED — copy these for validator registration"
    info "  Aura  (sr25519): ${AURA_PUBLIC}"
    info "  GRANDPA (ed25519): ${GRAN_PUBLIC}"
    info "  Store the secret seeds securely — they are NOT saved to disk."
    info "═══════════════════════════════════════════════════════════════"

    # Clear secrets from environment (belt-and-suspenders)
    unset AURA_SECRET GRAN_SECRET AURA_OUTPUT GRAN_OUTPUT
  fi
fi

# ── Step 4: Build bootnode arguments ─────────────────────────────────────────
BOOTNODE_ARGS=""

if [ -n "${BOOTNODES}" ]; then
  # BOOTNODES is comma-separated: split and build --bootnodes flags
  OLD_IFS="${IFS}"
  IFS=","
  for bn in ${BOOTNODES}; do
    # Trim whitespace
    bn=$(printf '%s' "${bn}" | tr -d ' ')
    if [ -n "${bn}" ]; then
      BOOTNODE_ARGS="${BOOTNODE_ARGS} --bootnodes ${bn}"
    fi
  done
  IFS="${OLD_IFS}"
  info "Using BOOTNODES: ${BOOTNODES}"
elif [ -n "${TESTNET_BOOTNODE}" ] && [ "${CHAIN_SPEC}" != "dev" ]; then
  # Use testnet bootnode when not on a pure dev chain
  BOOTNODE_ARGS="--bootnodes ${TESTNET_BOOTNODE}"
  info "Using TESTNET_BOOTNODE: ${TESTNET_BOOTNODE}"
fi

# ── Step 5: Exec into clawchain-node ─────────────────────────────────────────
# Use exec so tini (PID 1) receives signals from clawchain-node directly.
# shellcheck disable=SC2086
info "Launching clawchain-node..."
exec clawchain-node \
  --validator \
  --base-path "${BASE_PATH}" \
  --chain "${CHAIN_SPEC}" \
  --name "${NODE_NAME}" \
  --port "${P2P_PORT}" \
  --rpc-port "${RPC_PORT}" \
  --unsafe-rpc-external \
  --rpc-cors all \
  --rpc-methods unsafe \
  --prometheus-external \
  --prometheus-port "${METRICS_PORT}" \
  --state-pruning 256 \
  --blocks-pruning archive-canonical \
  --database paritydb \
  --log "${RUST_LOG}" \
  ${BOOTNODE_ARGS} \
  ${EXTRA_ARGS}
