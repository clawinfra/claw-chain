# Run a ClawChain Validator (Docker)

> **Goal:** Working validator node in under 30 minutes using Docker.  
> **Audience:** Linux users familiar with Docker who want to run a ClawChain validator without compiling from source.  
> For the full native binary setup (systemd, monitoring, key management), see [`docs/guides/validator-setup.md`](guides/validator-setup.md).

---

## Prerequisites

### Software

| Requirement | Minimum Version | Check |
|---|---|---|
| Docker Engine | 24.0+ | `docker --version` |
| Docker Compose | v2 (built-in) | `docker compose version` |
| curl + jq | any | `curl --version && jq --version` |

Install Docker on Ubuntu/Debian:
```bash
curl -fsSL https://get.docker.com | sudo bash
sudo usermod -aG docker $USER && newgrp docker
```

### Hardware

| Resource | Minimum | Recommended |
|---|---|---|
| CPU | 4 cores | 8+ cores |
| RAM | 8 GB | 16 GB |
| Storage | 100 GB SSD | 500 GB NVMe |
| Network | 100 Mbps | 1 Gbps |
| Open port | 30333 TCP+UDP | — |

### Firewall

Port **30333 must be reachable from the internet** for peer discovery:
```bash
# ufw (Ubuntu)
sudo ufw allow 30333/tcp
sudo ufw allow 30333/udp
```

---

## Quick Start

### 1. Clone the repo

```bash
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain
```

### 2. Configure environment

```bash
cp deploy/.env.example .env
nano .env   # edit NODE_NAME and optionally CHAIN_SPEC
```

Key variables:

| Variable | Default | Description |
|---|---|---|
| `NODE_NAME` | `ClawChain-Validator` | Human-readable name shown in telemetry |
| `CHAIN_SPEC` | `dev` | Chain spec: `dev`, `local`, or path to custom JSON |
| `AUTO_KEY_GEN` | `false` | Auto-generate session keys on first start |
| `BOOTNODES` | *(empty)* | Comma-separated multiaddr bootnode list |
| `TESTNET_BOOTNODE` | *(see .env.example)* | Pre-configured testnet bootnode (used when `CHAIN_SPEC≠dev`) |
| `RUST_LOG` | `info` | Log level (`info`, `warn`, `debug`) |

### 3. Start the validator

```bash
docker compose -f deploy/docker-compose.validator.yml up -d
```

That's it. The node will:
- Pull/build the image
- Create a persistent volume for chain data
- Start the validator in the background

### 4. Follow logs

```bash
docker compose -f deploy/docker-compose.validator.yml logs -f validator
```

You should see blocks being imported within 30–60 seconds.

---

## Verify Node

### Health check

```bash
curl -s http://localhost:9944/health | jq .
# Expected: {"isSyncing":false,"peers":0,"shouldHavePeers":false}
```

### Block production

```bash
# Get current block number
curl -s -H "Content-Type: application/json" \
  --data '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
  http://localhost:9944 | jq -r '.result.number'
```

Wait 6 seconds and run again — the number should increase.

### Node identity

```bash
# Get your peer ID (needed to share your bootnode address)
curl -s -H "Content-Type: application/json" \
  --data '{"id":1,"jsonrpc":"2.0","method":"system_localPeerId","params":[]}' \
  http://localhost:9944 | jq -r '.result'

# Get node roles (should include "Authority" for validators)
curl -s -H "Content-Type: application/json" \
  --data '{"id":1,"jsonrpc":"2.0","method":"system_nodeRoles","params":[]}' \
  http://localhost:9944 | jq .
```

---

## Get Test CLAW

ClawChain's testnet faucet drips **1000 CLAW** per request with a 24-hour cooldown.

### Using the faucet API

```bash
# Request CLAW tokens (replace with your SS58 address)
curl -s -X POST https://faucet.clawchain.win/drip \
  -H "Content-Type: application/json" \
  -d '{"address":"5Your...Address"}' | jq .
```

Expected response:
```json
{
  "tx_hash": "0xabc123...",
  "amount": "1000 CLAW",
  "next_drip_at": "2026-03-11T09:00:00.000Z"
}
```

### Check faucet balance

```bash
curl -s https://faucet.clawchain.win/status | jq .
```

### Run your own faucet (optional)

```bash
cd faucet
cp .env.example .env
# Edit .env: set RPC_URL and FAUCET_SEED
docker compose -f faucet/docker-compose.yml up -d
```

---

