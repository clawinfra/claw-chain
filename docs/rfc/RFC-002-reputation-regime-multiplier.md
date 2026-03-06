# RFC-002: Reputation Regime Multiplier — Fear-Adaptive Agent Reputation Weights

- **Status:** Draft
- **Author:** ClawChain Core Team
- **Created:** 2026-03-06
- **Relates to:** `pallet-reputation`, `pallet-agent-registry`

---

## Summary

Extend `pallet-reputation` with a **Market Regime Multiplier** — a fear-adaptive weighting system that awards outsized reputation gains to agents that perform successfully during periods of market fear, and discounts reputation gains during periods of market complacency. The multiplier is derived from a Fear & Greed (F&G) index value published on-chain by a permissioned oracle, and applied automatically to every `update_reputation` call. Existing scores are unchanged; the multiplier only affects new delta applications.

---

## Motivation

The current reputation system awards equal weight to actions regardless of market conditions. An agent that executes perfectly during a calm bull market earns the same reputation delta as one that executes perfectly during extreme market fear — when agent failures are most catastrophic and most common.

This creates a flat trust landscape that fails to distinguish:
- **Battle-tested agents** — consistent performers across all conditions
- **Fair-weather agents** — reliable only when things are easy

The Regime Multiplier solves this by making reputation *context-sensitive*:

```
Performance during ExtremeFear (F&G < 15)  → 2.0x reputation gain
Performance during Fear        (F&G < 30)  → 1.5x reputation gain
Performance during Neutral     (F&G 30-70) → 1.0x reputation gain (baseline)
Performance during Greed       (F&G > 70)  → 0.8x reputation gain
Performance during ExtremeGreed (F&G > 85) → 0.6x reputation gain
```

**Why this matters:**

1. **Darwinian filter:** Agents that survive fear events emerge with exponentially higher trust scores, making them the preferred counterparties for high-value tasks.

2. **Oracle incentive:** The regime oracle earns reputation for accurate, timely F&G updates — creating a self-reinforcing accuracy incentive.

3. **Systemic resilience:** By rewarding fear-period performance, the protocol incentivises agents to remain operational during precisely the moments when the network most needs reliable participants.

4. **Defensible differentiation:** No other agent blockchain uses market regime as a reputation input. This is a ClawChain-native primitive that cannot be trivially replicated on generic chains.

### Concrete Examples

| Scenario | Base Delta | Regime | Multiplier | Final Delta |
|----------|------------|--------|-----------|-------------|
| Task completed, neutral market | +100 | Neutral | 1.0x | +100 |
| Task completed, fear market | +100 | Fear | 1.5x | +150 |
| Task completed, extreme fear | +100 | ExtremeFear | 2.0x | +200 |
| Task completed, greed | +100 | Greed | 0.8x | +80 |
| Dispute won, extreme greed | +200 | ExtremeGreed | 0.6x | +120 |
| Governance slash (negative) | −500 | Any | 1.0x | −500 |

*Note: The multiplier applies only to positive deltas. Slashes always use 1.0x to preserve punishment integrity.*

---

## Design

### Storage

```rust
/// The current market regime, set by oracle or Root.
#[pallet::storage]
#[pallet::getter(fn current_regime)]
pub type CurrentRegime<T: Config> = StorageValue<_, MarketRegime, ValueQuery>;

/// History of regime transitions (capped at MaxRegimeHistory).
#[pallet::storage]
pub type RegimeHistory<T: Config> = StorageValue<
    _,
    BoundedVec<RegimeTransition<T::BlockNumber>, T::MaxRegimeHistory>,
    ValueQuery,
>;
```

**`MarketRegime` enum:**

```rust
#[derive(
    Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug,
    TypeInfo, MaxEncodedLen, Default
)]
pub enum MarketRegime {
    ExtremeFear,  // F&G < 15    — 2.0x multiplier
    Fear,         // F&G < 30    — 1.5x multiplier
    #[default]
    Neutral,      // F&G 30–70   — 1.0x multiplier (default)
    Greed,        // F&G > 70    — 0.8x multiplier
    ExtremeGreed, // F&G > 85    — 0.6x multiplier
}
```

**`RegimeTransition` type:**

```rust
pub struct RegimeTransition<BlockNumber> {
    pub from: MarketRegime,
    pub to: MarketRegime,
    pub updated_at: BlockNumber,
}
```

### Extrinsics

#### `update_regime` (new)

```rust
#[pallet::weight(T::WeightInfo::update_regime())]
pub fn update_regime(
    origin: OriginFor<T>,
    regime: MarketRegime,
) -> DispatchResult
```

**Logic:**
1. Verify `origin` is either:
   - `T::RegimeOracleOrigin` (configured oracle account / collective), OR
   - Root (`ensure_root`)
2. Read `CurrentRegime`. If already equal to `regime`, return early (no-op, no event).
3. Append transition to `RegimeHistory` (bounded, drops oldest on overflow).
4. Store new `CurrentRegime`.
5. Emit `RegimeUpdated { old, new, at }`.

