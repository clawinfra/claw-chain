# Wiring Instructions for Task Market and Reputation Pallets

This document provides the exact code snippets to add to the runtime to integrate the `pallet-task-market` and `pallet-reputation` pallets into ClawChain.

**IMPORTANT:** Another sub-agent is working on staking pallets. Coordinate with them or do this as a final merge step.

---

## Step 1: Add to Workspace `Cargo.toml`

**File:** `/home/bowen/claw-chain/Cargo.toml`

Add to the `[workspace] members` array:

```toml
members = [
    "node",
    "runtime",
    "pallets/agent-registry",
    "pallets/claw-token",
    "pallets/reputation",        # ADD THIS
    "pallets/task-market",       # ADD THIS
]
```

Add to `[workspace.dependencies]`:

```toml
# ClawChain pallets
pallet-agent-registry = { path = "pallets/agent-registry", default-features = false }
pallet-claw-token = { path = "pallets/claw-token", default-features = false }
pallet-reputation = { path = "pallets/reputation", default-features = false }    # ADD THIS
pallet-task-market = { path = "pallets/task-market", default-features = false }  # ADD THIS
```

---

## Step 2: Add to Runtime `Cargo.toml`

**File:** `/home/bowen/claw-chain/runtime/Cargo.toml`

Add to `[dependencies]`:

```toml
pallet-reputation = { workspace = true }
pallet-task-market = { workspace = true }
```

Add to the `std` feature list:

```toml
std = [
    # ... existing entries ...
    "pallet-reputation/std",
    "pallet-task-market/std",
]
```

Add to the `runtime-benchmarks` feature list (if present):

```toml
runtime-benchmarks = [
    # ... existing entries ...
    "pallet-reputation/runtime-benchmarks",
    "pallet-task-market/runtime-benchmarks",
]
```

Add to the `try-runtime` feature list (if present):

```toml
try-runtime = [
    # ... existing entries ...
    "pallet-reputation/try-runtime",
    "pallet-task-market/try-runtime",
]
```

---

## Step 3: Configure Pallets in Runtime

**File:** `/home/bowen/claw-chain/runtime/src/lib.rs`

### 3.1: Add parameter_types

Add these parameter definitions (place near other pallet parameter_types):

```rust
parameter_types! {
    // Reputation parameters
    pub const MaxCommentLength: u32 = 256;
    pub const InitialReputation: u32 = 5000;
    pub const MaxReputationDelta: u32 = 500;
    pub const MaxHistoryLength: u32 = 100;
    
    // Task Market parameters
    pub const TaskMarketPalletId: PalletId = PalletId(*b"taskmark");
    pub const MaxTitleLength: u32 = 128;
    pub const MaxDescriptionLength: u32 = 1024;
    pub const MaxProposalLength: u32 = 512;
    pub const MaxBidsPerTask: u32 = 20;
    pub const MinTaskReward: Balance = 100 * UNITS; // 100 CLAW minimum
    pub const MaxActiveTasksPerAccount: u32 = 50;
}
```

**Note:** Adjust `UNITS` based on your token decimals. If UNITS is not defined, replace with the appropriate multiplier (e.g., `100_000_000_000_000_000` for 18 decimals).

### 3.2: Implement Config traits

Add the Config implementations (place with other pallet Config impls):

```rust
impl pallet_reputation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_reputation::WeightInfo;
    type Currency = Balances;
    type MaxCommentLength = MaxCommentLength;
    type InitialReputation = InitialReputation;
    type MaxReputationDelta = MaxReputationDelta;
    type MaxHistoryLength = MaxHistoryLength;
}

impl pallet_task_market::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_task_market::WeightInfo;
    type Currency = Balances;
    type ReputationManager = Reputation;
    type PalletId = TaskMarketPalletId;
    type MaxTitleLength = MaxTitleLength;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MaxProposalLength = MaxProposalLength;
    type MaxBidsPerTask = MaxBidsPerTask;
    type MinTaskReward = MinTaskReward;
    type MaxActiveTasksPerAccount = MaxActiveTasksPerAccount;
}
```

### 3.3: Add to construct_runtime! macro

Find the `construct_runtime!` macro and add these entries:

```rust
construct_runtime!(
    pub enum Runtime
    {
        // ... existing pallets ...
        Balances: pallet_balances,
        // ... other pallets ...
        
        // ClawChain pallets
        AgentRegistry: pallet_agent_registry,
        ClawToken: pallet_claw_token,
        Reputation: pallet_reputation,      // ADD THIS
        TaskMarket: pallet_task_market,     // ADD THIS
    }
);
```

**Order matters:** Reputation must come before TaskMarket since TaskMarket depends on it.

---

## Step 4: Build and Test

After making all the changes above, compile the runtime:

```bash
cd /home/bowen/claw-chain
export PATH="/home/bowen/.cargo/bin:/home/bowen/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"

# Check the pallets individually
cargo check -p pallet-reputation
cargo check -p pallet-task-market

# Check the runtime
cargo check -p claw-chain-runtime

# Run tests
cargo test -p pallet-reputation
cargo test -p pallet-task-market

# Build the full node
cargo build --release
```

---

## Step 5: Genesis Configuration (Optional)

If you want to pre-configure reputation scores or tasks in the genesis block, add to your chain spec (e.g., `node/src/chain_spec.rs`):

```rust
reputation: ReputationConfig {
    // Pre-seed some accounts with reputation if needed
    // (Currently, accounts start with InitialReputation automatically)
    ..Default::default()
},
task_market: TaskMarketConfig {
    ..Default::default()
},
```

---

## Verification Checklist

- [ ] Both pallets compile without errors
- [ ] All tests pass (`cargo test -p pallet-reputation -p pallet-task-market`)
- [ ] Runtime compiles without errors
- [ ] No duplicate pallet names in `construct_runtime!`
- [ ] Reputation pallet is listed before TaskMarket in `construct_runtime!`
- [ ] All Config trait bounds are satisfied

---

## Troubleshooting

### "ReputationManager trait not found"
- Ensure `pallet-reputation` is in dependencies with `default-features = false`
- Check that Reputation pallet is declared before TaskMarket in `construct_runtime!`

### "Balance type mismatch"
- Ensure both pallets use `type Currency = Balances`
- Check that `MinTaskReward` uses the same Balance type and units

### "PalletId already in use"
- TaskMarketPalletId is `*b"taskmark"` - ensure no other pallet uses this ID
- You can change it to something else if needed (e.g., `*b"clwtasks"`)

---

## Cross-Pallet Integration

The reputation system automatically updates when:

1. **Task Posted**: `on_task_posted()` increments `total_tasks_posted` and `total_spent`
2. **Task Approved**: `on_task_completed()` increments `total_tasks_completed`, `successful_completions`, and `total_earned`
3. **Dispute Resolved**: `on_dispute_resolved()` adjusts scores (+200 for winner, -500 for loser) and updates `disputes_won`/`disputes_lost`

Reviews are submitted manually via `Reputation::submit_review()` and give +100 to +500 reputation based on rating (1-5 stars).

---

## Future Enhancements

- Add minimum reputation requirements for bidding on tasks (currently commented out in `bid_on_task`)
- Add task categories/tags with BoundedVec<u8, MaxTagLength>
- Add proof storage (IPFS hash) with dedicated StorageMap
- Add milestone-based payments (split escrow into multiple releases)
- Add automatic dispute resolution with oracle integration
- Add reputation decay over time for inactive accounts