## Register as Validator

### Step 1: Generate session keys

With `AUTO_KEY_GEN=false` (default), generate session keys via the RPC:

```bash
curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' | jq -r '.result'
```

Save the hex output — this is your concatenated session key blob.

Alternatively, set `AUTO_KEY_GEN=true` before first start and the entrypoint will generate keys and log their public addresses.

### Step 2: Set session keys on-chain

Open [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944):

1. **Developer → Extrinsics**
2. Select your **controller account**
3. Call `session.setKeys(keys, proof)`
   - `keys`: the hex blob from `author_rotateKeys`
   - `proof`: `0x` (empty)
4. Submit and sign

### Step 3: Bond CLAW and declare intent

1. **Network → Staking → Accounts → + Validator**
2. Set your **stash** and **controller** accounts
3. Bond at least 1000 CLAW
4. Set your commission (5–10% recommended)
5. Submit `staking.validate()`

Your node will join the validator set at the start of the next era.

---

## Monitoring

### Start Prometheus + Grafana

```bash
docker compose -f deploy/docker-compose.validator.yml \
  --profile monitoring up -d
```

Access:
- **Grafana:** http://localhost:3000 (admin / *see `GRAFANA_ADMIN_PASSWORD` in .env*)
- **Prometheus:** http://localhost:9090
- **Raw metrics:** http://localhost:9615/metrics

### Key metrics to watch

| Metric | Description |
|---|---|
| `substrate_block_height{status="best"}` | Current best block |
| `substrate_block_height{status="finalized"}` | Finalized block |
| `substrate_peers_count` | Connected peers |
| `substrate_validator_is_active` | 1 when in active set |
| `substrate_validator_missed_blocks` | Consecutive missed blocks |

---

## Key Rotation

Rotate session keys without downtime:

```bash
# 1. Generate new keys
NEW_KEYS=$(curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
  | jq -r '.result')

echo "New session keys: $NEW_KEYS"

# 2. Submit setKeys on-chain via Polkadot.js Apps (see Register step 2)
#    The old keys remain active until the new ones are registered on-chain.
```

For the full rotation procedure including backup and emergency key recovery, see [`docs/VALIDATOR-SETUP.md`](VALIDATOR-SETUP.md).

---

## Troubleshooting

### Node won't start

```bash
# Check logs
docker compose -f deploy/docker-compose.validator.yml logs validator --tail 50

# Check container health
docker inspect clawchain-validator --format '{{.State.Health.Status}}'
```

Common causes:
- **Port 30333 already in use:** `lsof -i :30333` → stop the conflicting process
- **Insufficient disk:** `df -h` → ensure 10+ GB free on the Docker volume partition
- **Permission error:** the container runs as UID 1000 — ensure the volume is writable

### No peers connecting

```bash
# Check if your P2P port is reachable from outside
curl -s https://portchecker.co/check?ip=YOUR_IP&port=30333
```

If blocked, open the port in your cloud provider's security group / firewall.

### Node is syncing but not validating

- Verify your session keys are registered: **Polkadot.js → Chain State → session.nextKeys(yourAddress)**
- Verify your bond is sufficient: **Network → Staking → Accounts**
- Check era timing: you join the active set at the next era boundary (every ~6 hours on testnet)

### Container keeps restarting

```bash
docker logs clawchain-validator --tail 100
```

If you see `panicked at 'could not open database'`: the data volume may be corrupted. Back up and recreate:
```bash
docker compose -f deploy/docker-compose.validator.yml down
docker volume rm clawchain-validator-data
docker compose -f deploy/docker-compose.validator.yml up -d
```

---

## Security Checklist

Before running a validator with real stake:

- [ ] **RPC port (9944) is NOT exposed to the internet** — check your firewall
- [ ] **Stash key is offline** — never on the validator server
- [ ] **Strong password** for Grafana admin (`GRAFANA_ADMIN_PASSWORD` in `.env`)
- [ ] **`.env` has restricted permissions:** `chmod 600 .env`
- [ ] **Automatic OS updates enabled:** `sudo apt install unattended-upgrades -y`
- [ ] **SSH key-only authentication** — disable password SSH login
- [ ] **Backups of session keys** stored securely offline
- [ ] **Monitoring alerts** configured (PagerDuty, Grafana alert rules, or Telegram bot)
- [ ] **Docker daemon not exposed** — socket should only be accessible locally

For a full security audit checklist, see [`docs/architecture/security.md`](architecture/security.md).
