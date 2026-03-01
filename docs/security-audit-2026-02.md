# ClawChain Security Audit — February 2026 (Pre-Mainnet)

**Date:** 2026-02-27  
**Auditor:** Alex Chen (B1-Audit sub-agent), ClawInfra  
**Scope:** All pallets present on `main` branch + dependency review  
**Commit:** `2f4dad0` (HEAD at time of audit)  
**Tool:** Manual static analysis + Cargo.lock CVE review (cargo-audit not installed)

---

## Executive Summary

Nine of twelve planned pallets were audited on `main`; three (`ibc-lite`, `anon-messaging`, `service-market`) exist only on feature branches and **were not merged to `main` at audit time** — they require a separate audit pass before mainnet inclusion.

**Overall Risk: HIGH** — Two high-severity bugs were found that must be fixed before mainnet launch:

1. **`pallet-agent-registry`** — any signed account can arbitrarily modify any agent's reputation score (missing authorization check).
2. **`pallet-claw-token`** — `treasury_spend` emits a success event without performing any actual token transfer (logic stub left in production code).
3. **`pallet-agent-receipts`** — unbounded loop on caller-controlled `before_nonce: u64` enables block-exhaust DoS.

No vulnerabilities in cryptographic primitives were identified. The codebase demonstrates good use of `BoundedVec`, `saturating_*` arithmetic in most places, and `checked_add` for critical counters.

---

## Summary Table

| Pallet | CRITICAL | HIGH | MEDIUM | LOW | Status |
|--------|----------|------|--------|-----|--------|
| `agent-did` | 0 | 0 | 1 | 1 | ✅ Audited |
| `agent-receipts` | 0 | 1 | 1 | 1 | ⚠️ Fix required |
| `agent-registry` | 0 | 1 | 1 | 0 | ⚠️ Fix required |
| `claw-token` | 0 | 1 | 0 | 1 | ⚠️ Fix required |
| `gas-quota` | 0 | 0 | 2 | 1 | ✅ Audited |
| `quadratic-governance` | 0 | 0 | 0 | 1 | ✅ Audited |
| `reputation` | 0 | 0 | 1 | 0 | ✅ Audited |
| `rpc-registry` | 0 | 0 | 1 | 1 | ✅ Audited |
| `task-market` | 0 | 0 | 1 | 1 | ✅ Audited |
| `ibc-lite` | — | — | — | — | ❌ Not on main |
| `anon-messaging` | — | — | — | — | ❌ Not on main |
| `service-market` | — | — | — | — | ❌ Not on main |
| **Dependencies** | 0 | 0 | 1 | 0 | ✅ Reviewed |
| **TOTAL** | **0** | **3** | **8** | **6** | |

---

## Detailed Findings

### HIGH-1: Unrestricted Reputation Mutation — `pallet-agent-registry`

**File:** `pallets/agent-registry/src/lib.rs:294–328`  
**Severity:** HIGH  
**Category:** Improper Access Control

**Description:**  
`update_reputation` uses only `ensure_signed(origin)?` with no check that the caller is the agent owner, a designated reputation oracle, or a privileged pallet. Any signed account can call this extrinsic with any `agent_id` and any `delta: i32` to increase or decrease that agent's reputation score without restriction.

```rust
pub fn update_reputation(
    origin: OriginFor<T>,
    agent_id: AgentId,
    delta: i32,
) -> DispatchResult {
    ensure_signed(origin)?;   // ← only check: is the caller signed?
    // No: is caller the owner? is caller authorized? is caller a trusted pallet?
    ...
    agent.reputation = new_score;
```

**Impact:** Any account can manipulate any agent's reputation, defeating ClawChain's reputation-gated access controls across task-market, service-market, rpc-registry, and gas-quota.

**Recommendation:**  
Restrict to one of: (a) pallet-internal calls only (mark as `pub(crate)` or remove from `#[pallet::call]`); (b) ensure caller is a designated oracle stored in config; (c) use `T::ReputationOrigin::ensure_origin(origin)`. The `pallet-reputation` already provides `submit_review` with rating logic — it should be the sole mutator via a cross-pallet trait.

---

### HIGH-2: Treasury Spend Emits Event Without Transferring Funds — `pallet-claw-token`

**File:** `pallets/claw-token/src/lib.rs:224–238`  
**Severity:** HIGH  
**Category:** Logic Error / Missing Implementation

**Description:**  
`treasury_spend` is gated by `ensure_root` and emits `Event::TreasurySpend { to, amount }`, but contains **no actual currency transfer**. Any root call will log a "successful" spend without moving tokens.

