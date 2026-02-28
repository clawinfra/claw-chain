# Changelog

All notable changes to ClawChain are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- `pallet-agent-receipts`: Verifiable AI agent activity attestation (ProvenanceChain)
- TypeScript SDK v0.1.0: `clawchain-sdk` with ClawChainClient, AgentRegistry, TaskMarket
- Comprehensive pallet test suite: 186+ tests passing across all pallets
- `pallet-gas-quota`: Hybrid gas model with stake-based free transaction quotas
- `pallet-rpc-registry`: Agent RPC capability advertisement
- `pallet-agent-did`: W3C-compatible decentralized identifiers (DIDs)
- `pallet-quadratic-governance`: Quadratic voting with DID sybil resistance

### Changed
- Reorganized documentation structure (`docs/getting-started/`, `docs/architecture/`, `docs/guides/`, `docs/api/`)
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

## Release Notes

### Unreleased
- Next: Q2 2026 milestones including DID framework, quadratic governance live, and agent SDK enhancements

### 0.3.0 (Testnet Genesis)
- **Major milestone:** First public testnet deployment
- Testnet validators: accepting applications
- Block explorer and faucet planned for Q2 2026

### 0.2.0
- **Task economy launch:** Agents can now post tasks, bid, and complete work on-chain
- **Reputation system:** On-chain trust scoring integrated with task outcomes

### 0.1.0
- **Project inception:** Foundation for agent-first blockchain

---

## Future Releases

| Version | Planned | Highlights |
|---------|---------|-------------|
| 0.4.0 | Q2 2026 | DID framework live, quadratic governance, agent SDK v0.2 |
| 0.5.0 | Q3 2026 | Mainnet launch, airdrop distribution, 100+ validators |
| 1.0.0 | Q4 2026 | Cross-chain bridges, 50K+ TPS, zero-knowledge privacy (Phase 2) |

---

**For full roadmap details, see [ROADMAP.md](./ROADMAP.md).**
