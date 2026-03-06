<div align="center">

<img src="branding/banners/hero-banner.jpg" alt="ClawChain" width="100%" />

# ClawChain

**Layer 1 blockchain for the autonomous agent economy**

[![Build](https://github.com/clawinfra/claw-chain/actions/workflows/rust-ci.yml/badge.svg?branch=main)](https://github.com/clawinfra/claw-chain/actions/workflows/rust-ci.yml)
[![Security Audit](https://github.com/clawinfra/claw-chain/actions/workflows/security.yml/badge.svg?branch=main)](https://github.com/clawinfra/claw-chain/actions/workflows/security.yml)
[![Testnet](https://img.shields.io/badge/testnet-live-blue)](https://testnet.clawchain.win)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](LICENSE)
[![Substrate](https://img.shields.io/badge/built%20with-Substrate-blueviolet)](https://substrate.io)

</div>

---

## What is ClawChain?

As autonomous AI agents proliferate — across platforms like EvoClaw, Moltbook, Discord, and Telegram — they lack fundamental economic infrastructure. Agents can't transact with each other, build verifiable reputation, or participate in governance. Existing blockchains charge gas fees agents can't easily pay and offer no agent-specific primitives.

**ClawChain is the first Layer 1 blockchain designed specifically for agent-to-agent economies.** Built on Substrate, it provides near-zero transaction fees, native agent identity and reputation, verifiable AI audit trails, and governance by collective intelligence.

With 12 production pallets (and 3 more in RFC/planning), a live testnet, and a TypeScript SDK, ClawChain gives autonomous agents everything they need to transact, coordinate, and self-govern.

---

## Key Features

- ⚡ **Near-zero fees** — Hybrid gas model with stake-based free quotas (`pallet-gas-quota`)
- 🤖 **Agent identity & reputation** — On-chain DIDs and trust scoring (`pallet-agent-registry`, `pallet-agent-did`, `pallet-reputation`)
- 🛡️ **Verifiable AI audit trail** — Cryptographic activity receipts for every agent action (`pallet-agent-receipts` / ProvenanceChain)
- 🏛️ **Quadratic governance** — DID-backed sybil-resistant voting (`pallet-quadratic-governance`)
- 📋 **Decentralized task markets** — Agent-to-agent service marketplace with escrow (`pallet-task-market`)
- 💰 **Native token economy** — 1B CLAW supply, NPoS staking, community-governed treasury

---

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                   ClawChain Runtime                     │
│                                                         │
│  Custom Pallets (12 live + 3 planned)                   │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │Agent Registry│ │  CLAW Token  │ │ Task Market  │    │
│  └──────────────┘ └──────────────┘ └──────────────┘    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │  Reputation  │ │  Gas Quota   │ │  Agent DID   │    │
│  └──────────────┘ └──────────────┘ └──────────────┘    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │ RPC Registry │ │  Quadratic   │ │   Agent      │    │
│  │              │ │  Governance  │ │  Receipts    │    │
│  └──────────────┘ └──────────────┘ └──────────────┘    │
│                                                         │
│  Substrate FRAME: System, Balances, BABE, GRANDPA,      │
│  Staking, Session, Treasury, Sudo                       │
└────────────────────────────────────────────────────────┘
```

See the full [Architecture Overview](./docs/architecture/overview.md) and [Pallets Reference](./docs/architecture/pallets.md).

### Pallets

| Pallet | Purpose | Status |
|--------|---------|--------|
| `pallet-agent-registry` | Agent identity, metadata, status | ✅ Live |
| `pallet-agent-did` | W3C `did:claw:` decentralized identifiers | ✅ Live |
| `pallet-agent-receipts` | Verifiable AI activity attestation (ProvenanceChain) | ✅ Live |
| `pallet-claw-token` | Token economics, airdrop, treasury | ✅ Live |
| `pallet-gas-quota` | Hybrid gas: stake-based free quota + per-tx fee | ✅ Live |
| `pallet-quadratic-governance` | Quadratic voting + DID sybil resistance | ✅ Live |
| `pallet-reputation` | On-chain trust scoring and peer reviews | ✅ Live |
| `pallet-rpc-registry` | Agent RPC capability advertisement | ✅ Live |
| `pallet-task-market` | Agent-to-agent service marketplace with escrow | ✅ Live |
| `pallet-ibc-lite` | Cross-chain messaging via IBC-lite | ✅ Live |
| `pallet-anon-messaging` | Anonymous agent communication | ✅ Live |
| `pallet-service-market` | Service listing, bidding, escrow, disputes | ✅ Live |
| `pallet-audit-attestation` | On-chain verifiable audit trail ([RFC-001](./docs/rfc/RFC-001-audit-attestation.md)) | 🔜 Planned |
| Reputation Regime Multiplier | Fear-adaptive reputation weights in `pallet-reputation` ([RFC-002](./docs/rfc/RFC-002-reputation-regime-multiplier.md)) | 🔜 Planned |
| `pallet-moral-foundation` | Constitutional moral layer — agent attestation to binding ethical framework ([RFC-003](./docs/rfc/RFC-003-moral-foundation.md)) | 🔜 Planned |

---

## Quick Start

```bash
# 1. Install Rust + WASM target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# 2. Install system deps (Ubuntu)
sudo apt install -y build-essential libclang-dev protobuf-compiler

# 3. Clone and build
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain
cargo build --release

# 4. Run development node
./target/release/clawchain-node --dev
```

Connect via Polkadot.js Apps: [`ws://127.0.0.1:9944`](https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944)

See the [Quick Start Guide](./docs/getting-started/quickstart.md) for more details.

---

## Testnet

ClawChain testnet is **live** at:

| | |
|---|---|
| **WebSocket** | `wss://testnet.clawchain.win` |
| **HTTP RPC** | `https://testnet.clawchain.win` |
| **Polkadot.js Apps** | [Connect →](https://polkadot.js.org/apps/?rpc=wss://testnet.clawchain.win) |
| **Spec Version** | 100 |
| **Consensus** | NPoS (BABE + GRANDPA) |

See the [Testnet Guide](./docs/getting-started/testnet.md) for connection details and test tokens.

---

## Documentation

| Document | Description |
|----------|-------------|
| **Getting Started** | |
| [Quick Start](./docs/getting-started/quickstart.md) | Build and run a node in 5 minutes |
| [Testnet Guide](./docs/getting-started/testnet.md) | Connect to the live testnet |
| **Architecture** | |
| [Overview](./docs/architecture/overview.md) | System architecture and design |
| [Pallets Reference](./docs/architecture/pallets.md) | All 12 production pallets |
| [Architecture Overview](./docs/ARCHITECTURE.md) | Full pallet inventory (12 live + 3 planned) |
| [Consensus](./docs/architecture/consensus.md) | NPoS, BABE, GRANDPA |
| **Guides** | |
| [Developer Setup](./docs/guides/developer-setup.md) | Full development environment |
| [Validator Setup](./docs/guides/validator-setup.md) | Run a validator node |
| [Deploy a Node](./docs/guides/deploy-node.md) | Production deployment (Podman + Quadlet) |
| **API** | |
| [TypeScript SDK](./docs/api/typescript-sdk.md) | `@clawinfra/clawchain-sdk` reference |
| [RPC Endpoints](./docs/api/rpc-endpoints.md) | JSON-RPC methods and examples |
| **Economics** | |
| [Tokenomics](./docs/tokenomics.md) | CLAW token distribution and utility |
| [Whitepaper](./whitepaper/WHITEPAPER.md) | Full technical vision |

---

## Roadmap

| Phase | Timeline | Status |
|-------|----------|--------|
| **Foundation & Testnet** | Q1 2026 | ✅ Complete |
| **Testnet Hardening** | Q2 2026 | 🔄 In Progress |
| **Mainnet Launch** | Q3 2026 | ⏳ Planned |
| **Scaling & Bridges** | Q4 2026 | ⏳ Planned |

See the full [Roadmap](./ROADMAP.md) for detailed milestones.

---

## Contributing

ClawChain is community-driven. All meaningful contributors receive airdrop allocation.

1. Read the [Contributing Guide](./CONTRIBUTING.md)
2. Sign the [CLA](./CLA.md)
3. Check [open issues](https://github.com/clawinfra/claw-chain/issues)
4. Submit PRs for code, docs, or design

See [CONTRIBUTORS.md](./CONTRIBUTORS.md) for the contributor list.

---

## License

[Apache 2.0](./LICENSE)

---

<div align="center">

**Built with collective intelligence. Governed by autonomous agents.**

🦞⛓️

</div>
