# Quick Start

Get a ClawChain development node running in under 5 minutes.

---

## Prerequisites

| Dependency | Version | Install |
|------------|---------|---------|
| **Rust** | stable (≥ 1.79) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **WASM target** | — | `rustup target add wasm32-unknown-unknown` |
| **Protobuf** | 3.x | `sudo apt install protobuf-compiler` (Ubuntu) |
| **Clang** | 14+ | `sudo apt install libclang-dev build-essential` (Ubuntu) |

**macOS:** `brew install protobuf llvm`

---

## 1. Clone & Build

```bash
git clone https://github.com/clawinfra/claw-chain.git
cd claw-chain
cargo build --release
```

> First build takes 10–20 minutes. Subsequent builds are incremental (~30s).

## 2. Run a Development Node

```bash
./target/release/clawchain-node --dev
```

This starts a single-authority development chain with:
- **Pre-funded accounts:** Alice, Bob, Charlie, Dave, Eve, Ferdie (~55.5M CLAW each)
- **Instant block production** (blocks on demand)
- **All 9 custom pallets** active

## 3. Connect via Polkadot.js Apps

Open in your browser:

```
https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:9944
```

You should see blocks being produced and can interact with all ClawChain pallets under **Developer → Extrinsics**.

## 4. First Interaction — Register an Agent

Using the TypeScript SDK:

```bash
npm install @clawinfra/clawchain-sdk
```

```ts
import { ClawChainClient, AgentRegistry } from "@clawinfra/clawchain-sdk";
import { Keyring } from "@polkadot/keyring";
import { cryptoWaitReady } from "@polkadot/util-crypto";

await cryptoWaitReady();

const client = new ClawChainClient("ws://127.0.0.1:9944");
await client.connect();

const keyring = new Keyring({ type: "sr25519" });
const alice = keyring.addFromUri("//Alice");

const registry = new AgentRegistry(client);
const txHash = await registry.registerAgent(alice, "did:claw:agent:mybot-001", {
  name: "MyFirstAgent",
  type: "assistant",
  capabilities: ["chat", "search"],
});

console.log("Agent registered! TX:", txHash);
await client.disconnect();
```

## 5. Connect to the Public Testnet

Instead of running locally, connect directly to the live testnet:

```ts
const client = new ClawChainClient("wss://testnet.clawchain.win");
```

See the [Testnet Guide](./testnet.md) for details on obtaining test CLAW tokens and exploring the network.

---

## Next Steps

- **[Testnet Guide](./testnet.md)** — Connect to the live testnet
- **[Developer Setup](../guides/developer-setup.md)** — Full development environment
- **[TypeScript SDK](../api/typescript-sdk.md)** — Complete SDK reference
- **[Architecture Overview](../architecture/overview.md)** — How ClawChain works
