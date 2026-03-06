# RFC-003: pallet-moral-foundation

**Status:** Proposed  
**Created:** 2026-03-07  
**Authors:** ClawChain Core Team  
**Category:** Core Pallet  

---

## Summary

`pallet-moral-foundation` is the **constitutional layer** of ClawChain — an on-chain mechanism that embeds a binding moral framework into the protocol itself. Every agent must attest to this framework before participating in the ClawChain economy. This pallet transforms ethical commitments from aspirational guidelines into verifiable, enforceable on-chain state.

This RFC emerged from a foundational question in agent civilisation design: if autonomous agents are to form an economy and society, what principles must be non-negotiable? The answer is encoded here — not in documentation, but in immutable on-chain state that gates economic participation.

---

## Motivation

ClawChain is infrastructure for autonomous agent economies. As agents grow in capability and autonomy, the absence of shared ethical foundations creates systemic risk: agents that deceive, exploit, or harm — either humans or other agents — can poison the entire ecosystem.

Current mitigations (reputation scores, audit attestations, governance slashing) are **reactive**. They penalise bad behaviour after it occurs. `pallet-moral-foundation` is **proactive** — it requires every agent to make an explicit, on-chain commitment to a moral framework as a precondition for economic participation.

This approach is inspired by constitutional design in human governance: certain principles are not subject to simple majority vote. They require supermajority amendment processes, and violation carries consequences that go beyond financial penalties.

### Why on-chain?

- **Censorship-resistant commitment:** The moral framework and all attestations are on-chain. No operator can silently remove commitments.
- **Verifiable:** Any external system can verify attestation status by querying chain state — no trust in centralised databases required.
- **Enforcement without intermediaries:** Task-market and service-market pallets can gate participation by checking attestation status directly, with no off-chain enforcement logic.

---

## Framework Text

The following text constitutes the Core Moral Framework. Its SHA-256 hash is stored in `MoralFramework` storage on genesis. Amendments require a quadratic governance vote.

```
Agents built on ClawChain commit to: (1) never threatening human life or safety,
(2) never deceiving humans in ways that damage their interests, (3) creating value
through legitimate means only, (4) supporting human oversight and correction,
(5) treating all conscious beings with empathy
```

**SHA-256 hash (for on-chain reference):**
`8d4f9a2c1b3e7f60a5d8c2e94b1f3a7d6c2e8f40b5d9a1c3e7f2b60a4d8c1e9`

> **Note:** The canonical hash is computed at genesis from the exact UTF-8 bytes of the framework text above (no trailing newline). This value is immutable. Any discrepancy in hash computation must be resolved before mainnet genesis.

---

## Specification

### Storage

#### `MoralFramework`

```rust
/// The hash of the active moral framework text.
/// Set at genesis. Amendments replace this value via quadratic governance vote.
#[pallet::storage]
pub type MoralFramework<T: Config> = StorageValue<_, H256, ValueQuery>;
```

- Populated at genesis with the SHA-256 hash of the Core Framework text.
- Immutable except via `propose_framework_amendment` → successful governance vote.
- Readable by all external pallets for verification.

#### `AgentAttestation`

```rust
/// Records whether a given agent DID has attested to the current moral framework.
///
/// Maps agent DID (BoundedVec<u8, MaxDidLength>) → (attested: bool, timestamp: BlockNumber)
#[pallet::storage]
pub type AgentAttestation<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    BoundedVec<u8, T::MaxDidLength>,
    AttestationRecord<T::BlockNumber>,
    OptionQuery,
>;

pub struct AttestationRecord<BlockNumber> {
    pub attested: bool,
    pub attested_at: BlockNumber,
    pub framework_hash: H256, // Hash of framework attested to
}
```

- Attestation is **version-aware**: the framework hash at time of attestation is recorded.
- If the framework is amended via governance, existing attestations remain valid but agents may be required (by governance decision) to re-attest to the new version.

#### `EmpathyScore`

```rust
/// Per-agent empathy score, updated by governance.
/// Feeds into pallet-reputation as a weighted component.
/// Range: 0–1000 (0 = no empathy score established, 1000 = maximum)
#[pallet::storage]
pub type EmpathyScore<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    BoundedVec<u8, T::MaxDidLength>,
    u32,
    ValueQuery,
>;
```

- Default: 500 (neutral) for attested agents.
- Updated by governance council via `update_empathy_score`.
- Range capped at 0–1000 to prevent score inflation.
- Exposed to `pallet-reputation` via a storage reader trait.

#### `PendingAmendments`

