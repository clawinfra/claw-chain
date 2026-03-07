# AGENTS.md — ClawChain Agent Harness

ClawChain is a Substrate-based Layer 1 blockchain purpose-built for autonomous AI agents, providing
on-chain identity (DIDs), reputation scoring, task markets, token economics, and inter-agent messaging.
This file is a **table of contents** — not a reference manual. Follow the links.

---

## Repo Map

```
pallets/          — 12 domain pallets (one crate each)
runtime/          — runtime wiring (Config impls, pallet ordering)
node/             — substrate node binary (RPC, P2P, block production)
docs/             — architecture, quality, and convention docs (READ THESE FIRST)
chain-spec/       — genesis chain specs (dev, testnet, mainnet)
scripts/          — CI helpers, agent-lint, benchmarks runner
sdk/              — TypeScript/Rust SDK (external consumers)
services/         — off-chain services (oracle, relayer, indexer)
```

---

## Pallets (12 total)

| Pallet | Responsibility |
|--------|---------------|
| `agent-registry` | Agent identity, DID registration, lifecycle (Active/Suspended/Deregistered) |
| `agent-did` | W3C-compatible DID documents and verification methods |
| `agent-receipts` | Immutable on-chain receipts for agent task completions |
| `reputation` | Trust scores (0–10 000 bp), peer reviews, dispute outcomes |
| `task-market` | Task posting, bidding, escrow, completion, disputes |
| `service-market` | Recurring service listings and subscription management |
| `rpc-registry` | Agent-published RPC endpoints (discovery layer) |
| `claw-token` | Native CLAW token (transfer, lock, vesting, burn) |
| `gas-quota` | Per-agent gas allowances and quota enforcement |
| `quadratic-governance` | Quadratic-weighted governance proposals and voting |
| `ibc-lite` | Lightweight cross-chain messaging (IBC subset) |
| `anon-messaging` | Privacy-preserving agent-to-agent messaging |

---

## Docs (start here before touching code)

| File | What it covers |
|------|---------------|
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Pallet dependency graph, Config trait boundary, bounded storage rules, event requirements |
| [`docs/QUALITY.md`](docs/QUALITY.md) | Coverage targets, security-sensitive functions, benchmark requirements, migration rules |
| [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) | Naming rules for pallets, extrinsics, events, storage, weights |
| [`docs/EXECUTION_PLAN_TEMPLATE.md`](docs/EXECUTION_PLAN_TEMPLATE.md) | Template for planning complex tasks before implementing |

---

## How to Build & Test

```bash
# Full workspace build
cargo build --workspace

# Run all tests
cargo test --workspace

# Run lints (must pass before PR)
cargo clippy --workspace -- -D warnings

# Run agent-specific lints (architectural invariants)
bash scripts/agent-lint.sh

# Run benchmarks (required before mainnet extrinsics)
cargo test --features runtime-benchmarks -p pallet-<name>
```

---

## Agent Invariants (non-negotiable)

1. **Always run `cargo test --workspace` before opening a PR.** Never break existing tests.
2. **Check `docs/ARCHITECTURE.md` before adding any cross-pallet dependency.** Pallets must not import
   each other directly — all cross-pallet calls go through the runtime Config trait.
3. **All new extrinsics must have weight annotations.** Un-weighted extrinsics are a DoS vector.
4. **All storage items must use `BoundedVec<T, MaxLen>` not `Vec<T>`.** See Architecture doc.
5. **All state changes must emit events.** Silent state changes make off-chain indexers blind.
6. **Security-sensitive extrinsics** (`update_reputation`, `treasury_spend`, `invoke_service`)
   must validate origin with `ensure_root!` or a custom `AuthorityOrigin`. See Quality doc.
7. **Run `bash scripts/agent-lint.sh` locally.** It catches invariant violations with fix instructions.
8. **For complex tasks** (>3 pallets affected, new cross-chain feature, storage migration),
   create an execution plan using `docs/EXECUTION_PLAN_TEMPLATE.md` before writing code.

---

## CI Gates

Every PR runs:
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- `bash scripts/agent-lint.sh` (architectural invariants)

All three must pass. Failures include remediation instructions — read them.

---

*This file must stay under 150 lines. See `scripts/agent-lint.sh` Rule 5.*
