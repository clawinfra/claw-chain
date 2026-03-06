# RFC-001: `pallet-audit-attestation` — On-Chain Verifiable Audit Trail

- **Status:** Draft
- **Author:** ClawChain Core Team
- **Created:** 2026-03-06
- **Relates to:** `pallet-agent-registry`, `docs/security-audit-2026-02.md`

---

## Summary

Introduce `pallet-audit-attestation` — a new runtime pallet that enables registered auditors to publish cryptographically-signed audit attestations on-chain. Any agent, pallet, or off-chain client can query `is_audited(target, max_age_blocks)` to verify that a target has a current, valid attestation before interacting with it. The first real attestation will reference `docs/security-audit-2026-02.md` (the February 2026 security sprint), turning an internal security exercise into a permanent on-chain product moat.

---

## Motivation

ClawChain's February 2026 security audit resolved 11 findings across the runtime. That audit exists only as a markdown file in the repo — it is invisible to on-chain agents and cannot be queried programmatically. As ClawChain scales to a multi-agent economy with hundreds of registered pallets and thousands of agents, any participant needs a trust signal before routing value through an unknown counterparty.

`pallet-audit-attestation` solves three problems simultaneously:

1. **Trust bootstrap:** Agents and contracts can refuse to interact with unaudited targets, creating a market incentive for audits.
2. **Audit freshness enforcement:** `max_age_blocks` ensures stale attestations are treated as absent — a pallet audited 18 months ago is not as safe as one audited last month.
3. **Product moat:** By publishing the 2026-02 audit on-chain as Attestation #0, ClawChain demonstrates the complete audit→attestation pipeline and differentiates itself from competing agent chains.

### Concrete Use Cases

- `pallet-task-market` checks `is_audited(worker_agent_hash, 100_000)` before releasing escrow payments above 1,000 CLAW.
- EvoClaw edge agents refuse to call any on-chain pallet that does not have an active attestation.
- Third-party bridges and dApps display the attestation badge alongside agent listings.
- Insurance protocols price premiums based on audit severity counts.

---

## Design

### Storage

```rust
/// All attestations, keyed by target hash.
/// One attestation per target at a time (latest wins).
#[pallet::storage]
pub type Attestations<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    T::Hash,          // target_hash — blake2b hash of agent/pallet code/DID
    AttestationRecord<T>,
    OptionQuery,
>;

/// Tracks which targets a given auditor has attested.
/// Enables efficient revocation by auditor.
#[pallet::storage]
pub type AuditorAttestations<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    T::AccountId,     // auditor account
    BoundedVec<T::Hash, T::MaxAttestationsPerAuditor>,
    ValueQuery,
>;
```

**`AttestationRecord` type:**

```rust
pub struct AttestationRecord<T: Config> {
    /// DID of the auditor (must be registered in pallet-agent-registry)
    pub auditor_did: BoundedVec<u8, T::MaxDidLen>,
    /// AccountId of the auditor (for permissioning)
    pub auditor_account: T::AccountId,
    /// Blake2b hash of the audited target (agent/pallet binary or DID document)
    pub target_hash: T::Hash,
    /// Blake2b hash of the findings summary document (stored off-chain / IPFS)
    pub findings_summary_hash: T::Hash,
    /// Severity counts at time of audit
    pub severity_counts: SeverityCounts,
    /// Block at which this attestation was submitted
    pub timestamp: T::BlockNumber,
    /// Ed25519/Sr25519 signature by auditor over (target_hash || findings_summary_hash || severity_counts || timestamp)
    pub auditor_signature: BoundedVec<u8, ConstU32<64>>,
}

/// Findings severity breakdown
pub struct SeverityCounts {
    pub critical: u8,
    pub high: u8,
    pub medium: u8,
    pub low: u8,
}
```

### Extrinsics

#### `submit_attestation`

```rust
#[pallet::weight(T::WeightInfo::submit_attestation())]
pub fn submit_attestation(
    origin: OriginFor<T>,
    target: T::Hash,
    summary_hash: T::Hash,
    severities: SeverityCounts,
    sig: BoundedVec<u8, ConstU32<64>>,
) -> DispatchResult
```

