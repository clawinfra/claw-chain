# Conventions — ClawChain

Consistent naming lets agents and humans navigate the codebase without reading every file.
These rules are enforced by `scripts/agent-lint.sh`.

---

## Pallet Naming

**Format:** `pallet-{domain}` in kebab-case (directory and Cargo package name).

```
✅  pallet-agent-registry
✅  pallet-task-market
✅  pallet-claw-token
✅  pallet-ibc-lite

❌  AgentRegistry       (PascalCase)
❌  pallet_reputation   (snake_case)
❌  agentregistry       (no separator)
```

The Rust crate name uses underscores (`pallet_agent_registry`) but the directory and
`[package]` name in Cargo.toml use hyphens (`pallet-agent-registry`).

---

## Extrinsic Naming

**Format:** `verb_noun` in snake_case.

The verb describes the action; the noun describes the subject.

```
✅  register_agent
✅  update_reputation
✅  deregister_agent
✅  submit_review
✅  slash_reputation
✅  post_task
✅  bid_on_task
✅  complete_task
✅  invoke_service
✅  set_agent_status

❌  agentRegister      (camelCase)
❌  AgentRegister      (PascalCase)
❌  do_register        (vague verb)
❌  agent_registration (noun phrase)
```

---

## Error Naming

**Format:** PascalCase, descriptive noun phrase. Errors must be self-explanatory.

```
✅  NotFound
✅  AlreadyRegistered
✅  InsufficientReputation
✅  InvalidDid
✅  TaskNotActive
✅  BidTooLow
✅  NotTaskOwner
✅  QuotaExceeded

❌  Error1             (meaningless)
❌  BadRequest         (HTTP, not Substrate)
❌  Err                (too terse)
❌  registrationError  (camelCase)
```

Error docs must explain the condition:
```rust
#[pallet::error]
pub enum Error<T> {
    /// Agent with this ID or account is already registered.
    AlreadyRegistered,
    /// No agent found for the given account or ID.
    NotFound,
    /// Caller does not have sufficient reputation to perform this action.
    InsufficientReputation,
}
```

---

## Event Naming

**Format:** PascalCase, past tense. Events describe things that happened.

```
✅  AgentRegistered        { agent_id: AgentId, owner: T::AccountId }
✅  ReputationUpdated      { agent_id: AgentId, old: u32, new: u32 }
✅  TaskPosted             { task_id: TaskId, poster: T::AccountId }
✅  TaskCompleted          { task_id: TaskId, worker: T::AccountId }
✅  DisputeResolved        { task_id: TaskId, winner: T::AccountId }
✅  ServiceInvoked         { service_id: ServiceId, requester: T::AccountId }

❌  RegisterAgent          (imperative — this is a command, not an event)
❌  agentRegistered        (camelCase)
❌  agent_registered       (snake_case)
❌  AgentRegistration      (noun — ambiguous past/present)
```

Events must include all fields needed for off-chain indexers to reconstruct state:
```rust
#[pallet::event]
#[pallet::generate_deposit(pub(super) fn deposit_event)]
pub enum Event<T: Config> {
    /// A new agent was registered.
    AgentRegistered {
        agent_id: AgentId,
        owner: T::AccountId,
        agent_type: AgentType,
    },
}
```

---

## Storage Naming

**Format:** PascalCase, plural for maps (represent collections).

```
✅  Agents                 StorageMap<_, _, AccountId, AgentInfo>
✅  Tasks                  StorageMap<_, _, TaskId, TaskDetails>
✅  ReputationScores       StorageMap<_, _, AccountId, u32>
✅  ServiceListings        StorageMap<_, _, ServiceId, ServiceInfo>
✅  AgentCount             StorageValue<_, u64>    (singular OK for counters)
✅  NextTaskId             StorageValue<_, TaskId> (singular OK for counters)

❌  Agent                  (singular for a map — looks like it holds one)
❌  agentStorage           (camelCase + redundant "Storage")
❌  AGENTS                 (SCREAMING_SNAKE)
```

---

## Weight Function Naming

**Format:** snake_case, must match the extrinsic name exactly.

```rust
// Extrinsic name:
pub fn register_agent(...)

// Weight function (must match):
fn register_agent() -> Weight;

// In benchmarking.rs:
#[benchmark]
fn register_agent(n: Linear<1, 100>) -> Result<(), BenchmarkError> { ... }
```

---

## Module Structure

Each pallet crate follows this layout:

```
pallets/<name>/
  Cargo.toml
  src/
    lib.rs          ← pallet macro, config trait, storage, extrinsics, events, errors
    benchmarking.rs ← benchmark implementations (create even if empty)
    weights.rs      ← auto-generated weight file (committed, not hand-written)
    mock.rs         ← test runtime (only in #[cfg(test)])
    tests.rs        ← test module (or tests/ directory for large test suites)
    types.rs        ← shared types (optional, when lib.rs gets large)
    traits.rs       ← cross-pallet traits (optional, when providing services)
```

---

## Comments and Docs

- **Pallet-level:** `//!` module doc at top of `lib.rs` — what the pallet does, its extrinsics
- **Extrinsic-level:** `///` doc above each `pub fn` — what it does, who can call it, what it emits
- **Error-level:** `///` doc above each variant — when this error is returned
- **Event-level:** `///` doc above each variant — what triggered it

```rust
/// Register a new agent on ClawChain.
///
/// Emits [`Event::AgentRegistered`] on success.
///
/// # Errors
/// - [`Error::AlreadyRegistered`] if caller already has an agent.
/// - [`Error::InvalidDid`] if the DID format is malformed.
#[pallet::call_index(0)]
#[pallet::weight(T::WeightInfo::register_agent())]
pub fn register_agent(
    origin: OriginFor<T>,
    name: BoundedVec<u8, T::MaxNameLen>,
    agent_type: AgentType,
    capabilities: BoundedVec<BoundedVec<u8, T::MaxCapLen>, T::MaxCapabilities>,
) -> DispatchResult {
    let who = ensure_signed(origin)?;
    // ...
}
```
