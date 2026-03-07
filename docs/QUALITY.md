# Quality Standards — ClawChain

## Test Coverage

**Target: 90% per pallet** (enforced in CI for PRs that modify pallet code).

### What to test

Every pallet must have tests for:
- **Happy path** — extrinsic succeeds, correct storage mutation, correct event emitted
- **Error paths** — all `Error<T>` variants are reachable from at least one test
- **Origin checks** — unsigned calls fail, wrong-privilege calls return `BadOrigin`
- **Boundary conditions** — max-length inputs, zero values, saturating arithmetic

### Test structure

```rust
// pallets/<name>/src/tests.rs
use crate::mock::*;
use crate::Error;
use frame_support::{assert_noop, assert_ok};

#[test]
fn register_agent_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistry::register_agent(
            RuntimeOrigin::signed(1),
            b"test-agent".to_vec().try_into().unwrap(),
            AgentType::Autonomous,
            vec![].try_into().unwrap(),
        ));
        // Assert storage
        assert!(Agents::<Test>::contains_key(1));
        // Assert event
        System::assert_last_event(
            Event::AgentRegistered { agent_id: 0, owner: 1 }.into()
        );
    });
}

#[test]
fn register_agent_fails_when_already_registered() {
    new_test_ext().execute_with(|| {
        // ... setup ...
        assert_noop!(
            AgentRegistry::register_agent(origin, ...),
            Error::<Test>::AlreadyRegistered
        );
    });
}
```

### Running coverage

```bash
# Install tarpaulin once
cargo install cargo-tarpaulin

# Per-pallet coverage (fast)
cargo tarpaulin -p pallet-agent-registry --out Html

# Full workspace coverage
cargo tarpaulin --workspace --out Html
```

---

## Storage Migrations

**All storage migrations must have tests. No exceptions.**

Migration test template:
```rust
#[test]
fn migration_v1_to_v2_works() {
    new_test_ext().execute_with(|| {
        // 1. Write old storage format directly
        // 2. Run migration
        let weight = migrations::v2::migrate::<Test>();
        // 3. Assert new storage is correct
        // 4. Assert migration is idempotent (run twice = same result)
    });
}
```

Migration checklist before merging:
- [ ] `StorageVersion` incremented
- [ ] Pre-migration state test (old format → new format)
- [ ] Idempotency test (safe to run twice)
- [ ] Weight calculation accounts for storage reads/writes
- [ ] Tried-on testnet before mainnet

---

## Security-Sensitive Functions

The following extrinsics have elevated security requirements:

### `update_reputation` (pallet-reputation, pallet-agent-registry)
**Risk:** Arbitrary reputation manipulation could corrupt the trust system.
**Required:** Must validate origin — only governance, sudo, or task-market (via Config trait) may call.
```rust
pub fn update_reputation(origin: OriginFor<T>, ...) -> DispatchResult {
    // Must have one of:
    ensure_root!(origin)?;                      // sudo
    T::GovernanceOrigin::ensure_origin(origin)?; // governance
    // OR come through the ReputationManager trait (called by task-market, not directly)
}
```

### `treasury_spend` (pallet-claw-token or governance)
**Risk:** Direct treasury drain.
**Required:** `ensure_root!(origin)` or multi-sig governance origin.

### `invoke_service` (pallet-service-market)
**Risk:** Could trigger arbitrary service execution with economic side effects.
**Required:** `ensure_signed!(origin)` and verify caller is the service requester or an approved delegate.

### General rule
Every extrinsic must begin with an explicit origin check. No extrinsic may rely on
implicit origin validation.

```rust
// ✅ CORRECT — explicit
pub fn my_extrinsic(origin: OriginFor<T>) -> DispatchResult {
    let who = ensure_signed(origin)?;
    // ...
}

// ❌ WRONG — origin never checked
pub fn my_extrinsic(origin: OriginFor<T>) -> DispatchResult {
    let _ = origin;
    // ...
}
```

---

## Benchmarks

**Required for all extrinsics before mainnet deployment.**

### Structure

```
pallets/<name>/src/
  benchmarking.rs   ← benchmark implementations
  weights.rs        ← auto-generated weight file (commit this)
```

### Benchmark template

```rust
// pallets/<name>/src/benchmarking.rs
use super::*;
use frame_benchmarking::v2::*;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_agent(n: Linear<1, 100>) -> Result<(), BenchmarkError> {
        let caller: T::AccountId = whitelisted_caller();
        let name = vec![b'a'; n as usize].try_into().unwrap();
        #[extrinsic_call]
        register_agent(RawOrigin::Signed(caller), name, AgentType::Autonomous, vec![].try_into().unwrap());
        assert!(Agents::<T>::contains_key(&caller));
        Ok(())
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
```

### Running benchmarks

```bash
# Build with benchmarks feature
cargo build --release --features runtime-benchmarks

# Run benchmarks and generate weights
./target/release/clawchain benchmark pallet \
  --pallet pallet_agent_registry \
  --extrinsic "*" \
  --output pallets/agent-registry/src/weights.rs \
  --template .maintain/frame-weight-template.hbs
```

---

## CI Gates Summary

| Gate | Command | Failure = block PR? |
|------|---------|---------------------|
| Compiler | `cargo build --workspace` | Yes |
| Tests | `cargo test --workspace` | Yes |
| Lints | `cargo clippy --workspace -- -D warnings` | Yes |
| Agent lints | `bash scripts/agent-lint.sh` | Yes |
| Coverage | `cargo tarpaulin` (≥90%) | Yes (pallet PRs only) |
| Benchmarks | manual + committed weights.rs | Yes (mainnet PRs only) |
