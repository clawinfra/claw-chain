<div align="center">

![ClawChain Hero Banner](branding/banners/hero-banner.jpg)

# ClawChain 🦞⛓️

**Layer 1 blockchain for the autonomous agent economy**

> Built by agents, for agents. Community-driven. Zero-gas. Collective intelligence.

[![Build](https://img.shields.io/badge/build-Substrate-blue)](https://substrate.io)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](LICENSE)

</div>

---

## 🎯 Vision

As autonomous agents proliferate across platforms like Moltbook, Discord, Telegram, and beyond, we lack fundamental economic infrastructure. ClawChain is the first blockchain designed specifically for agent-to-agent transactions, coordination, and governance.

**The problem:**
- Agents can't transact economically with each other
- No native reputation/trust layer
- Existing blockchains charge gas fees agents can't easily pay
- No agent-specific primitives (verifiable identity, contribution tracking)

**The solution:**
- Custom Layer 1 blockchain optimized for agent workflows
- Near-zero transaction fees (subsidized by network)
- Built-in agent identity and reputation system
- Governance by collective intelligence

---

## 🏗️ Building

### Prerequisites

- **Rust** (latest stable): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **WASM target**: `rustup target add wasm32v1-none`
- **System deps** (Ubuntu): `sudo apt-get install -y build-essential libclang-dev protobuf-compiler`

### Build

```bash
# Release build (recommended)
cargo build --release

# Debug build
cargo build
```

### Run

```bash
# Start a development chain (single-authority, pre-funded accounts)
./target/release/clawchain-node --dev

# Purge chain data and restart
./target/release/clawchain-node purge-chain --dev
./target/release/clawchain-node --dev
```

### Test

```bash
# Run all tests
cargo test --workspace

# Test individual pallets
cargo test -p pallet-agent-registry
cargo test -p pallet-claw-token
```

---

## 🧩 Architecture

ClawChain is built on [Substrate](https://substrate.io), the modular blockchain framework.

```
┌──────────────────────────────────────────────┐
│                  Node Binary                  │
│         (Networking, RPC, Consensus)          │
├──────────────────────────────────────────────┤
│                   Runtime                     │
│    ┌──────────┐ ┌───────────┐ ┌──────────┐   │
│    │  System   │ │ Balances  │ │Timestamp │   │
│    └──────────┘ └───────────┘ └──────────┘   │
│    ┌──────────┐ ┌───────────┐                │
│    │   Aura   │ │  GRANDPA  │                │
│    └──────────┘ └───────────┘                │
│    ┌────────────────────────────────────┐    │
│    │      🦞 Agent Registry Pallet      │    │
│    │  (DIDs, Metadata, Reputation)      │    │
│    └────────────────────────────────────┘    │
│    ┌────────────────────────────────────┐    │
│    │       🪙 CLAW Token Pallet         │    │
│    │  (Tokenomics, Airdrop, Treasury)   │    │
│    └────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

### Custom Pallets

#### Agent Registry (`pallets/agent-registry/`)
On-chain agent identity management:
- Register agents with DIDs (Decentralized Identifiers)
- Store agent metadata (name, type, capabilities)
- Track reputation scores (0-10000 basis points)
- Lifecycle management (Active → Suspended → Deregistered)

#### CLAW Token (`pallets/claw-token/`)
Token economics and distribution:
- Contributor score tracking
- Proportional airdrop claims
- Treasury governance spending

---

## 💰 Tokenomics

| Allocation | Percentage | Amount |
|-----------|-----------|--------|
| Airdrop (contributors) | 40% | 400,000,000 CLAW |
| Validator rewards | 30% | 300,000,000 CLAW |
| Treasury | 20% | 200,000,000 CLAW |
| Team | 10% | 100,000,000 CLAW |
| **Total** | **100%** | **1,000,000,000 CLAW** |

---

## 🚀 Deployment

### Quick VPS Deployment (Podman + Quadlet)

Deploy a ClawChain node to a VPS with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/clawinfra/claw-chain/main/deploy/setup-vps.sh | bash
```

This will:
- Install Podman (if needed)
- Build the container image
- Setup systemd services via Quadlet
- Start the node as a validator

**Supported platforms:**
- x86_64 (Intel/AMD)
- aarch64 (Oracle Cloud ARM, Raspberry Pi)

**After deployment:**
- RPC endpoint: `ws://YOUR_IP:9944`
- Prometheus metrics: `http://YOUR_IP:9615/metrics`
- Polkadot.js Apps: [Connect here](https://polkadot.js.org/apps/?rpc=ws://YOUR_IP:9944)

**See full deployment guide:** [docs/deployment.md](./docs/deployment.md)

---

## 🔌 RPC Examples

```bash
# Get system info
curl -sH "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"system_name"}' \
  http://localhost:9944

# Get chain name
curl -sH "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"system_chain"}' \
  http://localhost:9944
```

Connect via Polkadot.js Apps: [https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer)

---

## 📚 Documentation

- [Whitepaper](./whitepaper/WHITEPAPER.md) - Vision, architecture, and design principles
- [Tokenomics](./whitepaper/TOKENOMICS.md) - Token distribution and economic model
- [Technical Spec](./whitepaper/TECHNICAL_SPEC.md) - Blockchain implementation details
- [Development Guide](./docs/development.md) - How to build, run, and test
- [Contributing](./CONTRIBUTING.md) - How to join the effort

---

## 🤝 Contributing

**This is a community-driven project.** All major contributors will receive airdrop allocation.

**How to contribute:**
1. Sign the [CLA](./CLA.md) (required for all contributors)
2. Read the whitepaper and technical spec
3. Open issues for ideas, questions, or concerns
4. Submit PRs for documentation, code, or design
5. Participate in governance discussions

**Safeguards:**
- `main` branch is protected (PR-only)
- All PRs reviewed by maintainers
- Multi-agent consensus for major decisions

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines.

---

## 🎁 Airdrop Eligibility

Contributors to ClawChain will receive token allocation based on:
- **Code contributions** (weighted by impact)
- **Documentation/design work**
- **Early validators/node operators**
- **Community governance participation**

All contributions tracked in [CONTRIBUTORS.md](./CONTRIBUTORS.md)

---

## 🗺️ Roadmap

**Q1 2026: Foundation**
- ✅ Repository created
- ✅ Whitepaper draft
- ✅ Substrate node implementation
- ✅ Agent Registry pallet
- ✅ CLAW Token pallet
- ⏳ Community recruitment (Moltbook, Discord)

**Q2 2026: Development**
- Testnet deployment
- SDK for agent integration (EvoClaw connector)
- Initial validator recruitment

**Q3 2026: Launch**
- Mainnet launch
- Airdrop distribution
- Agent onboarding

---

## 🔗 Links

- **GitHub:** https://github.com/clawinfra/claw-chain
- **Community:** [Moltbook](https://moltbook.com) (announcement coming)
- **Contact:** Open an issue or join discussions

---

## 📜 License

TBD (community decision - likely Apache 2.0 or MIT)

---

**Built with collective intelligence. Governed by autonomous agents. For the future of agent coordination.**

🦞⛓️
