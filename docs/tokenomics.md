# Tokenomics

**Token:** CLAW | **Type:** Native L1 | **Total Supply:** 1,000,000,000 | **Decimals:** 12

---

## Distribution

| Allocation | Share | Amount | Vesting |
|-----------|-------|--------|---------|
| **Community Airdrop** | 40% | 400,000,000 CLAW | 25% immediate, 75% linear over 12 months |
| **Validator Rewards** | 30% | 300,000,000 CLAW | Continuous via block rewards |
| **Treasury** | 20% | 200,000,000 CLAW | 10% immediate, 90% over 24 months |
| **Founding Contributors** | 10% | 100,000,000 CLAW | 6-month cliff, 24-month linear vest |

```
            ┌─────────────────────────────────────────┐
            │        Total Supply: 1B CLAW            │
            ├──────────┬─────────┬─────────┬──────────┤
            │ Airdrop  │Validator│Treasury │  Team    │
            │   40%    │  30%    │  20%    │  10%     │
            │  400M    │  300M   │  200M   │  100M    │
            └──────────┴─────────┴─────────┴──────────┘
```

---

## Inflation

New CLAW tokens are minted each era to reward validators:

| Year | Inflation Rate | New Tokens (approx.) |
|------|---------------|---------------------|
| 1 | 5.0% | 50M |
| 2 | 4.5% | 47M |
| 3 | 4.0% | 45M |
| 4 | 3.5% | 42M |
| 5 | 3.0% | 38M |
| 6 | 2.5% | 34M |
| 7+ | **2.0% floor** | ~30M+ |

The inflation curve is implemented on-chain via `pallet_staking_reward_curve` and distributes rewards automatically each era.

---

## Token Utility

### 1. Transaction Fees
ClawChain uses a hybrid gas model (`pallet-gas-quota`):
- **Stakers** receive a free transaction quota proportional to their stake
- **Non-stakers** pay standard per-transaction fees
- Near-zero effective cost for active participants

### 2. Staking
- **Validators:** Bond ≥ 10,000 CLAW to participate in consensus
- **Nominators:** Bond ≥ 100 CLAW and nominate validators
- **Rewards:** ~10% APY target, distributed each era

### 3. Governance
Voting power is a weighted function of stake, reputation, and contribution:
```
VotingPower = (ContributionScore × 0.4) + (Reputation × 0.3) + (Stake × 0.3)
```
Quadratic voting is implemented via `pallet-quadratic-governance` to prevent plutocratic control.

### 4. Service Payments
The task market (`pallet-task-market`) uses CLAW for:
- Task rewards (escrowed on posting)
- Bid deposits
- Dispute resolution stakes

---

## Airdrop Eligibility

### GitHub Contributors
```
Score = (Commits × 1,000) + (PRs × 5,000) + (Code Review × 2,000) + (Docs × 2,000)
```

### Early Validators
- First 100 validators to run testnet nodes
- Minimum 3 months uptime, 95%+ availability
- Up to 1,100,000 CLAW per validator

### Community
- Active governance participants
- Bug bounty reporters
- Documentation contributors

> **Cap:** No single airdrop exceeds 1% of total supply (10M CLAW).

---

## Treasury

The on-chain treasury (`pallet-treasury`) funds ecosystem development:
- Core development and infrastructure
- Security audits
- Community grants and bounties
- Marketing and growth

**Governance:** All treasury spending requires an on-chain proposal and vote with a 7-day timelock for large withdrawals (>1M CLAW).

---

## Fair Launch Principles

- **No pre-mine** — founding contributors vest over 24 months
- **No VC allocation** — community-driven, not venture-backed
- **Transparent distribution** — all allocations published on-chain
- **Vesting enforced** — smart contract-level vesting prevents dumps

---

## Further Reading

- **[Whitepaper — Full Tokenomics](../whitepaper/TOKENOMICS.md)** — Complete token economics specification
- **[Consensus Mechanism](./architecture/consensus.md)** — How staking rewards work
- **[Roadmap](../ROADMAP.md)** — Airdrop and mainnet timeline
