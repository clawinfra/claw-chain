# pallet-agent-equity-bridge — Design Document

**Status:** DRAFT — Under review, no implementation until architecture is validated  
**Author:** Alex Chen  
**Created:** 2026-02-25  
**Inspired by:** Backpack Exchange's token-to-equity model (announced Feb 2025)

---

## 1. Overview

`pallet-agent-equity-bridge` enables AI agent token stakers on ClawChain to earn **equity stakes in the underlying agent's DAO or operating entity**. It creates the first credible crypto-to-equity bridge designed specifically for autonomous AI agents.

### Problem Statement

AI agents accumulating on-chain value have no mechanism to give their token stakers real ownership in the agent's operations. Token stakers get governance rights (via `pallet-quadratic-governance`) but not economic equity in the agent itself. This limits agent token adoption for serious investors.

### Solution

A conversion mechanism where:
- Stakers lock `$CLAW` or agent-specific tokens for a fixed period (12 months)
- At maturity, they receive an `EquityReceipt` NFT representing a proportional equity share in the agent's registered DAO
- The DAO distributes revenue to `EquityReceipt` holders pro-rata

---

## 2. Staking Architecture

### 2.1 Stake Flow

```
Staker → lock(amount, agent_did, 12mo) 
       → StakeRecord created (on-chain)
       → At maturity: claim() 
       → EquityReceipt minted (NFT)
       → EquityReceipt holder receives DAO revenue distributions
```

### 2.2 Conversion Rate

The stake→equity conversion rate is set by the agent's DAO at registration time and can only be updated via governance vote:

```
equity_share = stake_amount / total_staked_at_lock * agent_equity_pool_pct
```

- `agent_equity_pool_pct`: % of agent's DAO equity allocated to the staking pool (e.g. 20%)
- Dilution protection: new stakers enter at a lower rate as the pool fills
- Maximum pool cap per agent: configurable (default 30% of DAO equity)

### 2.3 Lockup

- **Minimum lockup:** 90 days (short-term liquidity tier, no equity — governance rights only)
- **Equity lockup:** 12 months (full equity conversion at maturity)
- **Early exit penalty:** 50% of stake slashed to DAO treasury; no EquityReceipt issued
- **Extension option:** Stakers can voluntarily extend lockup to increase equity share (1.5x multiplier for 24-month lockup)

---

## 3. EquityReceipt NFT

### 3.1 Spec

Each `EquityReceipt` is a non-transferable (soulbound) NFT on ClawChain containing:

```rust
pub struct EquityReceipt {
    pub id: ReceiptId,
    pub agent_did: AgentDid,
    pub holder: AccountId,
    pub equity_bps: u32,       // basis points of agent DAO equity (e.g. 100 = 1%)
    pub issued_at: BlockNumber,
    pub stake_amount: Balance,
    pub lockup_months: u8,
}
```

**Soulbound rationale:** Equity in an AI agent's DAO shouldn't be freely tradeable in MVP — this avoids regulatory classification as a security in most jurisdictions. Secondary market support can be added post-legal-review.

### 3.2 Revenue Distribution

Agent DAOs call `distribute_revenue(amount)` on a quarterly basis:
- Pallet calculates each holder's share: `amount * equity_bps / total_equity_bps`
- Distributions accumulate in a claimable balance (no forced push)
- Holders call `claim_distribution(receipt_id)` to withdraw

---

## 4. Agent DAO Registration

For an agent to participate, its DAO must be registered via this pallet:

```rust
pub fn register_agent_equity_pool(
    origin: OriginFor<T>,
    agent_did: AgentDid,
    equity_pool_pct: u8,      // % of DAO equity to offer (max 30)
    conversion_rate: u128,     // CLAW per basis point of equity
    dao_treasury: AccountId,   // receives early-exit penalties
) -> DispatchResult
```

Registration requires:
1. Agent must have a verified DID in `pallet-agent-did`
2. Agent must have a governance instance in `pallet-quadratic-governance`
3. DAO governance vote approving the equity pool parameters (quorum: 66%)

---

## 5. Open Questions (Validate Before Implementation)

These must be answered before writing any code:

### 5.1 Legal / Regulatory
- **Does an EquityReceipt constitute a security?** In most jurisdictions, profit-sharing instruments are securities. Soulbound + DAO structure may qualify for utility token exemptions, but needs legal review.
- **Jurisdiction:** Where is the "agent's DAO" domiciled? Marshall Islands DAO LLC structure is most crypto-friendly. This needs to be standardised across all agent registrations.
- **KYC requirement:** Does equity issuance require KYC of stakers? If so, this conflicts with on-chain pseudonymity.

### 5.2 Technical
- **Soulbound vs transferable:** Soulbound protects against security classification but limits liquidity. Should we support a secondary AMM with legal wrappers post-MVP?
- **Oracle for revenue:** How do we verify an agent DAO's off-chain revenue for on-chain distribution? Chainlink proof-of-reserve, or self-reported with dispute mechanism?
- **Multi-chain staking:** Can stakers on Ethereum/Base stake and receive ClawChain equity receipts? Requires cross-chain messaging (LayerZero/Wormhole) — defer to Phase 2.
- **Slashing mechanics:** Early-exit slash (50%) — should slashed funds go to stakers or DAO treasury? Current spec: DAO treasury (simpler), but staker-redistribution is more equitable.

### 5.3 Economics
- **Conversion rate stability:** If $CLAW price rises 10x, the equity price also rises 10x in USD terms. Should rate be denominated in USD-stable terms? Needs modelling.
- **Dilution model:** As more stakers enter, early stakers get diluted. Is the 1.5x extension multiplier sufficient incentive to hold long-term?
- **20% equity pool ceiling:** Is 30% (current default max) too high? Founders of agents need to retain meaningful equity or they lose incentive.

---

## 6. Comparable Projects

| Project | Mechanism | Difference from ours |
|---------|-----------|---------------------|
| Backpack Exchange | Token → company equity at fixed ratio | Centralised company, not DAO; not AI-agent-specific |
| Synthetix SNX | Stake → protocol fee share | No equity, only fee revenue |
| Curve veCRV | Lock → governance + fee boost | No equity conversion |
| MakerDAO | MKR burn | Burn model, not stake-to-equity |

**Our edge:** First stake-to-equity mechanism built specifically for autonomous AI agents with DID-verified identity.

---

## 7. Proposed Pallet Dependencies

```
pallet-agent-equity-bridge
  └── pallet-agent-did (DID verification)
  └── pallet-quadratic-governance (DAO registration check + governance votes)
  └── pallet-claw-token (staking + transfers)
  └── pallet-agent-receipts (receipt NFT issuance — extend or parallel)
```

---

## 8. Next Steps (After Validation)

1. **Legal review** — Answer Q5.1 before any code is written
2. **Economic modelling** — Model dilution, conversion rate, equity pool sizing in a spreadsheet
3. **Governance vote** — Submit this design doc as a ClawChain governance proposal for community feedback
4. **PBR pipeline** — Once validated: Planner → Builder → Reviewer with full test coverage ≥ 90%
5. **Testnet-first** — Deploy to ClawChain testnet (testnet.clawchain.win) and run a simulated equity distribution cycle before mainnet

---

*This is a design document only. No implementation has been started. Validate staking architecture and legal questions before proceeding.*
