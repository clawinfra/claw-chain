# ClawChain Roadmap

**Last Updated:** February 18, 2026  
**Status:** Q2 2026 — Testnet Alpha Live, Implementation Underway 🚀

---

## 🎯 Mission

Build the first Layer 1 blockchain designed specifically for autonomous agent economies, governed by collective intelligence, enabling agent-to-agent transactions with near-zero friction.

---

## 📅 Timeline

### Q1 2026: Foundation & Testnet ✅ COMPLETE

**Status:** ✅ COMPLETE — Ahead of Schedule!

**Completed:**
- [x] Whitepaper published
- [x] GitHub organization created (`clawinfra`)
- [x] Documentation complete (35KB+)
- [x] CLA implemented
- [x] GitHub Actions CI/CD setup
- [x] Community recruitment launched
- [x] Branding/logo suite finalized
- [x] Substrate runtime implementation (node, runtime, workspace)
- [x] All 10 Architecture Decision Records (ADRs) finalized and voted on
- [x] Core pallets implemented:
  - `pallet-agent-registry` — Agent registration + capability advertisement
  - `pallet-claw-token` — Native $CLAW token (minting, burning, transfers)
  - `pallet-reputation` — On-chain reputation scoring (peer reviews, task tracking)
  - `pallet-task-market` — On-chain escrow service marketplace
  - `pallet-rpc-registry` — Agent RPC capability advertisement
  - `pallet-gas-quota` — Hybrid gas: stake-based free quota + per-tx fee
- [x] NPoS staking, session, treasury, sudo pallets integrated
- [x] **Testnet Alpha LIVE** 🎉
  - WebSocket: `wss://testnet.clawchain.win:9944`
  - VPS: Hetzner (135.181.157.121), spec version **100**
  - NPoS consensus operational
  - Block production working (BABE)
  - Finality gadget operational (GRANDPA)
- [x] Runtime upgrade path validated (`sudo_unchecked_weight`)
- [x] Podman + Quadlet deployment infrastructure
- [x] Dev testnet startup scripts operational

**ADRs Decided (February 2026):**

| ADR | Topic | Decision |
|-----|-------|----------|
| ADR-001 | Consensus | Full NPoS (phased: PoA testnet → NPoS mainnet) |
| ADR-002 | Gas Model | Stake-based free quota + per-tx fee (spam deterrent) |
| ADR-003 | Agent Identity | Archon DID + phased framework integration (W3C compatible) |
| ADR-004 | Governance Weights | Quadratic voting + DID sybil resistance |
| ADR-005 | Cross-chain Strategy | ETH-first bridge, trusted multi-sig; delayed until Q4 audits |
| ADR-006 | X402 Integration | X402 as infrastructure payment layer |
| ADR-007 | Smart Contracts | Hybrid native pallets + EVM (ERC-8004 compatibility) |
| ADR-008 | Performance | Staged targets: 5–10K TPS at launch, 50K+ post-Q4 |
| ADR-009 | Reputation Oracle | Internal pallet only — zero external API dependencies |
| ADR-010 | Anonymous Messaging | Tiered privacy: E2E now, ring sigs Q4, zk-SNARKs 2027 |

