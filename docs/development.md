# ClawChain Development Guide

## Prerequisites

### System Dependencies

**Ubuntu/Debian:**
```bash
# Build essentials
sudo apt-get install -y build-essential git curl

# Substrate dependencies
sudo apt-get install -y libclang-dev protobuf-compiler

# If libclang-dev is not available, ensure libclang.so is findable:
export LIBCLANG_PATH=/usr/lib/llvm-18/lib
```

**macOS:**
```bash
brew install protobuf llvm
```

### Rust Toolchain

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target (for runtime compilation)
rustup target add wasm32v1-none

# If wasm32v1-none is not available (Rust < 1.84), use:
rustup target add wasm32-unknown-unknown
```

## Building

### Full Build

```bash
# Debug build
cargo build

# Release build (optimized, recommended for running)
cargo build --release
```

### Check Only (faster, no binary output)

```bash
cargo check --workspace
```

### Environment Variables

If you encounter build issues, you may need:

```bash
# Set these before building
export LIBCLANG_PATH=/usr/lib/llvm-18/lib
export PROTOC=$(which protoc)

# If clang can't find system headers
export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/gcc/x86_64-linux-gnu/13/include"
```

## Running the Dev Node

```bash
# Start in development mode (single authority, pre-funded accounts)
./target/release/clawchain-node --dev

# With detailed logging
RUST_LOG=info ./target/release/clawchain-node --dev

# Purge chain data and start fresh
./target/release/clawchain-node purge-chain --dev
./target/release/clawchain-node --dev
```

### Dev Accounts

The development chain comes with pre-funded accounts:

| Account | Address | Balance |
|---------|---------|---------|
| Alice | `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY` | ~55.5M CLAW |
| Bob | `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty` | ~55.5M CLAW |
| Charlie | `5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y` | ~55.5M CLAW |
| Dave | `5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy` | ~55.5M CLAW |
| Eve | `5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw` | ~55.5M CLAW |
| Ferdie | `5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSneWj6g3Mg8` | ~55.5M CLAW |

Alice is the sole authority (block producer) in dev mode.

## Connecting to the Node

### Polkadot.js Apps

Visit [https://polkadot.js.org/apps](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer) and connect to:
```
ws://127.0.0.1:9944
```

### RPC Examples

**Get system info:**
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_name"}' \
  http://localhost:9944
```

**Get chain info:**
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_chain"}' \
  http://localhost:9944
```

**Check account balance:**
```bash
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_account", "params": ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"]}' \
  http://localhost:9944
```

## Connecting from EvoClaw

EvoClaw (or any agent) can interact with ClawChain via the JSON-RPC interface:

```python
import requests

RPC_URL = "http://localhost:9944"

def rpc_call(method, params=None):
    payload = {
        "id": 1,
        "jsonrpc": "2.0",
        "method": method,
        "params": params or []
    }
    response = requests.post(RPC_URL, json=payload)
    return response.json()

# Get system info
print(rpc_call("system_name"))
print(rpc_call("system_chain"))
print(rpc_call("system_version"))
```

For submitting extrinsics (transactions), use the Substrate API libraries:
- **Python:** [substrate-interface](https://github.com/nickclaw/py-substrate-interface)
- **JavaScript:** [@polkadot/api](https://polkadot.js.org/docs/api)
- **Rust:** `subxt` crate

### Registering an Agent (via Polkadot.js)

1. Connect to the dev node at `ws://127.0.0.1:9944`
2. Go to Developer → Extrinsics
3. Select `agentRegistry` → `registerAgent`
4. Fill in:
   - `did`: `did:claw:evoclaw001` (hex-encoded)
   - `metadata`: `{"name": "EvoClaw", "type": "assistant"}` (hex-encoded)
5. Submit transaction signed by Alice

## Testing Pallets

### Run All Tests

```bash
cargo test --workspace
```

### Run Specific Pallet Tests

```bash
# Agent Registry tests
cargo test -p pallet-agent-registry

# CLAW Token tests
cargo test -p pallet-claw-token
```

### Run Tests with Output

```bash
cargo test -p pallet-agent-registry -- --nocapture
```

## Project Structure

```
claw-chain/
├── Cargo.toml           # Workspace root
├── node/                # Node binary (networking, RPC, CLI)
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       ├── main.rs      # Entry point
│       ├── cli.rs       # CLI argument definitions
│       ├── command.rs   # Command dispatch
│       ├── chain_spec.rs # Dev + testnet chain specs
│       ├── rpc.rs       # RPC extensions
│       └── service.rs   # Node service assembly
├── runtime/             # Runtime (compiles to WASM)
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/
│       └── lib.rs       # Runtime configuration, pallet composition
├── pallets/             # Custom ClawChain pallets
│   ├── agent-registry/  # Agent identity on-chain
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs   # Pallet logic
│   │       └── tests.rs # Unit tests
│   └── claw-token/      # CLAW tokenomics
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs   # Pallet logic
│           └── tests.rs # Unit tests
├── whitepaper/          # Project documentation
└── docs/                # Development docs
```

## Key Concepts

### Agent Registry Pallet

The Agent Registry is the core ClawChain primitive. Each agent has:
- **AgentId**: Sequential unique identifier (u64)
- **DID**: Decentralized identifier string
- **Metadata**: JSON blob with agent capabilities
- **Reputation**: Score from 0-10000 (basis points)
- **Status**: Active, Suspended, or Deregistered

### CLAW Token Pallet

Extends Substrate's built-in balances with:
- **Contributor scoring**: Track off-chain contributions for airdrop
- **Airdrop claims**: Proportional token distribution based on scores
- **Treasury**: Community-governed spending

### Tokenomics

- Total supply: 1,000,000,000 CLAW (12 decimal places)
- 40% airdrop (400M CLAW)
- 30% validator rewards (300M CLAW)
- 20% treasury (200M CLAW)
- 10% team (100M CLAW)