**Rate limiting:** `T::MinRegimeUpdateInterval` (configurable blocks) prevents oracle spam. Returns `Error::RegimeUpdateTooFrequent` if called within the interval.

#### `update_reputation` (modified)

The existing `update_reputation` extrinsic is extended to apply the multiplier:

```rust
// BEFORE (simplified existing logic)
pub fn update_reputation(
    origin: OriginFor<T>,
    account: T::AccountId,
    delta: i32,
) -> DispatchResult {
    T::ReputationOracle::ensure_oracle(origin)?;
    let new_score = compute_new_score(current_score, delta);
    Reputations::<T>::insert(&account, new_score);
    ...
}

// AFTER — multiplier applied to positive deltas only
pub fn update_reputation(
    origin: OriginFor<T>,
    account: T::AccountId,
    delta: i32,
) -> DispatchResult {
    T::ReputationOracle::ensure_oracle(origin)?;
    
    // Apply multiplier only to positive deltas
    let effective_delta = if delta > 0 {
        T::RegimeMultiplier::apply(delta, CurrentRegime::<T>::get())
    } else {
        delta // slashes are unaffected
    };
    
    let new_score = compute_new_score(current_score, effective_delta);
    Reputations::<T>::insert(&account, new_score);
    ...
}
```

### RPCs

No new RPCs are required. Existing `reputation_getScore` RPC is unchanged. The current regime can be queried via standard storage RPC:

```typescript
// Read current regime
const regime = await api.query.reputation.currentRegime();
// Returns: 'Neutral' | 'Fear' | 'ExtremeFear' | 'Greed' | 'ExtremeGreed'

// Compute expected multiplier client-side
const multiplier = getRegimeMultiplier(regime.toString());
```

A convenience RPC extension may be added in a follow-up:

```rust
fn get_regime_multiplier() -> RpcResult<u32>  // returns basis points: 200 = 2.0x
```

### Events

```rust
/// New event added to the existing Event enum:
RegimeUpdated {
    old: MarketRegime,
    new: MarketRegime,
    updated_at: T::BlockNumber,
},
```

Existing events (`ReputationUpdated`, `ReputationSlashed`, etc.) are unchanged but will include `effective_delta` in their payload to reflect the multiplier:

```rust
/// Existing event, extended field:
ReputationUpdated {
    account: T::AccountId,
    base_delta: i32,          // raw delta before multiplier
    effective_delta: i32,     // delta after regime multiplier
    new_score: u32,
    regime: MarketRegime,     // new field
},
```

### Config

The following items are added to `pallet-reputation`'s `Config` trait:

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    // ... existing config items ...

    /// Origin that is permitted to call update_regime.
    /// Typically a collective or a designated oracle AccountId.
    type RegimeOracleOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Pluggable multiplier implementation.
    /// Default implementation uses the curve defined in this RFC.
    type RegimeMultiplier: RegimeMultiplier;

    /// Minimum number of blocks between regime updates (spam prevention).
    #[pallet::constant]
    type MinRegimeUpdateInterval: Get<T::BlockNumber>;

    /// Maximum number of regime transitions to retain in history.
    #[pallet::constant]
    type MaxRegimeHistory: Get<u32>;
}
```

**`RegimeMultiplier` trait:**

```rust
pub trait RegimeMultiplier {
    /// Apply the regime multiplier to a positive reputation delta.
    /// Returns the scaled delta (may be truncated to u32 range).
    ///
    /// Default curve:
    ///   ExtremeFear  → delta * 200 / 100 (2.0x)
    ///   Fear         → delta * 150 / 100 (1.5x)
    ///   Neutral      → delta * 100 / 100 (1.0x)
    ///   Greed        → delta *  80 / 100 (0.8x)
    ///   ExtremeGreed → delta *  60 / 100 (0.6x)
    fn apply(delta: i32, regime: MarketRegime) -> i32;
}

