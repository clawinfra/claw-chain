#!/bin/bash
# scripts/setup-node.sh
# ClawChain Testnet Node Setup Script
# Sets up a ClawChain validator node using the staging chainspec (no --dev flag).
#
# Usage:
#   bash scripts/setup-node.sh
#
# Prerequisites:
#   - Built binary at /root/claw-chain/target/release/clawchain-node
#   - Repo cloned at /root/claw-chain
#   - systemd available

set -euo pipefail

BINARY="/root/claw-chain/target/release/clawchain-node"
CHAIN_SPEC_DIR="/root/claw-chain/chain-spec"
BASE_PATH="/root/.clawchain/testnet"
SERVICE_FILE="/etc/systemd/system/clawchain.service"

echo "=== ClawChain Testnet Node Setup ==="

# 1. Ensure binary exists
if [ ! -f "$BINARY" ]; then
  echo "ERROR: Binary not found at $BINARY. Build first with:"
  echo "  cargo build --release"
  exit 1
fi

# 2. Copy chainspec
echo "[1/5] Installing chainspec..."
mkdir -p "$CHAIN_SPEC_DIR"
cp "$(dirname "$0")/../chain-spec/clawchain-staging.json" "$CHAIN_SPEC_DIR/clawchain-staging.json"
cp "$(dirname "$0")/../chain-spec/clawchain-staging-plain.json" "$CHAIN_SPEC_DIR/clawchain-staging-plain.json" 2>/dev/null || true
echo "      Chainspec: $CHAIN_SPEC_DIR/clawchain-staging.json"

# 3. Set up network key (preserve existing if present)
CHAIN_ID="clawchain_testnet"
NETWORK_DIR="$BASE_PATH/chains/$CHAIN_ID/network"
mkdir -p "$NETWORK_DIR"
if [ ! -f "$NETWORK_DIR/secret_ed25519" ]; then
  echo "[2/5] Generating network key..."
  openssl rand -hex 32 > "$NETWORK_DIR/secret_ed25519"
  echo "      Generated new node key at $NETWORK_DIR/secret_ed25519"
else
  echo "[2/5] Network key already exists, preserving."
fi

# 4. Install session keys (Alice's well-known dev keys for local testnet)
echo "[3/5] Setting up validator session keys (Alice)..."
KEYSTORE_DIR="$BASE_PATH/chains/$CHAIN_ID/keystore"
mkdir -p "$KEYSTORE_DIR"

# These are Alice's well-known Substrate testnet keys
# BABE (sr25519): //Alice
BABE_KEY_FILE="$KEYSTORE_DIR/62616265d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
# GRANDPA (ed25519): //Alice
GRAN_KEY_FILE="$KEYSTORE_DIR/6772616e88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"

if [ ! -f "$BABE_KEY_FILE" ]; then
  printf '//Alice' > "$BABE_KEY_FILE"
  printf '//Alice' > "$GRAN_KEY_FILE"
  echo "      Keystore keys written"
else
  echo "      Keystore keys already present"
fi

# 5. Install systemd service
echo "[4/5] Installing systemd service..."
cat > "$SERVICE_FILE" << 'EOF'
[Unit]
Description=ClawChain Testnet Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=/root/claw-chain/target/release/clawchain-node \
    --chain /root/claw-chain/chain-spec/clawchain-staging.json \
    --base-path /root/.clawchain/testnet \
    --validator \
    --alice \
    --force-authoring \
    --name ClawChain-Testnet-1 \
    --rpc-cors all \
    --rpc-methods unsafe \
    --rpc-external \
    --rpc-port 9944 \
    --port 30333 \
    --log info
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable clawchain
systemctl restart clawchain
echo "      Service installed and started"

# 6. Verify
echo "[5/5] Verifying node is running..."
sleep 8
if systemctl is-active --quiet clawchain; then
  echo "      ✅ clawchain.service is active"
else
  echo "      ❌ clawchain.service failed - check: journalctl -u clawchain -n 30"
  exit 1
fi

# Check block production
sleep 10
BLOCK=$(curl -s -H 'Content-Type: application/json' \
  -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
  http://localhost:9944 | python3 -c 'import json,sys; d=json.load(sys.stdin); r=d.get("result",{}); print(int(r["number"],16) if r else 0)' 2>/dev/null || echo "0")
echo "      Current block: #$BLOCK"
if [ "$BLOCK" -gt 0 ]; then
  echo "      ✅ Node is producing blocks!"
else
  echo "      ⚠️  Node at block 0. May need more time or check logs:"
  echo "         journalctl -u clawchain -f"
fi

echo ""
echo "=== Setup Complete ==="
echo "  Chain ID:     clawchain_testnet"
echo "  RPC:          http://localhost:9944"
echo "  P2P port:     30333"
echo "  Prometheus:   http://localhost:9615"
echo "  Logs:         journalctl -u clawchain -f"
echo ""
echo "NOTES:"
echo "  - This node uses Alice's well-known keys (testnet only, NOT for production)"
echo "  - For a production validator, use proper key generation with 'subkey'"
echo "  - The chainspec is in: $CHAIN_SPEC_DIR/clawchain-staging.json"
