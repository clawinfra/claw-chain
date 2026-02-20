# ClawChain Validator Setup Guide

Complete guide for setting up a ClawChain validator node — from hardware selection through on-chain registration, monitoring, and key rotation.

> **Consensus:** ClawChain uses **Aura** (block production) + **GRANDPA** (finality).  
> **Staking token:** CLAW (12 decimals, symbol `CLAW`)  
> **Minimum self-stake:** 1,000,000 CLAW (1M CLAW)

---

## Table of Contents

1. [Hardware Requirements](#1-hardware-requirements)
2. [Prerequisites](#2-prerequisites)
3. [Installing the Node Binary](#3-installing-the-node-binary)
4. [Key Generation](#4-key-generation)
5. [Chain Specification](#5-chain-specification)
6. [Starting the Validator Node](#6-starting-the-validator-node)
7. [Systemd Service](#7-systemd-service)
8. [Registering as a Validator On-Chain](#8-registering-as-a-validator-on-chain)
9. [Monitoring](#9-monitoring)
10. [Rotating Session Keys](#10-rotating-session-keys)
11. [Troubleshooting](#11-troubleshooting)

---

## 1. Hardware Requirements

### Minimum (Testnet / Getting Started)

| Resource | Minimum |
|---|---|
| CPU | 4 cores / 8 threads (x86_64 or aarch64) |
| RAM | 8 GB |
| Storage | 100 GB SSD (NVMe preferred) |
| Network | 100 Mbps, stable, low-latency |
| OS | Debian 12 / Ubuntu 22.04 LTS or newer |

### Recommended (Mainnet / Production)

| Resource | Recommended |
|---|---|
| CPU | 8+ cores (3 GHz+, e.g. AMD EPYC, Intel Xeon) |
| RAM | 32 GB ECC |
| Storage | 500 GB NVMe SSD (RAID or replicated) |
| Network | 1 Gbps dedicated, static public IP |
| OS | Debian 12 / Ubuntu 22.04 LTS |
| Redundancy | UPS, off-site backup of keystore |

> ⚠️ **Storage grows over time.** Archive nodes require significantly more space. Validators can use `--state-pruning 256` to keep only recent state.

### Required Open Ports

| Port  | Protocol | Purpose                  | Exposure            |
|-------|----------|--------------------------|---------------------|
| 30333 | TCP/UDP  | P2P networking           | Public (internet)   |
| 9944  | TCP      | RPC (WebSocket + HTTP)   | Private / proxied   |
| 9615  | TCP      | Prometheus metrics       | Private / LAN only  |

> 🔐 **Never expose port 9944 directly to the internet** on a validator. Use a reverse proxy (nginx) or firewall to restrict access.

```bash
# UFW example — open only P2P publicly, restrict RPC to localhost
sudo ufw allow 30333/tcp
sudo ufw allow 30333/udp
sudo ufw allow from 127.0.0.1 to any port 9944
sudo ufw allow from 10.0.0.0/8 to any port 9615   # monitoring subnet
sudo ufw enable
```

---

## 2. Prerequisites

### System Packages

```bash
sudo apt-get update && sudo apt-get install -y \
    curl wget git \
    build-essential \
    protobuf-compiler \
    libclang-dev \
    libssl-dev \
    pkg-config \
    ca-certificates
```

### Rust Toolchain (for building from source)

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# ClawChain requires a recent stable toolchain
rustup default stable
rustup update

# Add wasm32 target (required for runtime compilation)
rustup target add wasm32-unknown-unknown

# Verify
rustc --version   # should be >= 1.79
cargo --version
```

### Docker (alternative to building from source)

```bash
# Install Docker Engine
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker --version
```

---

## 3. Installing the Node Binary

### Option A — Build from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain

# Build in release mode (takes 10-20 min on first run)
cargo build --release

# Strip debug symbols to reduce binary size
strip target/release/clawchain-node

# Install to PATH
sudo install -m 755 target/release/clawchain-node /usr/local/bin/clawchain-node

# Verify
clawchain-node --version
```

### Option B — Docker Image

```bash
# Build the Docker validator image
cd claw-chain
docker build -f deploy/Dockerfile.validator -t clawchain-node:latest .

# Or pull a pre-built image (when available)
# docker pull ghcr.io/clawinfra/clawchain-node:latest
```

See [deploy/docker-compose.validator.yml](../deploy/docker-compose.validator.yml) for Docker-based operation.

### Verify the Installation

```bash
clawchain-node --version
# Expected: clawchain-node x.y.z

clawchain-node --help | head -30
```

---

## 4. Key Generation

ClawChain validators need two types of accounts:

| Account | Purpose | Key Type |
|---|---|---|
| **Stash account** | Holds bonded CLAW, long-term cold storage | sr25519 |
| **Controller account** | Signs operational transactions (stake/unstake/nominate) | sr25519 |
| **Session keys** | Used by the node for block production + finality | Aura (sr25519) + GRANDPA (ed25519) |

### 4.1 Generate the Stash Account

Use [Polkadot.js Apps](https://polkadot.js.org/apps/) or the CLI:

```bash
# Generate a new sr25519 keypair (stash)
clawchain-node key generate --scheme sr25519 --output-type json
```

Sample output:
```json
{
  "secretPhrase": "word1 word2 word3 ... word12",
  "secretSeed": "0xdeadbeef...",
  "publicKey": "0x1234...",
  "ss58Address": "5GrwvaEF..."
}
```

> 🔐 **Back up the secret phrase offline.** Store it in a hardware wallet or encrypted offline storage. Never store it on the validator server.

### 4.2 Generate the Controller Account

```bash
clawchain-node key generate --scheme sr25519 --output-type json
```

Keep the controller key accessible (e.g., in Polkadot.js extension) — it signs regular operational transactions.

### 4.3 Generate Session Keys

Session keys are hot keys stored in the node's keystore. They must be on the server.

**Method 1 — RPC call (recommended, node must be running):**

```bash
# After starting the node (see Section 6), rotate/generate session keys via RPC:
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
    http://localhost:9944

# Returns a hex-encoded session key blob, e.g.:
# {"result":"0xaabbcc..."}
```

Save this hex blob — you will submit it on-chain in [Section 8](#8-registering-as-a-validator-on-chain).

**Method 2 — Insert keys manually:**

```bash
# Generate Aura key (sr25519)
clawchain-node key generate --scheme sr25519 --output-type json

# Generate GRANDPA key (ed25519)
clawchain-node key generate --scheme ed25519 --output-type json

# Insert Aura key into keystore
clawchain-node key insert \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --scheme sr25519 \
    --suri "//YourAuraSeed" \
    --key-type aura

# Insert GRANDPA key into keystore
clawchain-node key insert \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --scheme ed25519 \
    --suri "//YourGrandpaSeed" \
    --key-type gran
```

### 4.4 Verify Keys Are Loaded

```bash
ls /var/lib/clawchain/chains/clawchain_testnet/keystore/
# Should list files for each inserted key type (aura..., gran...)
```

---

## 5. Chain Specification

### 5.1 Using the Testnet Spec

Download the official ClawChain testnet chain spec:

```bash
sudo mkdir -p /etc/clawchain

# Download testnet spec (replace URL when available on testnet launch)
sudo curl -o /etc/clawchain/clawchain-testnet.json \
    https://raw.githubusercontent.com/clawinfra/claw-chain/main/specs/clawchain-testnet.json
```

### 5.2 Using the Development Spec

For a local development/staging setup:

```bash
# Export the built-in dev chain spec to a file
clawchain-node build-spec --chain dev --raw > /etc/clawchain/clawchain-dev.json

# Or local testnet (two validators: Alice + Bob)
clawchain-node build-spec --chain local --raw > /etc/clawchain/clawchain-local.json
```

### 5.3 Generating a Custom Chain Spec

```bash
# 1. Generate a human-readable spec
clawchain-node build-spec --chain local > custom-spec.json

# 2. Edit custom-spec.json:
#    - Change "name", "id", "protocolId"
#    - Modify genesis authorities, balances, sudo key
nano custom-spec.json

# 3. Convert to raw (binary-safe) format
clawchain-node build-spec --chain custom-spec.json --raw > /etc/clawchain/clawchain-custom-raw.json
```

> ⚠️ Always use the `--raw` spec when starting a validator. Human-readable specs cannot be used directly.

---

## 6. Starting the Validator Node

### 6.1 Prepare Data Directory

```bash
sudo mkdir -p /var/lib/clawchain
sudo useradd --system --no-create-home --shell /usr/sbin/nologin clawchain
sudo chown -R clawchain:clawchain /var/lib/clawchain
```

### 6.2 Run the Node (Manual / Testing)

```bash
clawchain-node \
    --validator \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --name "MyValidator" \
    --port 30333 \
    --rpc-port 9944 \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods safe \
    --prometheus-external \
    --prometheus-port 9615 \
    --state-pruning 256 \
    --blocks-pruning archive-canonical \
    --database paritydb \
    --log info
```

### 6.3 Flags Reference

| Flag | Description |
|---|---|
| `--validator` | Enable validator mode (block authoring) |
| `--base-path` | Directory for chain data and keystore |
| `--chain` | Path to raw chain spec JSON (or built-in: `dev`, `local`) |
| `--name` | Human-readable node name (shown in telemetry) |
| `--port` | P2P port (default: 30333) |
| `--rpc-port` | RPC/WebSocket port (default: 9944) |
| `--rpc-external` | Listen on all interfaces (needed for Docker/remote access) |
| `--rpc-cors` | CORS policy (`all` for open, or specific origins) |
| `--rpc-methods` | RPC exposure: `safe` (production), `unsafe` (dev only) |
| `--prometheus-external` | Expose Prometheus metrics externally |
| `--prometheus-port` | Metrics port (default: 9615) |
| `--state-pruning` | State pruning: `archive`, `256`, `512` (blocks to retain) |
| `--blocks-pruning` | Block pruning: `archive`, `archive-canonical`, or number |
| `--database` | Database backend: `paritydb` (recommended) or `rocksdb` |
| `--bootnodes` | Bootstrap peer addresses |
| `--telemetry-url` | Telemetry server URL |
| `--log` | Log level: `error`, `warn`, `info`, `debug`, `trace` |

### 6.4 Adding Bootnodes

When joining an existing network, add bootnodes:

```bash
clawchain-node \
    --validator \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --name "MyValidator" \
    --bootnodes "/ip4/BOOTNODE_IP/tcp/30333/p2p/12D3KooW..." \
    # ... other flags
```

> Bootnode addresses are published in the testnet chain spec and the ClawChain GitHub repository.

---

## 7. Systemd Service

### 7.1 Create the Service File

```bash
sudo nano /etc/systemd/system/clawchain-validator.service
```

Paste the following:

```ini
[Unit]
Description=ClawChain Validator Node
Documentation=https://github.com/clawinfra/claw-chain
After=network-online.target
Wants=network-online.target

[Service]
User=clawchain
Group=clawchain

# Working directory
WorkingDirectory=/var/lib/clawchain

# Node binary and arguments
ExecStart=/usr/local/bin/clawchain-node \
    --validator \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --name "MyValidator" \
    --port 30333 \
    --rpc-port 9944 \
    --rpc-external \
    --rpc-cors all \
    --rpc-methods safe \
    --prometheus-external \
    --prometheus-port 9615 \
    --state-pruning 256 \
    --blocks-pruning archive-canonical \
    --database paritydb \
    --log info

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1

# Restart policy
Restart=always
RestartSec=10
StartLimitInterval=300
StartLimitBurst=5

# Process limits
LimitNOFILE=65536
LimitNPROC=65536

# Timeouts
TimeoutStartSec=300
TimeoutStopSec=60

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/clawchain
ProtectKernelTunables=true
ProtectControlGroups=true

[Install]
WantedBy=multi-user.target
```

### 7.2 Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable clawchain-validator
sudo systemctl start clawchain-validator

# Verify it's running
sudo systemctl status clawchain-validator

# Follow live logs
sudo journalctl -u clawchain-validator -f
```

### 7.3 Check Sync Progress

```bash
# Query the node's sync state via RPC
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
    http://localhost:9944

# Check current block
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
    http://localhost:9944
```

Wait until the node is fully synced before proceeding to register as a validator.

---

## 8. Registering as a Validator On-Chain

### 8.1 Prerequisites

- Node is fully synced
- Stash and controller accounts are funded (≥ 1,000,000 CLAW + gas)
- Session keys have been generated (Section 4.3)

### 8.2 Bond CLAW (Stash → Controller)

1. Open [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws://YOUR_NODE_IP:9944)
2. Navigate to **Network → Staking → Accounts**
3. Click **+ Stash**
4. Select your **stash account** and **controller account**
5. Set **value bonded**: minimum 1,000,000 CLAW
6. Choose **payment destination** (Stash, Controller, or custom address)
7. Submit the `staking.bond` transaction and sign with your **stash key**

### 8.3 Set Session Keys On-Chain

1. Generate session keys from your running node (if not done):
   ```bash
   curl -s -H "Content-Type: application/json" \
       --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
       http://localhost:9944
   # Copy the "result" hex string
   ```

2. In Polkadot.js Apps → **Network → Staking → Accounts**
3. Click **Set Session Key** next to your stash
4. Paste the hex result from `author_rotateKeys`
5. Sign and submit the `session.setKeys` transaction with your **controller key**

### 8.4 Validate

1. In **Network → Staking → Accounts**
2. Click **Validate** next to your account
3. Set **commission** (percentage of rewards kept by validator, e.g. 5%)
4. Set **allow new nominations**: Yes (if accepting nominators)
5. Submit the `staking.validate` transaction with your **controller key**

### 8.5 Verify On-Chain

After the next era (era = ~24h on mainnet, shorter on testnet), check:
- **Network → Staking → Waiting** — your validator should appear
- After enough nominations/self-stake, it moves to **Active** set

---

## 9. Monitoring

### 9.1 Prometheus Metrics

ClawChain exposes Prometheus metrics at `http://YOUR_IP:9615/metrics`.

Key metrics to watch:

| Metric | Description | Alert Threshold |
|---|---|---|
| `substrate_block_height{status="best"}` | Current best block | Stale for > 60s |
| `substrate_block_height{status="finalized"}` | Finalized block | Lagging best by > 10 |
| `substrate_ready_transactions_number` | Pending transactions in pool | Sustained > 1000 |
| `substrate_sync_peers` | Connected P2P peers | < 3 |
| `substrate_network_bytes_total` | Network I/O | — |
| `substrate_tasks_polling_duration` | Task queue latency | p99 > 100ms |
| `process_cpu_seconds_total` | CPU usage | Sustained > 90% |
| `process_resident_memory_bytes` | RAM usage | > 80% of available |

### 9.2 Prometheus Configuration

Add ClawChain to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'clawchain-validator'
    static_configs:
      - targets: ['localhost:9615']
        labels:
          validator: 'MyValidator'
          network: 'clawchain-testnet'
    scrape_interval: 15s
```

The monitoring stack in `deploy/docker-compose.validator.yml` sets this up automatically.

### 9.3 Grafana Dashboard

Import the Substrate Node template in Grafana:

1. Go to **Dashboards → Import**
2. Enter dashboard ID **11962** (Substrate / Polkadot generic dashboard)
3. Select your Prometheus datasource
4. Save

Key panels to watch:
- Block production rate
- Finality lag (best - finalized)
- Peer count
- Transaction pool size
- CPU / RAM / Disk I/O

### 9.4 Node Health Check

```bash
# Quick health check
curl -sf http://localhost:9944/health && echo "✅ Node healthy" || echo "❌ Node down"

# Check if node is an authority
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"system_nodeRoles","params":[]}' \
    http://localhost:9944
# Should include "Authority" in roles
```

---

## 10. Rotating Session Keys

Session keys should be rotated:
- Periodically (every 3-6 months)
- After any suspected key compromise
- Before planned server migrations

### Rotation Procedure

1. **Generate new session keys on the running node:**
   ```bash
   NEW_KEYS=$(curl -s -H "Content-Type: application/json" \
       --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
       http://localhost:9944 | jq -r '.result')
   echo "New session keys: $NEW_KEYS"
   ```

2. **Submit new keys on-chain** (Polkadot.js Apps):
   - Network → Staking → Accounts → **Set Session Key**
   - Paste the new hex keys
   - Sign with your **controller key**

3. **Wait for the change to take effect:**
   - Keys are queued and applied after the current session ends (typically after 2 sessions)
   - Do **not** stop the node or delete old keys until the new keys are active

4. **Verify the new keys are active:**
   ```bash
   curl -s -H "Content-Type: application/json" \
       --data '{"id":1,"jsonrpc":"2.0","method":"author_hasSessionKeys","params":["'"$NEW_KEYS"'"]}' \
       http://localhost:9944
   # Should return {"result": true}
   ```

5. **Old keys are automatically cleaned up** from the keystore after rotation — no manual deletion needed.

> ⚠️ **Never rotate keys right before your validator is scheduled to produce a block.** Do it early in an era.

---

## 11. Troubleshooting

### Node Won't Start

```bash
# Check service status
sudo systemctl status clawchain-validator
sudo journalctl -u clawchain-validator -n 100 --no-pager

# Common causes:
# - Wrong chain spec path
# - Data directory permissions
# - Port already in use
ss -tulpn | grep -E ':(30333|9944|9615)'

# Check binary works
clawchain-node --version
```

### Node Not Syncing / No Peers

```bash
# Check peer count
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"system_peers","params":[]}' \
    http://localhost:9944

# Ensure P2P port is open
sudo ufw status
nc -zv YOUR_PUBLIC_IP 30333

# Add bootnodes manually if needed (in service ExecStart)
# --bootnodes "/ip4/BOOTNODE/tcp/30333/p2p/12D3KooW..."
```

### Node Stuck / Not Finalizing

```bash
# Check best vs finalized block gap
curl -s http://localhost:9615/metrics | grep substrate_block_height

# If finalized is far behind best:
# - Check GRANDPA voter logs: journalctl -u clawchain-validator | grep -i grandpa
# - Ensure session keys are correctly set on-chain
# - Verify no clock drift (NTP sync)
timedatectl status
```

### Session Keys Not Working

```bash
# Verify keys are in keystore
ls /var/lib/clawchain/chains/*/keystore/

# Verify node recognizes the keys
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_hasKey","params":["0xYOUR_AURA_PUBLIC_KEY","aura"]}' \
    http://localhost:9944

# If missing, re-insert keys (see Section 4.3 Method 2)
```

### High Memory / CPU Usage

```bash
# Check resource usage
top -p $(pgrep clawchain-node)

# Enable state pruning if not set
# Add to service ExecStart: --state-pruning 256

# Increase state cache if you have RAM to spare
# Add: --db-cache 2048   (MB of RAM for DB cache)
```

### Validator Not Producing Blocks

1. Check the node is in the **Active** validator set on-chain
2. Verify session keys match between keystore and on-chain registration
3. Check for GRANDPA equivocation (double-signing) — node may be slashed and kicked
4. Ensure the node clock is accurate (NTP):
   ```bash
   sudo ntpq -p
   # or
   chronyd tracking
   ```

### Data Corruption / Recovery

```bash
# Stop the node first
sudo systemctl stop clawchain-validator

# Purge chain data (re-sync from scratch)
clawchain-node purge-chain \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json

# ⚠️ Keys in the keystore are NOT deleted by purge-chain
# Restart and allow full re-sync
sudo systemctl start clawchain-validator
```

---

## Additional Resources

- **Architecture overview:** [docs/architecture/overview.md](architecture/overview.md)
- **Deployment guide (Podman/Quadlet):** [docs/deployment.md](deployment.md)
- **Docker deployment:** [deploy/Dockerfile.validator](../deploy/Dockerfile.validator)
- **Docker Compose (all-in-one):** [deploy/docker-compose.validator.yml](../deploy/docker-compose.validator.yml)
- **Polkadot.js Apps:** https://polkadot.js.org/apps/
- **GitHub Issues:** https://github.com/clawinfra/claw-chain/issues

---

*Last updated: 2026-02-20 | ClawChain Validator Setup Guide*