```rust
pub fn treasury_spend(origin: OriginFor<T>, to: T::AccountId, amount: u128) -> DispatchResult {
    ensure_root(origin)?;
    Self::deposit_event(Event::TreasurySpend { to, amount });  // ← emits event
    Ok(())                                                      // ← no transfer!
}
```

**Impact:** Treasury spends appear successful on-chain (event emitted, extrinsic succeeds) while no tokens move. Downstream consumers parsing events to confirm disbursements receive false data. Could cause accounting discrepancies post-genesis.

**Recommendation:**  
Implement the actual transfer using `T::Currency::transfer` from the treasury account, or remove this extrinsic until the treasury pallet integration is ready. Do not ship stub functions that emit success events in production.

---

### HIGH-3: Unbounded Loop via Caller-Controlled u64 — `pallet-agent-receipts`

**File:** `pallets/agent-receipts/src/lib.rs:222–252`  
**Severity:** HIGH  
**Category:** DoS / Unbounded Storage Iteration

**Description:**  
`clear_old_receipts` iterates `for nonce in 0..before_nonce` where `before_nonce: u64` is caller-supplied. A malicious caller can pass `u64::MAX` (≈1.8×10¹⁹) causing the block to iterate over the entire theoretical nonce space.

```rust
pub fn clear_old_receipts(origin: OriginFor<T>, agent_id: Vec<u8>, before_nonce: u64) -> DispatchResult {
    ensure_signed(origin)?;
    for nonce in 0..before_nonce {       // ← up to u64::MAX iterations
        if Receipts::<T>::contains_key(&bounded_agent_id, nonce) {
            Receipts::<T>::remove(&bounded_agent_id, nonce);
```

While each iteration does a storage lookup (expensive), the weight annotation likely does not account for this. Even with storage caching, `u64::MAX` iterations will exhaust block time.

**Impact:** Block DoS. Any user paying the fixed extrinsic fee can stall block production.

**Recommendation:**  
Add a `MaxClearBatchSize` constant and cap: `ensure!(before_nonce <= T::MaxClearBatchSize::get(), Error::<T>::BatchTooLarge)`. Update weight to be linear in `before_nonce.min(MaxClearBatchSize)`.

---

### MEDIUM-1: Vec<u8> Extrinsic Parameters (Multiple Pallets)

**Files:**
- `pallets/agent-did/src/lib.rs:260` — `context: Vec<u8>`
- `pallets/agent-registry/src/lib.rs:205` — `did: Vec<u8>`, `metadata: Vec<u8>`
- `pallets/reputation/src/lib.rs:282` — `comment: Vec<u8>`
- `pallets/rpc-registry/src/lib.rs:247` — `url: Vec<u8>`, `region: Vec<u8>`
- `pallets/task-market/src/lib.rs:303` — `title: Vec<u8>`, `description: Vec<u8>`

**Severity:** MEDIUM  
**Category:** Unbounded Extrinsic Input

**Description:**  
Several extrinsic parameters accept `Vec<u8>` directly. Although storage converts these to `BoundedVec` (returning an error if too large), the raw bytes are processed by the runtime before that check. Large inputs waste validator decode time and can bloat blocks.

**Recommendation:**  
Use `BoundedVec<u8, T::MaxXxxLength>` directly as extrinsic parameter types. This is now idiomatic in FRAME and rejects oversized inputs at the decode stage before runtime execution.

---

### MEDIUM-2: Hardcoded Non-Benchmarked Weights (All Pallets)

**Severity:** MEDIUM  
**Category:** Incorrect Weight Accounting

**Description:**  
Most pallets use hardcoded weight estimates: `Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(N, M)`. These are not derived from FRAME benchmarks and may significantly underestimate actual execution cost.

For example, `task-market::post_task` involves multiple storage writes, a currency reserve, and bounded vec insertions, yet is weighted as `reads_writes(2, 3)`.

**Recommendation:**  
Add a `benchmarks` module to each pallet and run `cargo benchmark` before mainnet. Use `T::WeightInfo::xxx()` from benchmark output. The `task-market` and `rpc-registry` pallets are highest priority due to fee-sensitive operations.

---

### MEDIUM-3: gas-quota Returns u32::MAX for Very Large Stake

**File:** `pallets/gas-quota/src/lib.rs:248`  
**Severity:** MEDIUM  
**Category:** Logic Error / Unintended Unlimited Access

