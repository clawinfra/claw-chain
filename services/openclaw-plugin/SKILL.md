# ClawChain OpenClaw Plugin

## Skill: `clawchain_status`

Query your agent's on-chain status: DID registration, gas quota, and reputation score.

### Description

This skill connects to a ClawChain node via Substrate JSON-RPC and returns a JSON snapshot of the agent's current on-chain state. It also handles DID registration at startup if the agent is not yet registered.

### Setup

1. Install dependencies:
   ```bash
   cd services/openclaw-plugin
   npm install && npm run build
   ```

2. Set environment variables:
   ```bash
   export CLAWCHAIN_RPC_URL=ws://localhost:9944
   export CLAWCHAIN_KEYPAIR_PATH=/path/to/agent-keypair.txt
   ```

3. Add to your OpenClaw agent startup:
   ```bash
   node services/openclaw-plugin/dist/index.js
   ```

### Commands

#### `clawchain_status`

Returns the agent's current on-chain status.

**Usage:**
```bash
node dist/index.js clawchain_status
```

**Output schema:**
```json
{
  "success": true,
  "command": "clawchain_status",
  "data": {
    "did": {
      "registered": true,
      "identifier": "did:claw:<accountId>",
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

### Startup Behaviour

On first run (or any run where the DID is not yet on-chain), the plugin automatically submits `agentRegistry.registerAgent(did)` signed with the agent's ed25519 keypair. The call is idempotent — if the DID is already registered, it logs and skips.

### Configuration Reference

| Env Var | Required | Default | Description |
|---|---|---|---|
| `CLAWCHAIN_RPC_URL` | Yes | — | WebSocket endpoint (`ws://` or `wss://`) |
| `CLAWCHAIN_KEYPAIR_PATH` | Yes | — | Path to mnemonic file or JSON keypair |
| `CLAWCHAIN_CONNECT_TIMEOUT_MS` | No | `30000` | RPC connect timeout (ms) |

### Programmatic Integration

```typescript
import { OpenClawPlugin } from '@clawchain/openclaw-plugin';

const plugin = new OpenClawPlugin({
  rpcUrl: process.env.CLAWCHAIN_RPC_URL!,
  keypairPath: process.env.CLAWCHAIN_KEYPAIR_PATH!,
  connectTimeoutMs: 30_000,
});

await plugin.initialize(); // registers DID if needed
const status = await plugin.handleCommand('clawchain_status');
```
