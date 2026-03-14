# ClawChain Validator Incentive Structure

> **Version:** 1.0 — Testnet Phase  
> **Last Updated:** 2026-03-10  
> **Audience:** Prospective and active validators evaluating the economics of running a ClawChain node.

---

## Overview

ClawChain allocates **30% of the total supply (300M CLAW)** to validator rewards. Validators secure the network through Proof of Authority (PoA) consensus initially, transitioning to Nominated Proof of Stake (NPoS) as the network matures.

| Pool | Allocation | CLAW |
|---|---|---|
| Validator rewards | 30% | 300,000,000 |
| Community airdrop | 40% | 400,000,000 |
| Treasury | 20% | 200,000,000 |
| Team & advisors | 10% | 100,000,000 |
| **Total supply** | **100%** | **1,000,000,000** |

---

## 1. Airdrop Tiers

Early validators receive bonus CLAW based on when they join and how long they maintain uptime.

### Testnet Airdrop (Pre-Mainnet)

| Tier | Requirement | Reward | Cap |
|---|---|---|---|
| **Pioneer** | Join within first 2 weeks of testnet | 50,000 CLAW + NFT badge | 20 validators |
| **Early Adopter** | Join within first month | 25,000 CLAW | 50 validators |
| **Contributor** | Join within first quarter | 10,000 CLAW | 200 validators |
| **Participant** | Any testnet validator | 2,000 CLAW | Unlimited |

### Uptime Bonuses

| Uptime (30 days) | Bonus Multiplier |
|---|---|
| ≥ 99.9% | 2.0× |
| ≥ 99.0% | 1.5× |
| ≥ 95.0% | 1.2× |
| ≥ 90.0% | 1.0× (base) |
| < 90.0% | 0.5× (reduced) |

**Example:** A Pioneer with 99.9% uptime receives `50,000 × 2.0 = 100,000 CLAW` at mainnet genesis.

### Mainnet Transition Airdrop

All testnet validators with ≥ 7 days of uptime receive their accumulated airdrop at mainnet genesis block. The airdrop is automatically bonded with a 28-day unbonding period.

---

## 2. APY Model

Validator rewards come from two sources: **block rewards** (inflationary) and **transaction fees**.

### Block Rewards (Year 1)

| Parameter | Value |
|---|---|
| Annual inflation target | 5% of circulating supply |
| Year 1 inflation pool | ~50,000,000 CLAW |
| Validators' share | 80% of inflation (40M CLAW) |
| Treasury's share | 20% of inflation (10M CLAW) |

### Expected APY by Validator Count

APY is calculated as: `(annual_validator_pool / total_staked) × 100`

| Active Validators | Total Staked (est.) | APY | Monthly per Validator |
|---|---|---|---|
| 10 | 10M CLAW | ~400% | ~333,333 CLAW |
| 25 | 25M CLAW | ~160% | ~133,333 CLAW |
| 50 | 75M CLAW | ~53% | ~66,667 CLAW |
| 100 | 200M CLAW | ~20% | ~33,333 CLAW |
| 250 | 500M CLAW | ~8% | ~13,333 CLAW |

> **Note:** Early validators benefit from high APY due to low validator count. APY decreases naturally as more validators join and more CLAW is staked.

### Commission Structure

Validators set their own commission rate (0–100%) on rewards earned from nominators:

| Commission | Validator Keeps | Nominators Receive |
|---|---|---|
| 0% | Own-stake rewards only | 100% of their proportional share |
| 5% | 5% of all nominator rewards + own-stake | 95% of their share |
| 10% | 10% of all nominator rewards + own-stake | 90% of their share |
| 100% | All rewards | Nothing beyond their own stake |

**Recommended testnet commission:** 5–10% (encourages nominators while compensating node operators).

### Fee Distribution

Transaction fees are distributed per block:

| Recipient | Share |
|---|---|
| Block author (validator) | 80% |
| Treasury | 20% |

During testnet, transaction volume is low, so fees are a minor component.

---

## 3. Slashing Conditions