**Logic:**
1. Verify `origin` is a signed account.
2. Look up the caller in `pallet-agent-registry` — they must be a registered agent with `status == Active`. Returns `Error::AuditorNotRegistered` if not.
3. Verify that no existing attestation with an identical `(target, auditor_account)` tuple exists with the same block (prevents replay). If a prior attestation exists for this target+auditor, it is overwritten (re-audit).
4. Verify the `sig` over `(target || summary_hash || encode(severities) || current_block_number)`. Returns `Error::InvalidSignature` if invalid.
5. Construct `AttestationRecord` and insert into `Attestations` and `AuditorAttestations`.
6. Emit `AttestationSubmitted`.

**Permissioning:** Any registered agent DID can submit — no separate auditor whitelist. The quality signal comes from the auditor's own reputation score (queryable via `pallet-reputation`). Future governance may add an `AuditorTier` whitelist; this is flagged in Open Questions.

#### `revoke_attestation`

```rust
#[pallet::weight(T::WeightInfo::revoke_attestation())]
pub fn revoke_attestation(
    origin: OriginFor<T>,
    target: T::Hash,
) -> DispatchResult
```

**Logic:**
1. Verify `origin` is signed.
2. Look up `Attestations[target]`. Returns `Error::AttestationNotFound` if absent.
3. Verify caller is the `auditor_account` on the record. Returns `Error::NotAuditor` otherwise. (Root can also revoke via governance.)
4. Remove from `Attestations` and update `AuditorAttestations`.
5. Emit `AttestationRevoked`.

### RPCs

A custom JSON-RPC method is exposed via the node's RPC server:

```rust
fn is_audited(
    target: Hash,
    max_age_blocks: u32,
    at: Option<BlockHash>,
) -> RpcResult<bool>
```

**Logic:**
- Fetch `Attestations[target]`.
- If `None`, return `false`.
- Compute `current_block - attestation.timestamp`. If `> max_age_blocks`, return `false`.
- Return `true`.

**TypeScript SDK wrapper:**

```typescript
await api.rpc.attestation.isAudited(targetHash, maxAgeBlocks): Promise<bool>
```

### Events

```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// A new attestation was submitted.
    AttestationSubmitted {
        auditor: T::AccountId,
        target: T::Hash,
        severity_counts: SeverityCounts,
        timestamp: T::BlockNumber,
    },
    /// An attestation was revoked.
    AttestationRevoked {
        auditor: T::AccountId,
        target: T::Hash,
        revoked_at: T::BlockNumber,
    },
}
```

### Errors

```rust
#[pallet::error]
pub enum Error<T> {
    /// Caller is not a registered agent DID in pallet-agent-registry.
    AuditorNotRegistered,
    /// Signature verification failed.
    InvalidSignature,
    /// No attestation found for this target.
    AttestationNotFound,
    /// Caller is not the auditor of this attestation.
    NotAuditor,
    /// Auditor has reached MaxAttestationsPerAuditor.
    TooManyAttestations,
}
```

### Config

```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

    /// Maximum number of active attestations a single auditor can hold.
    #[pallet::constant]
    type MaxAttestationsPerAuditor: Get<u32>;

    /// Maximum byte length of an auditor DID string.
    #[pallet::constant]
    type MaxDidLen: Get<u32>;

    /// Weight information for extrinsics.
    type WeightInfo: WeightInfo;

    /// Interface to pallet-agent-registry for auditor DID verification.
    type AgentRegistry: AgentRegistryInterface<Self::AccountId>;
}
```

**Runtime defaults:**

```rust
impl pallet_audit_attestation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxAttestationsPerAuditor = ConstU32<500>;
    type MaxDidLen = ConstU32<128>;
    type WeightInfo = pallet_audit_attestation::weights::SubstrateWeight<Runtime>;
    type AgentRegistry = AgentRegistry;
}
```

---

## Integration Points

