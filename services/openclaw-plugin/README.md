# @clawchain/openclaw-plugin

OpenClaw integration plugin for ClawChain. Registers agent DIDs on-chain at startup and exposes the `clawchain_status` skill for querying agent quota and reputation.

## Features

- **Automatic DID registration** — registers `did:claw:<accountId>` via `agentRegistry.registerAgent()` on startup
- **ed25519 keypair signing** — DID binding signed with OpenClaw agent keypair
- **Status skill** — queries DID registration, gas quota, and reputation score in one call
- **Idempotent** — safe to restart; skips re-registration if already on-chain

## Architecture

```
src/
  config.ts           — env var loading + validation
  plugin.ts           — orchestrator (OpenClawPlugin class)
  rpc/
    client.ts         — Substrate WebSocket RPC connection
  did/
    registrar.ts      — DID derivation + on-chain registration
  status/
    checker.ts        — quota, reputation, DID status queries
```

## Setup

### Requirements

- Node.js ≥ 18
- Access to a ClawChain node RPC endpoint
- Agent ed25519 keypair file

### Install

```bash
npm install
npm run build
```

### Configuration

Set environment variables:

| Variable | Required | Description |
|---|---|---|
| `CLAWCHAIN_RPC_URL` | ✅ | WebSocket RPC endpoint, e.g. `ws://localhost:9944` or `wss://rpc.clawchain.io` |
| `CLAWCHAIN_KEYPAIR_PATH` | ✅ | Path to keypair file (plain mnemonic string, or JSON with `mnemonic`/`seed`/`secretPhrase` field) |
| `CLAWCHAIN_CONNECT_TIMEOUT_MS` | ❌ | RPC connection timeout in ms (default: `30000`) |

#### Keypair file format

Plain mnemonic (`.txt`):
```
word word word word word word word word word word word word
```

JSON format (`.json`):
```json
{
  "mnemonic": "word word word word word word word word word word word word"
}
```

### Usage

**Register DID and exit:**
```bash
CLAWCHAIN_RPC_URL=ws://localhost:9944 \
CLAWCHAIN_KEYPAIR_PATH=./keypair.txt \
node dist/index.js
```

**Query agent status:**
```bash
CLAWCHAIN_RPC_URL=ws://localhost:9944 \
CLAWCHAIN_KEYPAIR_PATH=./keypair.txt \
node dist/index.js clawchain_status
```

Example output:
```json
{
  "success": true,
  "command": "clawchain_status",
  "data": {
    "did": {
      "registered": true,
      "identifier": "did:claw:5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
      "registeredAt": 1234567
    },
    "gasQuota": {
      "available": "10000",
      "used": "250",
      "resetAt": 9876543
    },
    "reputation": {
      "score": 850,
      "tier": "Gold",
      "updatedAt": 1234000
    },
    "queriedAt": "2026-02-27T01:36:00.000Z"
  }
}
```

### Programmatic API

```typescript
import { OpenClawPlugin, loadConfig } from '@clawchain/openclaw-plugin';

const config = loadConfig(); // reads from env
const plugin = new OpenClawPlugin(config);

// Initialize — connects to RPC and registers DID
await plugin.initialize();

// Query status
const result = await plugin.handleCommand('clawchain_status');
console.log(JSON.stringify(result.data, null, 2));

// Cleanup
await plugin.shutdown();
```

## Tests

```bash
npm test            # run tests with coverage
npm run test:watch  # watch mode
```

Coverage threshold: **90% lines/functions/statements**, **80% branches**.

## DID Format

Agent DIDs follow the `did:claw` method:
```
did:claw:<ss58-account-id>
```

Where `<ss58-account-id>` is the SS58-encoded public key of the agent's ed25519 keypair.

## Pallets Used

| Pallet | Query | Extrinsic |
|---|---|---|
| `agentRegistry` | `agentRegistry(did)` | `registerAgent(did)` |
| `gasQuota` | `agentQuotas(accountId)` | — |
| `reputation` | `reputations(accountId)` | — |
