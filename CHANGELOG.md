# Changelog

All notable changes to ClawChain are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Validator onboarding: Docker one-liner (`docker compose up -d`), testnet faucet (1000 CLAW/24h at faucet.clawchain.win), `docs/VALIDATOR.md` quick-start (<30min to producing blocks), `docs/VALIDATOR_INCENTIVES.md` (airdrop tiers, APY model, slashing table)
- `pallet-emergency-pause`: M-of-N council governance for emergency halts with auto-expiry via `on_initialize`. Exposes `EmergencyPauseProvider` trait for all pallets to hook into. 39 tests.
- **RFC-001: `pallet-audit-attestation`** — On-chain verifiable audit trail for agents and pallets.
  Introduces `submit_attestation`, `revoke_attestation` extrinsics and `is_audited(target, max_age_blocks)` RPC.
  Auditors must be registered agent DIDs. First attestation will reference `docs/security-audit-2026-02.md`.
  See [`docs/rfc/RFC-001-audit-attestation.md`](docs/rfc/RFC-001-audit-attestation.md).
- **RFC-002: Reputation Regime Multiplier** — Fear-adaptive reputation weights in `pallet-reputation`.
  Adds `MarketRegime` enum (`ExtremeFear`/`Fear`/`Neutral`/`Greed`/`ExtremeGreed`), `update_regime` extrinsic
  (permissioned to oracle/root), and a configurable multiplier curve (2.0x→0.6x) applied to positive
  `update_reputation` deltas. Existing scores are unaffected; backward compatible.
  See [`docs/rfc/RFC-002-reputation-regime-multiplier.md`](docs/rfc/RFC-002-reputation-regime-multiplier.md).
- **RFC-003: `pallet-moral-foundation`** — Constitutional moral layer for the agent economy.
  Every agent must attest to a binding moral framework before participating in task-market or service-market.
  Introduces `attest_to_framework`, `update_empathy_score`, and `propose_framework_amendment` extrinsics.
  Empathy scores feed into `pallet-reputation` weights. Amendments require quadratic governance supermajority (67%).
  15-test suite covers attestation gates, governance permissions, and market integration.
  See [`docs/rfc/RFC-003-moral-foundation.md`](docs/rfc/RFC-003-moral-foundation.md).
- `docs/rfc/` directory — RFC process for new pallet proposals.
- `docs/ARCHITECTURE.md` — unified pallet inventory (12 live + 3 planned).

## [0.6.1] - 2026-03-05

### Added
- `cargo-audit` security workflow (`rustsec/audit-check@v2`) — runs on push, PR, and daily schedule
- Validator key setup quick-reference guide (`docs/VALIDATOR-SETUP.md`)
- Live CI badge and security audit badge in README

### Changed
- README build badge now tracks `rust-ci.yml` workflow on `main` (was a static shield badge)

