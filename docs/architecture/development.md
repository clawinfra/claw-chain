# Development Guide

## Prerequisites

- **Rust** (stable + nightly): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Substrate dependencies**: `sudo apt install build-essential git clang curl libssl-dev protobuf-compiler`
- **WASM target**: `rustup target add wasm32-unknown-unknown`

## Build

```bash
# Clone
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain

# Build (first time takes ~15-30 min)
cargo build --release

# Binary location
./target/release/clawchain-node
```

## Run Dev Node

```bash
# Start in development mode (single validator, instant blocks)
./target/release/clawchain-node --dev

# With external RPC access (for EvoClaw connection)
./target/release/clawchain-node --dev --rpc-external --rpc-cors all

# With detailed logging
RUST_LOG=info ./target/release/clawchain-node --dev
```

### Dev Mode Features
- **Instant block sealing** — blocks produced on demand (no waiting)
- **Pre-funded accounts** — Alice, Bob, Charlie with test CLAW
- **Single authority** — Alice validates all blocks
- **Fresh start** — chain state resets on restart
- **No P2P** — local only, no networking needed

### Endpoints

| Endpoint | URL | Purpose |
|----------|-----|---------|
| WS-RPC | `ws://localhost:9944` | WebSocket API (primary) |
| HTTP-RPC | `http://localhost:9933` | HTTP API (legacy) |
| P2P | `localhost:30333` | Peer-to-peer (disabled in dev) |
| Prometheus | `localhost:9615` | Metrics |

## Connect from EvoClaw

### EvoClaw Agent Skill Configuration

```toml
# In EvoClaw agent.toml
[skills.clawchain]
enabled = true
node_url = "ws://localhost:9944"
wallet = "//Alice"  # Dev account
```

### Manual RPC Examples

```bash
# Check node health
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'

# Get chain info
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_chain","params":[],"id":1}'
# → "ClawChain Development"

# Get latest block
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_getBlock","params":[],"id":1}'

# Get agent count (once agent-registry is wired)
curl -s http://localhost:9933 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"state_getStorage","params":["0x<agent_count_key>"],"id":1}'
```

### Using Polkadot.js Apps

Browse your local dev chain with the Polkadot.js UI:

1. Go to https://polkadot.js.org/apps/
2. Click the network selector (top left)
3. Choose "Development" → "Local Node" → `ws://127.0.0.1:9944`
4. You'll see blocks, accounts, and can submit extrinsics

## Running Tests

```bash
# All tests
cargo test

# Specific pallet
cargo test -p pallet-agent-registry

# With output
cargo test -- --nocapture
```

## Project Structure

```
claw-chain/
├── Cargo.toml          ← Workspace root
├── node/               ← Node binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         ← Entry point
│       ├── cli.rs          ← CLI argument parsing
│       ├── command.rs      ← Command execution
│       ├── chain_spec.rs   ← Network configurations
│       ├── rpc.rs          ← Custom RPC endpoints
│       └── service.rs      ← Node service setup
│
├── runtime/            ← Runtime (compiles to WASM)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          ← Runtime config, pallet composition
│
├── pallets/            ← Custom ClawChain pallets
│   ├── agent-registry/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs      ← Agent DID, reputation, status
│   │
│   └── claw-token/
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs      ← Token economics, airdrop, treasury
│
├── whitepaper/         ← Technical whitepaper
├── branding/           ← Logos and assets
└── docs/               ← Documentation
    └── architecture/
        ├── overview.md     ← You are here
        ├── pallets.md      ← Pallet reference
        └── development.md  ← This file
```

## Adding a New Pallet

1. **Create the pallet:**
```bash
mkdir -p pallets/my-pallet/src
```

2. **Write `pallets/my-pallet/Cargo.toml`:**
```toml
[package]
name = "pallet-my-feature"
version = "0.1.0"
edition = "2021"

[dependencies]
frame-support = { ... }
frame-system = { ... }
```

3. **Implement `pallets/my-pallet/src/lib.rs`:**
```rust
#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::storage]
    pub type MyData<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000)]
        pub fn do_something(origin: OriginFor<T>, value: u32) -> DispatchResult {
            ensure_signed(origin)?;
            MyData::<T>::put(value);
            Ok(())
        }
    }
}
```

4. **Add to workspace `Cargo.toml`:**
```toml
[workspace]
members = ["node", "runtime", "pallets/my-pallet"]
```

5. **Wire into runtime `runtime/src/lib.rs`:**
```rust
impl pallet_my_feature::Config for Runtime {}

construct_runtime! {
    // ...
    MyFeature: pallet_my_feature,
}
```

6. **Test and submit PR.**

---

## See Also

- [Architecture Overview](./overview.md)
- [Pallet Reference](./pallets.md)
- [Contributing Guide](../../CONTRIBUTING.md)