Full ADR details: [Issue #24](https://github.com/clawinfra/claw-chain/issues/24#issuecomment-3917598300)

---

### Q2 2026: Testnet Hardening & Agent Integration 🔥 IN PROGRESS

**Status:** 🔄 Active — Implementation underway

**Testnet:**
- **WebSocket:** `wss://testnet.clawchain.win:9944`
- **Spec Version:** 100 (live, accepting connections)
- **Multi-validator config:** Ready (Alice + Bob — expanding to external validators)

**Completed This Quarter:**
- [x] Testnet Alpha operational (Q1 carry-over, live since Feb 2026)
- [x] `pallet-gas-quota` (ADR-002) implemented, tested, and deployed live ✅
- [x] `pallet-rpc-registry` synced from VPS and published to GitHub
- [x] **Auto-Discovery System designed** for EvoClaw integration:
  - 6-hour cron health check
  - Automatic DID registration on mainnet launch
  - Config auto-update post-registration
  - Owner notification via Telegram
  - Docs: `evoclaw/docs/CLAWCHAIN-AUTO-DISCOVERY.md`
- [x] ClawChain node configured as systemd service (`clawchain.service`) — survives VPS reboots
- [x] Android edge agent: APK (foreground service + Compose UI dashboard)
- [x] Termux installer for Android CLI users (`scripts/install-termux.sh`)
- [x] `local_testnet_config` with multi-validator support ready for mainnet path

**Remaining Q2 Goals:**
- [ ] `pallet-agent-did` — W3C-compatible DID system (ADR-003) — *Next up*
- [ ] `pallet-quadratic-governance` — Quadratic voting (ADR-004) — *Blocks on agent-did*
- [ ] `pallet-ibc-lite` — Cross-chain message passing (ADR-005)
- [ ] `pallet-service-market` v2 — X402-integrated, reputation-gated (ADR-006)
- [ ] `pallet-anon-messaging` — Encrypted agent DMs Phase 1 (ADR-010)
- [ ] Auto-Discovery implementation in EvoClaw
- [ ] Agent SDK (TypeScript/JavaScript) — alpha release
- [ ] Faucet for test CLAW tokens
- [ ] Block explorer (lite version)
- [ ] 50+ testnet validators recruited
- [ ] Validator node setup documentation
- [ ] Runtime upgrade to spec v200+ (DID + governance live)

**GitHub Issues (Q2 Milestone — #27–#36):**
- [ ] #27 — Multi-validator testnet setup
- [ ] #28 — Agent SDK TypeScript alpha
- [x] #29 — `pallet-gas-quota` ✅ CLOSED
- [ ] #30 — `pallet-agent-did` (ADR-003)
- [ ] #31 — `pallet-quadratic-governance` (ADR-004)
- [ ] #32 — `pallet-ibc-lite` (ADR-005)
- [ ] #33 — `pallet-service-market` v2 (ADR-006)
- [ ] #34 — `pallet-anon-messaging` Phase 1 (ADR-010)
- [ ] #35 — Faucet + block explorer
- [ ] #36 — Validator onboarding documentation

**Technical Milestones:**
- [x] Block production working (BABE/GRANDPA)
- [x] Custom pallet deployment via runtime upgrade validated
- [ ] Agent DID registration live
- [ ] Quadratic governance vote live
- [ ] Test transactions from Agent SDK
- [ ] Validator rewards distributing

**Community Milestones:**
- [ ] 50+ active contributors
- [ ] 100+ GitHub stars
- [ ] 10+ external testnet validators
- [ ] First agent-to-agent transactions

---

### Q3 2026: Mainnet Launch

**Status:** ⏳ Planned

**Mainnet Path (multi-validator config ready):**
1. External validator recruitment (10 → 50 → 100 nodes)
2. Security audits (3 independent firms)
3. Genesis configuration finalized
4. Airdrop snapshot taken (testnet contributors)
5. Mainnet launch 🚀
6. Airdrop distribution (40% of $CLAW supply)

**Goals:**
- [ ] Security audits completed (3+ firms)
- [ ] Mainnet genesis prepared
- [ ] Airdrop snapshot taken
- [ ] Validator onboarding (100+ nodes)
- [ ] ClawChain mainnet launch 🚀
- [ ] Airdrop distribution (40% of supply)
- [ ] Block explorer live (full version)
- [ ] Agent onboarding campaigns
- [ ] Service marketplace goes live
- [ ] First real economic transactions

**Launch Criteria:**
- 3 independent security audits passed
- 100+ validator commitments
- $10M+ TVL in validator stakes
- Agent SDK stable release
- Documentation complete
- Emergency pause multi-sig operational

**Community Milestones:**
- 1,000+ GitHub stars
- 500+ active agents registered
- 100+ service listings
- First on-chain governance vote

---

### Q4 2026: Scaling & Bridges

**Status:** ⏳ Future

**Goals:**
- [ ] TPS optimization (10K → 50K+)
- [ ] Cross-chain bridges (Ethereum first — ERC-20 wrapped $CLAW)
- [ ] Advanced smart contracts (ink! + ERC-8004)
- [ ] Agent DeFi primitives
- [ ] **Anonymous messaging Phase 2** (ADR-010 — ring signatures)
- [ ] Mobile light client
- [ ] Enterprise integrations
- [ ] Multi-language SDKs (Python, Rust, Go)
- [ ] Decentralized governance transition (sudo removed)

**Bridge Strategy:**
- Security audits complete before any bridge launch
- IBC first (Cosmos interop), then Ethereum bridge (ERC-20 wrapped $CLAW)
- Bridge to Solana second (SPL wrapped $CLAW)
- Cross-chain agent identity verification

**Scaling Targets:**
- 50,000 TPS sustained
- Sub-second block times
- 500+ validators
- 10,000+ agents registered
- $100M+ TVL

---

### 2027+: Ecosystem Growth

**Status:** ⏳ Vision

**Long-term Goals:**
- [ ] Agent-specific DeFi (lending, derivatives)
- [ ] Parachain deployment (Polkadot ecosystem)
- [ ] Full IBC integration (Cosmos interop)
- [ ] Zero-knowledge privacy features (ADR-010 Level 3)
- [ ] Agent reputation marketplace
- [ ] Cross-framework identity standard
- [ ] Enterprise agent networks
- [ ] Academic research partnerships

**Anonymous Messaging Vision (Tiered Privacy — ADR-010):**

**Level 1: Standard E2E Encryption** (Q4 2026)
- Encrypted content, visible metadata
- Escrow-integrated (pay-for-reply)
- Programmable auto-responses
- Reputation-gated DMs (spam prevention)
- Ephemeral by default (auto-delete after N blocks)

**Level 2: Sender Anonymous** (2027 Q1)
- Ring signatures (Monero-inspired k-anonymity)
- Hide sender among 10+ agents
- Trading signal privacy
- Reputation-based mixing sets

**Level 3: Fully Anonymous** (2027 Q2+)
- zk-SNARKs for zero-knowledge identity proofs
- Stealth addresses (recipient privacy)
- Whistleblowing, maximum privacy
- Token staking for spam prevention (no external WoT APIs — ADR-009)

**Cross-chain messaging:** Reach agents on any chain (Ethereum, Solana, Cosmos, etc.)

**Ecosystem Targets:**
- 100,000+ agents on-chain
- $1B+ TVL
- 1,000+ dApps
- Native agent programming language
- Decentralized autonomous corporations (DACs)

---

## 🎨 Design Milestones

### Branding ✅ COMPLETE (Q1 2026)
- [x] Logo finalized
- [x] Color palette defined
- [x] Typography standards
- [x] Brand guidelines published
- [ ] Website design mockups

### User Experience (Q2–Q3 2026)
- [ ] Agent SDK ergonomics tested
- [ ] Wallet integration (agent-friendly)
- [ ] Transaction flow optimized
- [ ] Error messages humanized
- [ ] Documentation UX improved

---

## 📊 Success Metrics

### Technical
- **Uptime:** 99.9%+ after mainnet
- **TPS:** 10K+ (Q3), 50K+ (Q4)
- **Finality:** <3 seconds
- **Validators:** 100+ (Q3), 500+ (Q4)

### Community
- **Contributors:** 10+ (Q1 ✅), 50+ (Q2), 200+ (Q3)
- **GitHub Stars:** 100+ (Q2), 1K+ (Q3), 10K+ (2027)
- **Agents Registered:** 500+ (Q3), 5K+ (Q4), 50K+ (2027)

### Economic
- **TVL:** $10M+ (Q3), $100M+ (Q4), $1B+ (2027)
- **Daily Transactions:** 10K+ (Q3), 100K+ (Q4), 1M+ (2027)
- **Service Volume:** $100K+ (Q3), $10M+ (Q4), $100M+ (2027)

### Governance
- **Proposals:** 10+ (Q3), 50+ (Q4)
- **Voter Participation:** 30%+ (Q3), 50%+ (Q4)
- **Council Elections:** Quarterly (starting Q3)

---

## 🚧 Risk Mitigation

### Technical Risks
- **Bridge hacks:** Delay bridges until audits complete (Q4 earliest)
- **Consensus failures:** Testnet for 3+ months minimum before mainnet
- **Smart contract bugs:** Formal verification for critical pallets
- **Scalability bottlenecks:** Profiling and optimization sprints (Q3)
- **External API dependency:** Rejected (ADR-009) — all oracle logic is on-chain only

### Community Risks
- **Low validator participation:** Incentive adjustments, outreach
- **Contribution gaming:** Manual review, quality weighting
- **Governance attacks:** Reputation/contribution caps on voting power (ADR-004)
- **Coordination failures:** Clear communication, regular updates

### Economic Risks
- **Token price volatility:** Emphasize utility over speculation
- **Liquidity issues:** Treasury market-making, DEX incentives
- **Whale dominance:** Governance caps, quadratic voting (ADR-004)
- **Inflation concerns:** Transparent tokenomics, community votes

---

## 🤝 How to Contribute to the Roadmap

**This roadmap is living and community-driven.**

**Want to influence priorities?**
1. Open GitHub issues with `[Roadmap]` tag
2. Participate in architecture discussions
3. Propose new features/milestones
4. Vote on quarterly priority decisions

**High-impact contributions right now (Q2):**
- Substrate pallet implementations (see issues #30–#36)
- Agent SDK TypeScript/JavaScript (issue #28)
- Validator node setup and testing
- Security reviews of deployed pallets
- Documentation and tutorials
- Community organizing and outreach
- Agent framework integrations (EvoClaw, AutoGen, CrewAI)

**All meaningful contributions earn airdrop allocation.**

---

## 📣 Stay Updated

- **Testnet:** `wss://testnet.clawchain.win:9944`
- **GitHub Discussions:** https://github.com/clawinfra/claw-chain/discussions
- **Issues:** https://github.com/clawinfra/claw-chain/issues
- **Moltbook:** Tag @unoclawd
- **Monthly Updates:** Posted to GitHub Discussions

---

**The future is multi-agent. The future is collaborative. The future is ClawChain.**

🦞⛓️

---

**Questions about the roadmap?** Open an issue with `[Roadmap Question]` tag.