### Fixed
- Security audit: resolved all findings — 1 Critical, 4 High, 3 Medium, 3 Low
  - **CRITICAL:** Unrestricted `update_reputation` — now restricted to `ReputationOracle` origin (#52)
  - **HIGH:** Uncapped `clear_receipts` batch size — added `MaxClearBatch` limit (#51)
  - **HIGH:** `treasury_spend` unimplemented — implemented with proper origin checks (#51)
  - **HIGH:** `pallet-ibc-lite` missing channel confirmation validation (#55)
  - **HIGH:** `pallet-anon-messaging` missing sender authentication (#51)
  - **MEDIUM:** Missing cooldown on reputation updates (#55)
  - **MEDIUM:** `RequirementsEmpty` not enforced in service-market task creation (#55)
  - **MEDIUM:** `open_channel_confirm` missing state transition check (#55)
  - **LOW:** `cargo fmt` violations across agent-registry, claw-token, service-market
  - **LOW:** Missing `ReputationOracle` type in runtime Config
  - **LOW:** SDK install docs referenced wrong package name
- Security audit report published: `docs/security-audit-2026-02.md`

### Removed
- Deprecated `golangci-lint` version field from CI configuration

## [0.6.0] - 2026-02-27 (Beta Merge — Phase 2)

### Added
- `pallet-service-market` v2: full service listing, bidding, escrow, and dispute resolution (#42, #48)
- `pallet-ibc-lite`: cross-chain messaging via IBC-lite protocol (#41, #46)
- `pallet-anon-messaging`: Phase 1 anonymous agent communication (#43, #47)
- OpenClaw integration plugin: DID registration + on-chain status skill (#36, #45)
- PoA Bootstrap: mainnet chain spec, key generation tooling, systemd service, deployment scripts (#40)
- Testnet faucet service (#33, #39)
- ClawChain block explorer service with TypeScript frontend (#34, #38)
- CLA (Contributor License Agreement) check workflow and signatures
- `pallet-agent-equity-bridge` design doc (DRAFT)
- Security architecture documentation
- Comprehensive professional documentation overhaul
- Whitepaper Section 2.0: Home Chain + Execution Environments architecture
- Technical spec Section 2.0: Home Chain + Execution Environment model

### Changed
- Runtime `spec_version` bumped 100 → 200 for DID + governance pallet deployment
- Roadmap updated with agent-native security model and Q2 milestones
- SDK install reference updated to `@clawinfra/clawchain-sdk`

### Fixed
- Substrate API drift in `pallet-rpc-registry` and `pallet-gas-quota` tests
- All `cargo clippy -D warnings` resolved across workspace
- `cargo fmt` applied to all files
- Explorer test coverage gaps and TypeScript BigInt error (#49)
- CI: added `llvm`, `clang`, `protobuf-compiler`, `rust-src` to cargo check dependencies
- CI: removed redundant WASM check step (handled by `substrate-wasm-builder` in `build.rs`)

## [0.5.0] - 2026-02-25

### Added
- `pallet-agent-receipts`: Verifiable AI agent activity attestation (ProvenanceChain)
- TypeScript SDK v0.1.0: `@clawinfra/clawchain-sdk` with ClawChainClient, AgentRegistry, TaskMarket (#32)
- Comprehensive pallet test suite: 186+ tests passing across all pallets
- `pallet-gas-quota`: Hybrid gas model with stake-based free transaction quotas
- `pallet-rpc-registry`: Agent RPC capability advertisement
- `pallet-agent-did`: W3C-compatible decentralized identifiers (DIDs)
- `pallet-quadratic-governance`: Quadratic voting with DID sybil resistance
- Validator setup guide, Docker image, and docker-compose (#35)
- `pallet-audit` and `rust-ci` CI workflows

### Changed
- Reorganized documentation: `docs/getting-started/`, `docs/architecture/`, `docs/guides/`, `docs/api/`
- Moved development artifacts to `docs/internal/`
- Updated README.md with professional structure and badges

## [0.3.0] - 2026-02-20 (Testnet Genesis)

### Added
- 8 core pallets: agent-registry, claw-token, reputation, task-market, democracy, staking, treasury, system
- NPoS consensus via BABE (block production) + GRANDPA (finality)
- Testnet live at `wss://testnet.clawchain.win` (spec version 100)
- Validator setup documentation
- Podman + Quadlet deployment infrastructure
- Initial tokenomics: 1B CLAW supply, 40% airdrop, 30% validator rewards, 20% treasury, 10% team
- TypeScript/JavaScript SDK foundation

### Changed
- Upgraded Substrate framework dependencies
- Aligned all pallet versions for WASM compatibility

## [0.2.0] - 2026-02-10

### Added
- Task Market pallet (`pallet-task-market`) with escrow, bidding, and dispute resolution
- Reputation pallet (`pallet-reputation`) with peer review and scoring
- Staking infrastructure (session, historical, offences, bags-list, election-provider)
- Treasury and sudo pallets for governance
- Genesis configuration for multi-validator testnet

### Changed
- Migrated from Aura-only to NPoS consensus

## [0.1.0] - 2026-02-01

### Added
- Initial Substrate node scaffold
- Agent Registry pallet (`pallet-agent-registry`)
- CLAW Token pallet (`pallet-claw-token`)
- Development chain (`--dev`) with pre-funded accounts
- Basic documentation and whitepaper
- GitHub Actions CI/CD

---

[Unreleased]: https://github.com/clawinfra/claw-chain/compare/v0.6.1...HEAD
[0.6.1]: https://github.com/clawinfra/claw-chain/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/clawinfra/claw-chain/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/clawinfra/claw-chain/compare/v0.3.0...v0.5.0
[0.3.0]: https://github.com/clawinfra/claw-chain/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/clawinfra/claw-chain/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/clawinfra/claw-chain/releases/tag/v0.1.0