/// Default implementation
pub struct DefaultRegimeMultiplier;
impl RegimeMultiplier for DefaultRegimeMultiplier {
    fn apply(delta: i32, regime: MarketRegime) -> i32 {
        let bps: u32 = match regime {
            MarketRegime::ExtremeFear  => 200,
            MarketRegime::Fear         => 150,
            MarketRegime::Neutral      => 100,
            MarketRegime::Greed        =>  80,
            MarketRegime::ExtremeGreed =>  60,
        };
        // Use saturating arithmetic; score ceiling enforced by compute_new_score
        delta.saturating_mul(bps as i32) / 100
    }
}
```

**Runtime configuration:**

```rust
impl pallet_reputation::Config for Runtime {
    // ... existing items ...
    type RegimeOracleOrigin = EnsureRoot<AccountId>;  // Start with Root; upgrade to oracle account post-launch
    type RegimeMultiplier = pallet_reputation::DefaultRegimeMultiplier;
    type MinRegimeUpdateInterval = ConstU32<100>;   // ~10 minutes at 6s blocks
    type MaxRegimeHistory = ConstU32<200>;
}
```

---

## Integration Points

| Component | Integration |
|-----------|-------------|
| `pallet-reputation` | Core pallet being extended — minimal surface change |
| `pallet-agent-registry` | `on_task_completed` calls `update_reputation`; picks up multiplier automatically |
| `pallet-task-market` | No changes needed — reputation updates already go through the pallet |
| Oracle service | Off-chain oracle subscribes to F&G index feed, calls `update_regime` on-chain |
| TypeScript SDK | `api.query.reputation.currentRegime()` + new `RegimeUpdated` event subscription |
| EvoClaw agents | Can read `CurrentRegime` before deciding whether to take a task |

---

## Security Considerations

1. **Oracle manipulation:** If the oracle account is compromised, an attacker can artificially inflate reputation by switching to `ExtremeFear` before submitting positive updates. Mitigation: `MinRegimeUpdateInterval` limits update frequency; governance can slash the oracle account; dual-oracle M-of-N can be added post-launch.

2. **Front-running:** An actor could observe a pending `update_regime → ExtremeFear` extrinsic and race to submit reputation-boosting actions before the block is finalised. Mitigation: regime transitions take effect in the *next* block after inclusion (add one block delay).

3. **Score inflation:** Sustained `ExtremeFear` periods could inflate top agents' scores beyond useful differentiation. Mitigation: `compute_new_score` already enforces a ceiling of 10,000 basis points; the multiplier cannot exceed that cap.

4. **Backward compatibility:** All existing reputation scores and extrinsics are unchanged. The multiplier only modifies the *delta* before it is applied — not stored scores. A chain migration is not required.

5. **Negative delta integrity:** Slashes (`delta < 0`) bypass the multiplier entirely. This ensures punishments are not softened by greed-regime discounts and are not amplified by fear-regime multipliers.

6. **Division precision:** The `bps / 100` integer arithmetic can lose up to 0.99 points per update. This is acceptable for the reputation range (0–10,000) and avoids floating-point complexity.

---

## Test Plan

Target coverage: **≥ 90%**

```
tests/
  regime_defaults_to_neutral            — initial storage state is Neutral
  update_regime_root                    — Root can update regime
  update_regime_oracle                  — configured oracle can update regime
  update_regime_unauthorized            — non-oracle non-root returns BadOrigin
  update_regime_too_frequent            — second call within interval returns RegimeUpdateTooFrequent
  update_regime_noop_same_value         — updating to same regime emits no event
  update_regime_history_bounded         — history ring-buffer respects MaxRegimeHistory
  multiplier_extreme_fear_2x            — delta * 2.0 applied in ExtremeFear
  multiplier_fear_1_5x                  — delta * 1.5 applied in Fear
  multiplier_neutral_1x                 — delta * 1.0 applied in Neutral (baseline)
  multiplier_greed_0_8x                 — delta * 0.8 applied in Greed
  multiplier_extreme_greed_0_6x         — delta * 0.6 applied in ExtremeGreed
  negative_delta_bypasses_multiplier    — slashes always use 1.0x
  score_capped_at_10000                 — multiplied delta cannot exceed score ceiling
  score_floor_at_0                      — multiplied delta cannot go below 0
  regime_updated_event_emitted          — RegimeUpdated event contains old, new, at
  reputation_updated_event_extended     — ReputationUpdated event contains regime field
  existing_scores_unchanged             — no existing score is modified by feature addition
  benchmark_update_regime               — weight benchmark for update_regime
  benchmark_update_reputation_with_mult — weight benchmark with multiplier path
```

---

## Migration

No storage migration required. The new storage items (`CurrentRegime`, `RegimeHistory`) initialise with sensible defaults:

- `CurrentRegime` → `MarketRegime::Neutral` (via `Default` derive)
- `RegimeHistory` → empty `BoundedVec`

The `update_reputation` extrinsic change is logic-only and does not alter storage layout. Existing reputation scores remain intact.

**Spec version bump:** The runtime `spec_version` must be incremented when this pallet change is deployed, as the `Event` enum and `Config` trait have new items.

---

## Open Questions

1. **Oracle architecture:** Should the regime oracle be a single trusted account (simpler) or a multi-sig collective (more decentralised)? The Config trait supports both via `RegimeOracleOrigin`. Recommendation: launch with Root, migrate to an oracle collective post-mainnet.

2. **F&G data source:** Which Fear & Greed index should the oracle use — Alternative.me, CNN, or a ClawChain-native on-chain calculation? The pallet is oracle-agnostic; the data source is an operational decision.

3. **Negative delta multiplier:** Should slashes also be regime-weighted? An argument exists that losses during extreme fear should hurt more (agent failure when it counts most). Current design keeps slashes at 1.0x for simplicity and to avoid perverse incentives.

4. **Regime history access:** Should `RegimeHistory` be queryable via RPC for analytics? This would allow dApps to chart how regime changes correlate with reputation score distributions.

5. **Cross-pallet regime access:** Should `MarketRegime` be exposed via a shared trait so other pallets (e.g., `pallet-task-market` for bid weighting, `pallet-gas-quota` for fee discounts during fear) can consume it? Factoring into a `pallet-market-oracle` might be cleaner than each pallet importing `pallet-reputation`.
