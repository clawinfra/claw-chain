# ClawChain Deployment Guide

Complete guide for deploying ClawChain Substrate testnet using Podman and Quadlet.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Prerequisites](#prerequisites)
3. [Quick Start](#quick-start)
4. [Detailed Setup](#detailed-setup)
5. [Running a Validator](#running-a-validator)
6. [Running an RPC-only Node](#running-an-rpc-only-node)
7. [Nginx Reverse Proxy](#nginx-reverse-proxy)
8. [Monitoring](#monitoring)
9. [Connecting to Polkadot.js Apps](#connecting-to-polkadotjs-apps)
10. [Troubleshooting](#troubleshooting)
11. [Scaling Plan](#scaling-plan)

---

## Architecture Overview

### System Architecture (ASCII Diagram)

```
                                    ┌─────────────────────────┐
                                    │   Polkadot.js Apps      │
                                    │   (Web Browser)         │
                                    └───────────┬─────────────┘
                                                │ ws://
                                                │
                              ┌─────────────────▼──────────────────┐
                              │   Nginx Reverse Proxy              │
                              │   - WebSocket support              │
                              │   - Rate limiting (100 req/s)      │
                              │   - SSL termination                │
                              │   Port: 80, 443                    │
                              └─────────────┬──────────────────────┘
                                            │
                                            │ proxy_pass
                                            │
               ┌────────────────────────────┼────────────────────────────┐
               │                            │                            │
               │  ┌─────────────────────────▼───────────────────────┐   │
               │  │   ClawChain Substrate Node                      │   │
               │  │   - Validator / RPC node                        │   │
               │  │   - Ports: 9944 (RPC), 9615 (metrics), 30333   │   │
               │  │   - Volume: /data (persistent blockchain data)  │   │
               │  └─────────────┬───────────────────────────────────┘   │
               │                │                                        │
               │                │ metrics                                │
               │                ▼                                        │
               │  ┌──────────────────────────┐                          │
               │  │   Prometheus              │                          │
               │  │   - Scrapes :9615/metrics │                          │
               │  │   - Retention: 30 days    │                          │
               │  │   Port: 9090              │                          │
               │  └──────────────┬────────────┘                          │
               │                 │                                       │
               │                 │ remote_write (optional)               │
               │                 ▼                                       │
               │  ┌──────────────────────────┐                          │
               │  │   Grafana Cloud / Viz     │                          │
               │  │   (External monitoring)   │                          │
               │  └───────────────────────────┘                          │
               │                                                         │
               │  Podman Pod Network: clawchain-net                     │
               └─────────────────────────────────────────────────────────┘

               ┌─────────────────────────────────────────────────────────┐
               │  Persistent Volumes:                                    │
               │  - clawchain-data.volume  → /data                       │
               │  - prometheus-data.volume → /prometheus                 │
               └─────────────────────────────────────────────────────────┘

               ┌─────────────────────────────────────────────────────────┐
               │  systemd (Quadlet):                                     │
               │  - clawchain-node.service                               │
               │  - clawchain-proxy.service (optional)                   │
               │  - prometheus.service (optional)                        │
               └─────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Container Runtime:** Podman (rootless, daemonless)
- **Service Manager:** systemd via Quadlet (`.container` files)
- **Reverse Proxy:** Nginx (WebSocket, rate limiting, SSL)
- **Monitoring:** Prometheus + Grafana Cloud
- **Blockchain:** Substrate (Rust-based)

### Why Podman + Quadlet?

**Podman:**
- Rootless containers (better security)
- No daemon required (direct fork/exec)
- Compatible with Docker images
- Native systemd integration via Quadlet

**Quadlet:**
- Declarative container management (like docker-compose but systemd-native)
- Automatic systemd service generation
- Restart policies, dependencies, health checks
- Better than raw `podman run` commands

---

## Prerequisites

### Supported Platforms

- **x86_64 (amd64):** Intel/AMD processors
- **aarch64 (arm64):** Oracle Cloud ARM, Raspberry Pi 4+

### Supported Operating Systems

- Ubuntu 22.04 LTS / 24.04 LTS
- Debian 12 (Bookworm)
- Fedora 38+
- RHEL 9+ / Rocky Linux 9+ / AlmaLinux 9+

### Minimum Hardware Requirements

| Component | Validator | RPC Node |
|-----------|-----------|----------|
| CPU       | 4 cores   | 2 cores  |
| RAM       | 8 GB      | 4 GB     |
| Storage   | 100 GB SSD | 50 GB SSD |
| Network   | 100 Mbps  | 50 Mbps  |

**Recommended:**
- 8+ cores, 16 GB RAM, 200 GB NVMe SSD
- Oracle Cloud free tier: ARM VM.Standard.A1.Flex (4 OCPU, 24 GB RAM)

### Software Dependencies

- Podman 4.0+ (4.9+ recommended)
- systemd 250+ (for Quadlet support)
- curl / wget
- git

---

## Quick Start

### One-Command Deployment

For a fresh VPS, run:

```bash
curl -fsSL https://raw.githubusercontent.com/clawinfra/claw-chain/main/deploy/setup-vps.sh | bash
```

This script will:
1. Install Podman (if not present)
2. Clone the repository
3. Build the container image (30-60 min)
4. Install Quadlet systemd files
5. Start the node service
6. Display connection info

**After deployment:**
- RPC endpoint: `ws://YOUR_IP:9944`
- Polkadot.js Apps: `https://polkadot.js.org/apps/?rpc=ws://YOUR_IP:9944`

---

## Detailed Setup

### Step 1: Install Podman

**Debian/Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y podman
```

**Fedora/RHEL:**
```bash
sudo dnf install -y podman
```

**Verify installation:**
```bash
podman --version
# Should show: podman version 4.x.x or newer
```

### Step 2: Clone Repository

```bash
cd ~
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain
```

### Step 3: Build Container Image

**Option A: Build from source (recommended for security)**

```bash
cd ~/claw-chain
podman build -t localhost/clawchain-node:latest -f deploy/Containerfile .
```

Build time:
- x86_64: 30-45 minutes
- aarch64: 45-90 minutes (slower on ARM)

**Option B: Pull pre-built image (when available)**

```bash
# TODO: Update when published to registry
# podman pull ghcr.io/clawinfra/clawchain-node:latest
# podman tag ghcr.io/clawinfra/clawchain-node:latest localhost/clawchain-node:latest
```

**Verify image:**
```bash
podman images | grep clawchain-node
# Should show: localhost/clawchain-node  latest  ...  71MB (compressed)
```

### Step 4: Install Quadlet Files

```bash
# Create Quadlet directory
mkdir -p ~/.config/containers/systemd

# Copy container definitions
cp ~/claw-chain/deploy/quadlet/clawchain-node.container ~/.config/containers/systemd/
cp ~/claw-chain/deploy/quadlet/clawchain-data.volume ~/.config/containers/systemd/

# Reload systemd
systemctl --user daemon-reload
```

### Step 5: Start Node Service

```bash
# Enable service (auto-start on boot)
systemctl --user enable clawchain-node.service

# Start service
systemctl --user start clawchain-node.service

# Enable lingering (keep services running after logout)
loginctl enable-linger $USER
```

### Step 6: Verify Deployment

```bash
# Check service status
systemctl --user status clawchain-node.service

# View logs (follow mode)
journalctl --user -u clawchain-node -f

# Check container
podman ps | grep clawchain-node

# Test RPC endpoint
curl -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
     http://localhost:9944
```

Expected output:
```json
{"jsonrpc":"2.0","result":{"isSyncing":false,"peers":0,"shouldHavePeers":false},"id":1}
```

---

## Running a Validator

Validators participate in consensus and earn rewards.

### Prerequisites

- Validator key pair (session keys)
- Minimum stake (varies by network)
- Stable network connection
- 24/7 uptime

### Configuration

The default `clawchain-node.container` is configured as a validator.

**Key flags:**
```bash
--validator              # Enable validator mode
--base-path /data        # Persistent storage
--rpc-methods safe       # Security: disable dangerous RPC methods
--prometheus-external    # Enable metrics
```

### Generate Session Keys

```bash
# Generate keys via RPC
curl -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys"}' \
     http://localhost:9944

# Or inside container
podman exec -it clawchain-node clawchain-node key generate --scheme Sr25519
```

### Set Session Keys

1. Go to Polkadot.js Apps
2. Navigate to: Developer > Extrinsics
3. Submit: `session.setKeys(keys, proof)`

### Monitor Validator

```bash
# Check validator status
journalctl --user -u clawchain-node -f | grep -i "validator\|finalized"

# Metrics
curl http://localhost:9615/metrics | grep substrate_block_height
```

---

## Running an RPC-only Node

RPC nodes serve queries without participating in consensus.

### Create Custom Quadlet File

```bash
cp ~/.config/containers/systemd/clawchain-node.container \
   ~/.config/containers/systemd/clawchain-rpc.container
```

**Edit `clawchain-rpc.container`:**

```ini
[Unit]
Description=ClawChain RPC Node

[Container]
Image=localhost/clawchain-node:latest
ContainerName=clawchain-rpc
PublishPort=9944:9944
PublishPort=9615:9615
PublishPort=30333:30333
Volume=clawchain-rpc-data.volume:/data
Environment=RUST_LOG=info

# RPC-only mode (no validator flag)
Exec=--base-path /data \
     --chain dev \
     --name "ClawChain-RPC" \
     --rpc-external \
     --rpc-cors all \
     --rpc-methods safe \
     --rpc-port 9944 \
     --prometheus-external \
     --prometheus-port 9615 \
     --port 30333 \
     --state-pruning archive  # Optional: keep full state

[Service]
Restart=always

[Install]
WantedBy=default.target
```

**Create volume and start:**

```bash
echo "[Volume]" > ~/.config/containers/systemd/clawchain-rpc-data.volume
systemctl --user daemon-reload
systemctl --user start clawchain-rpc
```

### Archive Node (Full Historical Data)

Add flags:
```bash
--state-pruning archive
--blocks-pruning archive
```

**Storage requirements:** 500 GB+ (grows over time)

---

## Nginx Reverse Proxy

### Why Use a Reverse Proxy?

- **SSL/TLS termination:** Secure WebSocket connections
- **Rate limiting:** Prevent abuse (100 req/s per IP)
- **Load balancing:** Distribute load across multiple nodes
- **Access control:** IP whitelisting, authentication

### Deploy Nginx Proxy

```bash
# Copy Quadlet file
cp ~/claw-chain/deploy/quadlet/clawchain-proxy.container ~/.config/containers/systemd/

# Reload and start
systemctl --user daemon-reload
systemctl --user start clawchain-proxy
```

**Access via proxy:**
- HTTP: `http://YOUR_IP/`
- Health: `http://YOUR_IP/health`

### SSL/TLS with Let's Encrypt

**1. Install Certbot:**

```bash
sudo apt-get install certbot
```

**2. Obtain certificate:**

```bash
sudo certbot certonly --standalone -d your-domain.com
```

**3. Mount certificates in Quadlet:**

Edit `~/.config/containers/systemd/clawchain-proxy.container`:

```ini
Volume=/etc/letsencrypt:/etc/letsencrypt:ro,Z
```

**4. Update nginx.conf:**

Uncomment SSL server block in `deploy/nginx/nginx.conf`.

**5. Restart proxy:**

```bash
systemctl --user restart clawchain-proxy
```

### SSL with Cloudflare

1. Set DNS A record: `rpc.your-domain.com` → `YOUR_IP`
2. Enable Cloudflare proxy (orange cloud)
3. SSL/TLS mode: Full (strict)
4. No need for certbot (Cloudflare handles SSL)

---

## Monitoring

### Prometheus Metrics

ClawChain node exposes metrics at `:9615/metrics`.

**View raw metrics:**
```bash
curl http://localhost:9615/metrics
```

**Key metrics:**
- `substrate_block_height` - Current block height
- `substrate_finalized_height` - Finalized block height
- `substrate_peers_count` - Connected peers
- `substrate_ready_transactions_number` - Tx pool size

### Local Prometheus

```bash
# Install Prometheus container
cp ~/claw-chain/deploy/quadlet/prometheus.container ~/.config/containers/systemd/
cp ~/claw-chain/deploy/quadlet/prometheus-data.volume ~/.config/containers/systemd/

systemctl --user daemon-reload
systemctl --user start prometheus
```

**Access Prometheus:** http://localhost:9090

**Query examples:**
```promql
# Block production rate
rate(substrate_block_height[5m])

# Peer count over time
substrate_peers_count

# Transaction pool size
substrate_ready_transactions_number
```

### Grafana Cloud (Free Tier)

1. **Sign up:** https://grafana.com/products/cloud/
2. **Get credentials:** Prometheus > Remote Write
3. **Edit `deploy/monitoring/prometheus.yml`:**

```yaml
remote_write:
  - url: https://prometheus-prod-01-eu-west-0.grafana.net/api/prom/push
    basic_auth:
      username: <YOUR_INSTANCE_ID>
      password: <YOUR_API_KEY>
```

4. **Restart Prometheus:**

```bash
systemctl --user restart prometheus
```

5. **Import dashboard:** Search Grafana for "Substrate Node Exporter"

---

## Connecting to Polkadot.js Apps

### Local Node

```
https://polkadot.js.org/apps/?rpc=ws://localhost:9944
```

### Remote Node

```
https://polkadot.js.org/apps/?rpc=ws://YOUR_IP:9944
```

### Via SSL (Domain)

```
https://polkadot.js.org/apps/?rpc=wss://rpc.your-domain.com
```

### Verify Connection

1. Top-left corner should show green dot
2. Network status: "connected"
3. Block number should increment

---

## Troubleshooting

### Service Won't Start

```bash
# Check service status
systemctl --user status clawchain-node

# View full logs
journalctl --user -u clawchain-node -n 100 --no-pager

# Check container status
podman ps -a | grep clawchain-node

# Inspect container
podman inspect clawchain-node
```

### Port Already in Use

```bash
# Find process using port
sudo ss -tulpn | grep -E ':(9944|9615|30333)'

# Kill process (if safe)
sudo kill -9 <PID>
```

### Container Build Fails

**Common issues:**

1. **Out of memory:**
   - Increase swap space
   - Build with `--jobs 1` (slower but less RAM)

2. **Disk space:**
   ```bash
   df -h
   # Need 20+ GB free for build
   ```

3. **Network timeout:**
   - Retry build
   - Check cargo mirror settings

### Node Not Syncing

```bash
# Check logs for sync status
journalctl --user -u clawchain-node -f | grep "Syncing"

# Verify peer connections
curl -H "Content-Type: application/json" \
     -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
     http://localhost:9944

# Check if P2P port is open
sudo ufw allow 30333/tcp
```

### WebSocket Connection Fails

**Check Nginx config:**
```bash
podman exec -it clawchain-proxy nginx -t
```

**Check WebSocket upgrade headers:**
```bash
curl -i -N \
     -H "Connection: Upgrade" \
     -H "Upgrade: websocket" \
     -H "Sec-WebSocket-Version: 13" \
     -H "Sec-WebSocket-Key: test" \
     http://YOUR_IP:9944
```

Should return HTTP 101 Switching Protocols.

---

## Scaling Plan

### Phase 1: Single Node (Current)

**Setup:**
- 1 validator/RPC node
- Local Prometheus (optional)
- Direct RPC access

**Capacity:**
- ~100 RPC requests/second
- Single point of failure

**Cost:** $0-20/month (Oracle free tier or small VPS)

### Phase 2: Multi-Node + Load Balancer

**Setup:**
```
                  ┌────────────────────┐
                  │   Load Balancer    │
                  │   (Nginx / HAProxy)│
                  └───────┬────────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
    ┌─────────┐     ┌─────────┐     ┌─────────┐
    │  Node 1 │     │  Node 2 │     │  Node 3 │
    │ (RPC)   │     │ (RPC)   │     │ (RPC)   │
    └─────────┘     └─────────┘     └─────────┘
          │               │               │
          └───────────────┴───────────────┘
                          │
                  ┌───────▼────────┐
                  │   Validator    │
                  │   (Internal)   │
                  └────────────────┘
```

**Capacity:**
- ~1,000 RPC requests/second
- Horizontal scaling
- High availability

**Implementation:**
```bash
# Deploy 3 RPC nodes on different VMs
# Configure HAProxy for round-robin load balancing
# Keep validator node private (firewall RPC port)
```

**Cost:** ~$60-100/month (3x VPS)

### Phase 3: Managed Kubernetes + CDN

**Setup:**
```
          ┌────────────────────────┐
          │   Cloudflare CDN       │
          │   (DDoS protection)    │
          └───────┬────────────────┘
                  │
          ┌───────▼────────────────┐
          │   Kubernetes Ingress   │
          │   (SSL, rate limit)    │
          └───────┬────────────────┘
                  │
       ┌──────────┼──────────┐
       │          │          │
       ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │ RPC-1  │ │ RPC-2  │ │ RPC-N  │
   │  Pod   │ │  Pod   │ │  Pod   │
   └────────┘ └────────┘ └────────┘
       │
   ┌───▼────────────────────┐
   │  Persistent Volume      │
   │  (Shared state - read)  │
   └─────────────────────────┘
```

**Features:**
- Auto-scaling based on load
- Global CDN distribution
- DDoS protection
- Automated failover

**Capacity:** 10,000+ RPC req/s

**Cost:** ~$300-500/month (managed K8s + CDN)

**Implementation:**
```bash
# Convert Containerfile to Kubernetes Deployment
# Use ReadWriteMany volumes for shared state
# Cloudflare for CDN + DDoS
# Horizontal Pod Autoscaler for dynamic scaling
```

---

## Additional Resources

### Documentation

- Substrate Docs: https://docs.substrate.io/
- Polkadot Wiki: https://wiki.polkadot.network/
- Podman Docs: https://docs.podman.io/
- Quadlet: https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html

### Community

- GitHub: https://github.com/clawinfra/claw-chain
- Discord: [Coming soon]

### Tools

- Polkadot.js Apps: https://polkadot.js.org/apps/
- Polkadot.js Extension: https://polkadot.js.org/extension/
- Substrate Telemetry: https://telemetry.polkadot.io/

---

## License

ClawChain is licensed under [LICENSE]. Deployment scripts are MIT licensed.

---

**Questions?** Open an issue on GitHub: https://github.com/clawinfra/claw-chain/issues
