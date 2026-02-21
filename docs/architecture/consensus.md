# Consensus Mechanism

ClawChain uses **Nominated Proof-of-Stake (NPoS)** consensus, combining BABE for block production and GRANDPA for finality — the same proven model used by Polkadot and Kusama.

---

## Overview

| Component | Mechanism | Purpose |
|-----------|-----------|---------|
| **Block Production** | BABE (Blind Assignment for Blockchain Extension) | Determines which validator produces each block |
| **Finality** | GRANDPA (GHOST-based Recursive ANcestor Deriving Prefix Agreement) | Provides deterministic, accountable finality |
| **Validator Selection** | NPoS (Nominated Proof of Stake) | Selects the active validator set each era |

---

## NPoS Validator Selection

ClawChain uses Nominated Proof of Stake to select validators:

1. **Validators** bond CLAW tokens and declare intent to validate
2. **Nominators** bond CLAW and nominate trusted validators
3. **Election algorithm** (Sequential Phragmén) selects the active set each era, optimizing for even stake distribution
4. **Rewards** are distributed proportionally based on stake and performance

### Staking Parameters (Testnet)

| Parameter | Value |
|-----------|-------|
| Session length | 100 blocks (~10 minutes) |
| Sessions per era | 6 (~1 hour) |
| Bonding duration | 28 eras (~28 hours) |
| Min. validator bond | 10,000 CLAW |
| Min. nominator bond | 100 CLAW |
| Max. validators | 100 |
| Max. nominators per validator | 64 |
| Annual inflation | ~10% (via reward curve) |

---

## BABE Block Production

BABE assigns block production slots to validators using a Verifiable Random Function (VRF):

1. Each slot (~6 seconds), validators evaluate their VRF output against a threshold
2. If below threshold, the validator is authorized to produce a block for that slot
3. Multiple validators may be assigned (primary + secondary slots prevent empty blocks)
4. Block authors are rewarded with CLAW from inflation

**Properties:**
- Probabilistic slot assignment (unpredictable who produces next)
- Secondary slot fallback prevents missed blocks
- Fork choice: longest chain rule

---

## GRANDPA Finality

GRANDPA provides deterministic finality separate from block production:

1. Validators vote on chains, not individual blocks
2. Votes cascade — voting for block N implies voting for all ancestors
3. When ⅔+ of validators agree on a chain prefix, those blocks are **finalized**
4. Finalized blocks are irreversible — no reorgs possible

**Properties:**
- Finalizes multiple blocks at once (catches up efficiently)
- Tolerates up to ⅓ Byzantine validators
- Accountable — equivocating validators can be identified and slashed
- Typical finality: 2–3 seconds after block production

---

## Security Model

### Slashing

Validators face economic penalties for misbehavior:

| Offense | Penalty |
|---------|---------|
| **Equivocation** (producing two blocks in one slot) | Up to 100% of stake |
| **GRANDPA equivocation** (voting for conflicting chains) | Up to 100% of stake |
| **Unresponsiveness** (offline for extended period) | Progressive slashing |

Slashed funds are sent to the Treasury.

### Treasury

The on-chain Treasury accumulates funds from:
- Slashed validator stakes
- A portion of block rewards (reward remainder)
- Future: transaction fee burns

Treasury spending is governed on-chain (planned: quadratic governance via `pallet-quadratic-governance`).

---

## Current Testnet State

The testnet is currently operating with a limited validator set:

- **Active validators:** Genesis authorities (Alice, Bob)
- **External validators:** Applications open — see [Validator Setup Guide](../guides/validator-setup.md)
- **Target:** 10+ external validators by end of Q2 2026, 100+ for mainnet

### Mainnet Progression

| Phase | Validators | Consensus |
|-------|-----------|-----------|
| Testnet Alpha (current) | 1–2 (genesis) | NPoS with invulnerables |
| Testnet Beta (Q2 2026) | 10–50 | Full NPoS election |
| Mainnet (Q3 2026) | 100+ | Production NPoS, sudo removed |

---

## Further Reading

- **[Architecture Overview](./overview.md)** — Full system architecture
- **[Validator Setup](../guides/validator-setup.md)** — Run a validator node
- **[Pallets Reference](./pallets.md)** — Staking and governance pallets
- [Polkadot NPoS Documentation](https://wiki.polkadot.network/docs/learn-phragmen)
- [BABE Specification](https://research.web3.foundation/Polkadot/protocols/block-production/Babe)
- [GRANDPA Paper](https://github.com/nickclaw/finality-grandpa)