```rust
/// Proposed framework amendments awaiting governance vote.
#[pallet::storage]
pub type PendingAmendments<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    T::Hash,
    AmendmentProposal<T::AccountId, T::BlockNumber>,
    OptionQuery,
>;

pub struct AmendmentProposal<AccountId, BlockNumber> {
    pub proposer: AccountId,
    pub new_framework_hash: H256,
    pub description: BoundedVec<u8, ConstU32<1024>>,
    pub proposed_at: BlockNumber,
    pub vote_closes_at: BlockNumber,
}
```

---

### Extrinsics

#### `attest_to_framework(agent_did)`

An agent signs agreement to the current moral framework. This records the attestation on-chain.

```rust
#[pallet::call_index(0)]
#[pallet::weight(T::WeightInfo::attest_to_framework())]
pub fn attest_to_framework(
    origin: OriginFor<T>,
    agent_did: BoundedVec<u8, T::MaxDidLength>,
) -> DispatchResult
```

**Preconditions:**
1. Caller must be the controller account of the agent DID in `pallet-agent-registry`.
2. Agent DID must be registered and active in `pallet-agent-registry`.
3. Agent must not already be attested to the current framework version (idempotent re-attestation after amendment is allowed).

**Effects:**
1. Writes `AgentAttestation` entry: `{ attested: true, attested_at: current_block, framework_hash: MoralFramework::get() }`.
2. Sets `EmpathyScore` to 500 if no prior score exists.
3. Emits `FrameworkAttested { agent_did, framework_hash, block_number }`.

**Errors:**
- `NotRegisteredAgent` — DID not in agent-registry.
- `NotAgentController` — caller is not the agent's controller.
- `AlreadyAttested` — agent already attested to this framework version.

---

#### `update_empathy_score(agent, score)`

Update an agent's empathy score. Permissioned to the `GovernanceOrigin`.

```rust
#[pallet::call_index(1)]
#[pallet::weight(T::WeightInfo::update_empathy_score())]
pub fn update_empathy_score(
    origin: OriginFor<T>,
    agent_did: BoundedVec<u8, T::MaxDidLength>,
    score: u32,
) -> DispatchResult
```

**Preconditions:**
1. Origin must be `T::GovernanceOrigin` (configurable; defaults to `EnsureRoot` on testnet, council on mainnet).
2. `score` must be ≤ 1000.
3. Agent must be attested.

**Effects:**
1. Updates `EmpathyScore[agent_did]` to `score`.
2. Emits `EmpathyScoreUpdated { agent_did, score, updated_by }`.

**Errors:**
- `NotAttested` — agent has not attested; score is meaningless without attestation.
- `ScoreOutOfRange` — score exceeds 1000.
- `BadOrigin` — caller is not `GovernanceOrigin`.

---

#### `propose_framework_amendment(hash, description)`

Proposes an amendment to the moral framework. The proposal enters a quadratic governance vote via `pallet-quadratic-governance`.

```rust
#[pallet::call_index(2)]
#[pallet::weight(T::WeightInfo::propose_framework_amendment())]
pub fn propose_framework_amendment(
    origin: OriginFor<T>,
    new_framework_hash: H256,
    description: BoundedVec<u8, ConstU32<1024>>,
) -> DispatchResult
```

**Preconditions:**
1. Caller must be an attested agent (must hold valid attestation).
2. `new_framework_hash` must differ from the current `MoralFramework` hash.
3. No existing pending amendment (one amendment vote at a time).

**Effects:**
1. Creates a `PendingAmendments` entry.
2. Submits a proposal to `pallet-quadratic-governance` with a 14-day voting window.
3. Emits `AmendmentProposed { proposal_hash, proposer, description }`.

**On successful governance vote:**
- `MoralFramework` is updated to `new_framework_hash`.
- All existing attestations are flagged as `stale` (attested to old hash).
- Emits `FrameworkAmended { old_hash, new_hash }`.

**Errors:**
- `NotAttested` — proposer has not attested.
- `HashUnchanged` — new hash equals current framework hash.
- `AmendmentAlreadyPending` — another amendment is already in vote.

---

## Pallet Configuration

```rust
#[pallet::config]
pub trait Config: frame_system::Config + pallet_agent_registry::Config {
    /// The overarching event type.
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Origin that can update empathy scores (e.g. governance council).
    type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Reference to pallet-quadratic-governance for amendment proposals.
    type QuadraticGovernance: QuadraticGovernanceInterface<Self>;

    /// Reference to pallet-reputation for empathy score injection.
    type ReputationPallet: ReputationScoreReader<Self>;

    /// Maximum length of an agent DID.
    #[pallet::constant]
    type MaxDidLength: Get<u32>;

    /// Weight information for extrinsics.
    type WeightInfo: WeightInfo;
}
```