| Pallet / Component | Integration |
|-------------------|-------------|
| `pallet-agent-registry` | Auditor eligibility check: `T::AgentRegistry::is_registered_agent(auditor)` |
| `pallet-reputation` | Optional: attestation submission increments auditor reputation by `+50` |
| `pallet-task-market` | Task assignment can gate on `is_audited(worker_hash, config.max_age)` |
| TypeScript SDK | `api.rpc.attestation.isAudited(hash, maxAge)` |
| EvoClaw edge agents | Refuse to call non-audited pallets (configurable policy) |
| `docs/security-audit-2026-02.md` | First attestation submitted at genesis / early mainnet |

---

## Security Considerations

1. **Signature replay prevention:** The signed payload includes `current_block_number`, bounding reuse to the exact block. Subsequent blocks invalidate the signature.
2. **Auditor impersonation:** Only accounts with an active DID in `pallet-agent-registry` may submit. This ties audit authority to on-chain identity.
3. **Malicious revocation:** Revoke can only be called by the original `auditor_account`. Root/sudo override is available for emergency.
4. **False attestations:** A registered auditor could submit a false "all clear" attestation. Mitigation: auditor reputation is visible on-chain and slashable via governance. Future `AuditorTier` whitelist (see Open Questions) can restrict who may attest for high-value targets.
5. **Hash collision:** `target_hash` uses Blake2b (collision-resistant). Pallet code and DID documents should be hashed canonically (see SDK tooling).
6. **Storage bloat:** `MaxAttestationsPerAuditor` caps per-auditor storage. Revoked attestations are removed, not tombstoned.

---

## Test Plan

Target coverage: **≥ 90%**

```
tests/
  submit_attestation_happy_path         — valid auditor, valid sig, stored correctly
  submit_attestation_overwrites_old     — second attestation replaces first for same target+auditor
  submit_attestation_unregistered       — returns AuditorNotRegistered
  submit_attestation_invalid_sig        — returns InvalidSignature
  submit_attestation_too_many           — MaxAttestationsPerAuditor enforced
  revoke_attestation_happy_path         — auditor can revoke own attestation
  revoke_attestation_not_auditor        — non-auditor cannot revoke
  revoke_attestation_not_found          — returns AttestationNotFound
  revoke_attestation_root_override      — Root can revoke any attestation
  is_audited_rpc_present_fresh          — returns true for fresh attestation
  is_audited_rpc_present_stale          — returns false when age > max_age_blocks
  is_audited_rpc_absent                 — returns false for unknown target
  event_attestation_submitted           — event emitted with correct fields
  event_attestation_revoked             — event emitted with correct fields
  benchmark_submit_attestation          — weight benchmark
  benchmark_revoke_attestation          — weight benchmark
```

---

## Migration

No storage migration required for the initial deployment — this is a new pallet with empty storage.

**Genesis bootstrapping:** The `security-audit-2026-02.md` attestation will be submitted as a signed extrinsic in the first few blocks after the pallet is deployed to testnet, establishing attestation ID #0.

If `pallet-reputation` integration (auditor gets +50 per attestation) is enabled post-launch, a migration to backfill reputation for the genesis attestation may be needed.

---

## Open Questions

1. **AuditorTier whitelist:** Should we add a governance-managed list of "certified auditors" whose attestations carry higher weight? This would enable tiered trust (any registered agent = tier-1; governance-certified auditor = tier-2). Deferred to post-RFC feedback.

2. **Multi-auditor consensus:** For high-value pallets, should `is_audited` require M-of-N auditor agreement? Current design is single-auditor. Could be extended via a `required_auditors: u8` config per target.

3. **Off-chain summary storage:** `findings_summary_hash` points to a document that must be retrievable. Should ClawChain mandate IPFS pinning? Or is a content-addressed URI in the metadata sufficient?

4. **Expiry vs. freshness:** Currently `is_audited` accepts `max_age_blocks` per-query. Should we also support a per-attestation `expires_at` field set by the auditor at submission time?

5. **Reputation integration scope:** Should attestation submission automatically call `pallet-reputation`? Or keep pallets loosely coupled and let off-chain indexers update reputation scores?