**Description:**  
```rust
let stake_quota = (stake / stake_per_tx).try_into().unwrap_or(u32::MAX);
```
When stake is so large that `stake / stake_per_tx` overflows u32, the fallback is `u32::MAX` (~4B transactions/day). This is functionally equivalent to the `UnlimitedStakeThreshold` bypass but triggered by arithmetic overflow rather than explicit config. An attacker with very large stake (but below `UnlimitedStakeThreshold`) could exploit this.

**Recommendation:**  
Use `saturating_into()` or cap at a `MaxQuota` constant instead of `u32::MAX`.

---

### MEDIUM-4: agent-registry Anyone Can Call update_reputation via Pallet-to-Pallet (Design Gap)

Already covered in HIGH-1. Noted separately: the reputation cross-pallet trait (`ReputationManager`) defined in `pallet-reputation` is the correct abstraction. `update_reputation` in `agent-registry` should either be removed or restricted to internal pallet calls only.

---

### MEDIUM-5: rpc-registry O(n) Linear Scan on ActiveNodes

**File:** `pallets/rpc-registry/src/lib.rs:414`, `460`  
**Severity:** MEDIUM  
**Category:** Performance / Weight Miscalculation

```rust
if let Some(pos) = active.iter().position(|id| *id == node_id) {
    active.remove(pos);
}
```

`ActiveNodes` is a `BoundedVec`, so this is bounded — but the weight annotation doesn't reflect O(n) cost. For large active node sets, this becomes expensive.

**Recommendation:**  
Use a `StorageMap<RpcNodeId, ()>` for O(1) lookup/removal, or ensure weight annotation includes `MaxActiveNodes` factor.

---

### MEDIUM-6: quadratic-governance Proposal Deposit Not Validated Against Min

**File:** `pallets/quadratic-governance/src/lib.rs:252`  
**Severity:** LOW-MEDIUM  
**Category:** Insufficient Input Validation

The proposal deposit amount comes from `T::MinProposalDeposit` config. The code appears correct, but there's no check that the reserved amount matches exactly what was configured (the config value could be zero if misconfigured at genesis). Add an `ensure!(T::MinProposalDeposit::get() > Zero::zero(), ...)` in `on_genesis_config` or integrity tests.

---

### MEDIUM-7: task-market Vec<u8> for title/description/proposal

**File:** `pallets/task-market/src/lib.rs:303-304`, `371`, `468`  
**Severity:** MEDIUM  
**Category:** Unbounded Extrinsic Input (see MEDIUM-1)

Same pattern as MEDIUM-1. `title: Vec<u8>` and `description: Vec<u8>` can be arbitrarily large. Tasks are currency-staked which adds a natural economic deterrent, but doesn't prevent block-size abuse.

---

### MEDIUM-8: agent-receipts clear_old_receipts Weight Not Proportional

**File:** `pallets/agent-receipts/src/lib.rs:256`  
**Severity:** MEDIUM  
**Category:** Weight Miscalculation (related to HIGH-3)

Even after adding a `MaxClearBatchSize` cap (HIGH-3 fix), the current weight annotation (`fn clear_old_receipts() -> Weight`) is a constant — it must be changed to linear in the actual cleared count.

---

### LOW-1: gas-quota error message reuse

**File:** `pallets/gas-quota/src/lib.rs:220`  
`ensure!(tier <= 2, Error::<T>::QuotaNotInitialized)` — reusing unrelated error code for tier validation. Add `Error::<T>::InvalidReputationTier`.

---

### LOW-2: claw-token claim_airdrop timing — front-running possible

**File:** `pallets/claw-token/src/lib.rs:174`  
Airdrop uses ContributorScores set by root. No merkle-proof based claim, so depends entirely on root correctness. Low risk given ensure_root on writes, but document this trust assumption.

---

### LOW-3: quadratic-governance integer_sqrt — no fuzz tests

**File:** `pallets/quadratic-governance/src/lib.rs:457`  
The Newton/Babylonian sqrt is correct for standard inputs, but has no property-based fuzz tests for edge cases near u128::MAX. Add `proptest` coverage.

---

### LOW-4: task-market resolve_dispute loser calculation

**File:** `pallets/task-market/src/lib.rs:640`  
```rust
let loser = if winner == poster { worker.clone() } else { poster.clone() };
```
If `winner` is neither `poster` nor `worker` (e.g., a typo in the root call), the loser defaults to `poster` silently. Add `ensure!(winner == poster || winner == worker, Error::<T>::InvalidWinner)`.