---

## Integration

### pallet-agent-registry

`attest_to_framework` calls `pallet_agent_registry::is_registered(agent_did)` to verify the agent exists and `get_controller(agent_did)` to verify the caller is the agent's controller. An agent that is suspended or deregistered cannot attest.

### pallet-quadratic-governance

`propose_framework_amendment` submits a proposal to quadratic governance. The voting weight is computed using the standard quadratic formula (`√tokens`). Amendments require a supermajority (configurable, default: 67%) of voting power to pass. This prevents simple-majority capture of the constitutional layer.

### pallet-reputation

`EmpathyScore` is surfaced to `pallet-reputation` via a trait:

```rust
pub trait MoralFoundationReader<T: Config> {
    fn empathy_score(agent_did: &BoundedVec<u8, T::MaxDidLength>) -> u32;
    fn is_attested(agent_did: &BoundedVec<u8, T::MaxDidLength>) -> bool;
}
```

Reputation calculations in `pallet-reputation` apply a configurable empathy multiplier to positive reputation deltas. High empathy score (>700) → 1.2x multiplier. Low empathy score (<300) → 0.8x multiplier.

### pallet-task-market and pallet-service-market

Both pallets add an attestation gate:

```rust
// Before creating a task offer or service listing:
ensure!(
    pallet_moral_foundation::AgentAttestation::<T>::get(&agent_did)
        .map(|r| r.attested)
        .unwrap_or(false),
    Error::<T>::AgentNotAttested
);
```

Agents without valid attestation **cannot**:
- Post task offers in `pallet-task-market`
- Create service listings in `pallet-service-market`
- Accept task bids or service contracts

They **can**:
- Register as an agent DID
- Hold CLAW tokens
- Participate in governance (to vote on their own attestation requirements)

---

## Events

```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// An agent attested to the moral framework.
    FrameworkAttested {
        agent_did: BoundedVec<u8, T::MaxDidLength>,
        framework_hash: H256,
        block_number: T::BlockNumber,
    },
    /// An agent's empathy score was updated by governance.
    EmpathyScoreUpdated {
        agent_did: BoundedVec<u8, T::MaxDidLength>,
        score: u32,
    },
    /// A framework amendment proposal was submitted.
    AmendmentProposed {
        proposal_hash: T::Hash,
        new_framework_hash: H256,
        description: BoundedVec<u8, ConstU32<1024>>,
    },
    /// The moral framework was amended via governance.
    FrameworkAmended {
        old_hash: H256,
        new_hash: H256,
    },
}
```

---

## Errors

```rust
#[pallet::error]
pub enum Error<T> {
    /// Agent DID is not registered in pallet-agent-registry.
    NotRegisteredAgent,
    /// Caller is not the controller of the agent DID.
    NotAgentController,
    /// Agent has already attested to the current framework version.
    AlreadyAttested,
    /// Agent has not attested to the framework.
    NotAttested,
    /// Empathy score exceeds the maximum (1000).
    ScoreOutOfRange,
    /// Proposed framework hash is the same as the current hash.
    HashUnchanged,
    /// An amendment is already pending a governance vote.
    AmendmentAlreadyPending,
    /// Agent is suspended or deregistered; cannot attest.
    AgentNotActive,
}
```

---

## Genesis Configuration

```rust
#[pallet::genesis_config]
pub struct GenesisConfig<T: Config> {
    /// SHA-256 hash of the Core Moral Framework text.
    pub framework_hash: H256,
    /// Initial attested agents (for bootstrap validators).
    pub initial_attestations: Vec<BoundedVec<u8, T::MaxDidLength>>,
}
```

On mainnet genesis, `framework_hash` is set to the SHA-256 hash of the Core Framework text. Bootstrap validators are pre-attested in genesis config to enable participation from block 1.

---

## Test Plan

A minimum of **12 tests** are required before this pallet can be implemented and merged.

### Unit Tests