Slashing penalizes validators who act maliciously or fail to maintain their nodes. Slashed funds go to the Treasury.

| Offense | Severity | Slash Amount | Consequence |
|---|---|---|---|
| **Equivocation (double signing)** | Critical | 10% of bonded stake | Immediate removal from active set |
| **GRANDPA equivocation** | Critical | 10% of bonded stake | Immediate removal from active set |
| **Unresponsiveness (offline)** | Moderate | 0.1% per era offline | Warning after 2 consecutive eras |
| **Prolonged downtime (>7 eras)** | High | 1% of bonded stake | Must re-register to validate |

### Testnet Slashing Policy

- **Reduced severity:** All slash amounts are 10× lower than mainnet values during testnet
- **Grace period:** First offense within 28 days triggers a warning only (no actual slash)
- **Recovery:** Slashed validators can re-bond and re-register after the cooldown period
- **Nominator impact:** Nominators backing a slashed validator lose stake proportional to their nomination share

### Slashing Protection

To minimize slashing risk:

1. **Use the Docker entrypoint's `AUTO_KEY_GEN=true`** — prevents key duplication
2. **Never run two validator instances with the same keys** — this is equivocation
3. **Monitor uptime** — use Prometheus + Grafana alerts (see [VALIDATOR.md](VALIDATOR.md))
4. **Ensure clean shutdowns** — always `docker compose down` before maintenance
5. **Key rotation** — rotate session keys if you suspect compromise

---

## 4. Unbonding

When validators or nominators want to withdraw their bonded CLAW, they enter an unbonding period.

| Parameter | Testnet | Mainnet |
|---|---|---|
| **Unbonding period** | 7 eras (~7 days) | 28 eras (~28 days) |
| **Minimum bond** | 1,000 CLAW | TBD (governance) |
| **Rewards during unbonding** | None | None |
| **Transferability during unbonding** | Locked | Locked |

### Unbonding Process

1. **Initiate unbonding:** Call `staking.chill()` (stop validating) then `staking.unbond(amount)`
2. **Wait for unbonding period:** Tokens are locked, earn no rewards, cannot be transferred
3. **Withdraw:** After the unbonding period ends, call `staking.withdrawUnbonded()` to release tokens
4. **Tokens become liquid:** Free to transfer, re-bond, or use elsewhere

### Rebonding

During the unbonding period, you can reverse the decision:
```
staking.rebond(amount)
```
This immediately re-bonds the specified amount and resumes reward eligibility at the next era.

---

## 5. Governance Participation

Validators have additional governance influence:

| Action | Requirement | Benefit |
|---|---|---|
| **Council voting** | Active validator | 2× voting weight |
| **Proposal submission** | Bond ≥ 10,000 CLAW | Can propose runtime upgrades |
| **Technical committee** | Nominated by council | Emergency governance powers |

---

## 6. Getting Started

1. **Set up your validator:** Follow [docs/VALIDATOR.md](VALIDATOR.md) for the Docker quick-start
2. **Get test CLAW:** Use the [faucet](../faucet/) to get 1,000 CLAW per day
3. **Bond and register:** Follow the registration steps in [VALIDATOR.md → Register as Validator](VALIDATOR.md#register-as-validator)
4. **Monitor:** Enable the monitoring stack for uptime tracking
5. **Earn rewards:** Maintain high uptime to maximize your airdrop multiplier

---

## FAQ

**Q: When does mainnet launch?**  
A: No firm date yet. Testnet validators will receive advance notice and priority onboarding.

**Q: Can I run multiple validators?**  
A: Yes, each with a separate bond and unique session keys. Never share keys between instances.

**Q: What happens if I stop validating?**  
A: Your testnet airdrop accrues only for the time you were actively validating with sufficient uptime.

**Q: Is there a minimum hardware requirement?**  
A: See [VALIDATOR.md → Prerequisites](VALIDATOR.md#prerequisites). 4 cores, 8GB RAM, 100GB SSD minimum.

**Q: How do I check my current rewards?**  
A: Via [Polkadot.js Apps](https://polkadot.js.org/apps/) → **Network → Staking → Payouts**.