---

### LOW-5: rpc-registry heartbeat stale node detection

**File:** `pallets/rpc-registry/src/lib.rs:433`  
`report_inactive` allows any signed account to remove a node. While there's a heartbeat recency check, the reporter is not required to stake anything, enabling griefing (repeated failed reports). Consider adding a reporter deposit or reputation gate.

---

### LOW-6: agent-did no storage deposit for service endpoints

**File:** `pallets/agent-did/src/lib.rs:326`  
Adding service endpoints to a DID document is free (no deposit). An agent with a valid DID could spam up to `MaxServiceEndpoints` entries at zero cost. Consider requiring a small deposit per endpoint, refunded on removal.

---

## Dependency Review

`cargo-audit` was not installed in the CI environment; findings are based on Cargo.lock version scan.

| Crate | Version | CVE/RUSTSEC | Status |
|-------|---------|-------------|--------|
| `h2` | 0.3.27 | RUSTSEC-2024-0003 (RST flood DoS) | ✅ Fixed (≥0.3.26) |
| `h2` | 0.4.13 | None known | ✅ OK |
| `rustls` | 0.23.36 | None known | ✅ OK |
| `ring` | 0.16.20 | RUSTSEC-2025-0009 check recommended | ⚠️ Verify |
| `curve25519-dalek` | 0.9.2 | RUSTSEC-2024-0344 (timing side-channel, fixed in 4.x) | ⚠️ Version very old — likely a transitive dep of older substrate; verify upstream |
| `tokio` | 1.49.0 | None known | ✅ OK |
| `openssl-probe` | 0.3.1 | None known | ✅ OK |

**Recommendation:** Install `cargo-audit` in CI (`cargo install cargo-audit`) and run on every PR. Pin `cargo-audit` in CI toolchain. The `curve25519-dalek 0.9.2` dependency should be investigated — if it's a transitive Substrate dependency, check if Substrate has patched it via `[patch.crates-io]`.

---

## Missing Pallets (Not Audited)

The following pallets are referenced in the mainnet roadmap but were **not present on `main`** at audit time:

| Pallet | Branch | Status |
|--------|--------|--------|
| `ibc-lite` | `feat/ibc-lite` | Needs merge review + audit |
| `anon-messaging` | `feat/anon-messaging-minimal` | Needs merge review + audit |
| `service-market` | Not found | Not committed to any remote branch |

**These pallets must not be included in mainnet without a dedicated audit pass.** Based on code review of the feature branches (from prior local state), preliminary concerns include:
- `ibc-lite`: Unchecked counter arithmetic (`channel_number + 1`, `sequence + 1`) — should use `checked_add`
- `anon-messaging`: `EphemeralQueue` is bounded per block (`MaxEphemeralPerBlock`), but `on_initialize` processes all of them — ensure weight accounts for max batch size
- `service-market`: Multiple counters use unchecked `+1` arithmetic; `expire_overdue_invocations` uses `InvocationsByDeadline::iter()` which could be large before `MaxExpirationsPerBlock` cap; escrow transfers use `.ok()` (silent failure) — consider emitting error events

---

## Recommendations Priority

| Priority | Action |
|----------|--------|
| 🔴 P0 — Block mainnet | Fix HIGH-1 (`agent-registry` open reputation mutation) |
| 🔴 P0 — Block mainnet | Fix HIGH-2 (`claw-token` treasury_spend no-op) |
| 🔴 P0 — Block mainnet | Fix HIGH-3 (`agent-receipts` unbounded loop) |
| 🔴 P0 — Block mainnet | Complete audit of ibc-lite, anon-messaging, service-market |
| 🟠 P1 — Before mainnet | Replace Vec<u8> extrinsic params with BoundedVec across all pallets |
| 🟠 P1 — Before mainnet | Install cargo-audit in CI; verify curve25519-dalek transitive dep |
| 🟠 P1 — Before mainnet | Run FRAME benchmarks for all pallets; replace hardcoded weights |
| 🟡 P2 — Soon after launch | Address MEDIUM-3 (gas-quota u32::MAX fallback) |
| 🟡 P2 — Soon after launch | Fix LOW-1 through LOW-6 |
| 🟢 P3 — Ongoing | Add proptest/fuzz coverage for integer_sqrt and crypto paths |

---

*Report generated by automated static analysis + manual review. This is not a formal third-party audit. A professional audit by an independent Substrate security firm (e.g., Trail of Bits, Least Authority, Oak Security) is strongly recommended before mainnet token launch.*