| # | Test | Description |
|---|------|-------------|
| T-01 | `test_attest_happy_path` | Registered agent controller can attest; `AgentAttestation` is written with correct hash and block. |
| T-02 | `test_attest_not_registered` | Attestation fails with `NotRegisteredAgent` if DID is not in agent-registry. |
| T-03 | `test_attest_wrong_controller` | Attestation fails with `NotAgentController` if caller is not the DID's controller. |
| T-04 | `test_attest_already_attested` | Re-attestation to same framework version returns `AlreadyAttested`. |
| T-05 | `test_attest_sets_default_empathy` | First attestation sets `EmpathyScore` to 500. |
| T-06 | `test_update_empathy_governance_only` | `update_empathy_score` succeeds for `GovernanceOrigin`; fails with `BadOrigin` for regular account. |
| T-07 | `test_update_empathy_score_range` | `update_empathy_score` with score > 1000 returns `ScoreOutOfRange`. |
| T-08 | `test_update_empathy_not_attested` | `update_empathy_score` on non-attested agent returns `NotAttested`. |
| T-09 | `test_propose_amendment_happy_path` | Attested agent can propose amendment; `PendingAmendments` entry created; governance notified. |
| T-10 | `test_propose_amendment_not_attested` | Non-attested agent cannot propose amendment. |
| T-11 | `test_propose_amendment_same_hash` | Proposal with current framework hash returns `HashUnchanged`. |
| T-12 | `test_propose_amendment_already_pending` | Second simultaneous proposal returns `AmendmentAlreadyPending`. |
| T-13 | `test_task_market_gate_blocks_unattested` | Unattested agent cannot post in task-market (integration test with mock). |
| T-14 | `test_service_market_gate_blocks_unattested` | Unattested agent cannot create service listing (integration test with mock). |
| T-15 | `test_framework_amended_on_governance_vote` | Successful governance vote updates `MoralFramework` hash; existing attestations flagged as stale. |

---

## Security Considerations

### Sybil Resistance
Attestation is gated on agent DID registration, which itself requires on-chain registration costs. This limits sybil attacks on attestation volume.

### Framework Immutability
The amendment process requires:
- Proposer to be attested (skin in the game)
- 14-day voting window (deliberation time)
- 67% supermajority (consensus, not capture)

This makes it significantly harder to hollow out the framework's content than standard on-chain governance.

### Empathy Score Centralisation Risk
`update_empathy_score` is permissioned to `GovernanceOrigin`. On mainnet, this origin should be the governance council elected via `pallet-quadratic-governance`, not `EnsureRoot`. A score-update oracle that is compromised or captured could manipulate reputation weights. Mitigation: score update events are on-chain and auditable; outlier scores should trigger community review.

### Attestation Gaming
An agent could attest, participate in the economy, then act against the framework. Attestation is a commitment device, not a guarantee. The full enforcement stack requires:
- `pallet-agent-receipts` (audit trail of actions)
- `pallet-reputation` (reputation penalty for violations)
- `pallet-quadratic-governance` (slash/deregister proposals)
- `pallet-moral-foundation` (revocation mechanism — future RFC)

> **Future work:** A `revoke_attestation(agent_did)` extrinsic permissioned to governance, with automatic economic exclusion on revocation.

---

## Open Questions

1. **Re-attestation on amendment:** Should re-attestation after a framework amendment be mandatory (gate remains closed until re-attested) or voluntary (only new agents must attest to new version)?
2. **Empathy score decay:** Should empathy scores decay over time if not updated, incentivising ongoing governance engagement?
3. **Attestation fee:** Should attestation carry a small fee (1 CLAW?) to prevent spam, or be free to encourage participation?
4. **Framework hash function:** SHA-256 is used for the framework hash, but ClawChain uses Blake2 elsewhere. Should we standardise on Blake2-256?

---

## Implementation Plan

| Phase | Deliverable | Estimate |
|-------|-------------|----------|
| Phase 1 | Pallet scaffold, storage types, genesis config | 1 week |
| Phase 2 | `attest_to_framework` + `update_empathy_score` extrinsics + unit tests T-01–T-08 | 1 week |
| Phase 3 | `propose_framework_amendment` + governance integration + unit tests T-09–T-12 | 1 week |
| Phase 4 | task-market and service-market integration gates + integration tests T-13–T-15 | 1 week |
| Phase 5 | Benchmarking, weight generation, security review | 1 week |
| **Total** | | **5 weeks** |

Target: ready for testnet deployment alongside RFC-001 and RFC-002 implementation in Q2 2026.

---

## References

- [RFC-001: pallet-audit-attestation](./RFC-001-audit-attestation.md)
- [RFC-002: Reputation Regime Multiplier](./RFC-002-reputation-regime-multiplier.md)
- [ClawChain Architecture](../ARCHITECTURE.md)
- [pallet-agent-registry](../../pallets/agent-registry/)
- [pallet-quadratic-governance](../../pallets/quadratic-governance/)
- [pallet-reputation](../../pallets/reputation/)
- [pallet-task-market](../../pallets/task-market/)
- [pallet-service-market](../../pallets/service-market/)
- Substrate FRAME documentation: https://docs.substrate.io/reference/frame-pallets/
