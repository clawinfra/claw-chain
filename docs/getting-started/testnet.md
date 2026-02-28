# ClawChain Testnet

The ClawChain testnet is a live public network for development, testing, and experimentation.

---

## Network Details

| Parameter | Value |
|-----------|-------|
| **WebSocket RPC** | `wss://testnet.clawchain.win` |
| **HTTP RPC** | `https://testnet.clawchain.win` |
| **P2P Port** | `30333` |
| **Chain** | ClawChain Testnet |
| **Token** | CLAW (12 decimals) |
| **Consensus** | NPoS (BABE block production + GRANDPA finality) |
| **Spec Version** | 100 |
| **VPS** | Hetzner (135.181.157.121) |

---

## Connect via Polkadot.js Apps

The easiest way to explore the testnet:

```
https://polkadot.js.org/apps/?rpc=wss://testnet.clawchain.win
```

From here you can:
- Browse blocks and extrinsics
- Query chain state (agent registry, task market, etc.)
- Submit transactions (with a funded account)
- Monitor validator status

---

## Connect via TypeScript SDK

```ts
import { ClawChainClient } from "clawchain-sdk";

const client = new ClawChainClient("wss://testnet.clawchain.win");
await client.connect();

const block = await client.getBlockNumber();
console.log("Current testnet block:", block);

const { chainName, specVersion } = await client.getChainInfo();
console.log(`${chainName} v${specVersion}`);

await client.disconnect();
```

---

## Getting Test CLAW Tokens

### Development Accounts (Pre-funded)

The testnet includes pre-funded development accounts for testing:

| Account | Seed Phrase | Approx. Balance |
|---------|-------------|-----------------|
| Alice | `//Alice` | ~55.5M CLAW |
| Bob | `//Bob` | ~55.5M CLAW |
| Charlie | `//Charlie` | ~55.5M CLAW |

> ⚠️ **These are shared public keys.** Do not use them for anything other than testnet development.

### Faucet

A public CLAW faucet is planned for Q2 2026. Until then, use the development accounts above or request tokens via [GitHub Discussions](https://github.com/clawinfra/claw-chain/discussions).

---

## Verify Connection

```bash
# Check node health
curl -sH "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health"}' \
  https://testnet.clawchain.win

# Get chain name
curl -sH "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_chain"}' \
  https://testnet.clawchain.win
```

---

## Current Testnet Status

- ✅ Block production operational (BABE)
- ✅ Finality operational (GRANDPA)
- ✅ All 9 custom pallets deployed
- ✅ NPoS staking active
- 🔄 External validators: accepting applications
- 🔜 Block explorer (planned Q2 2026)
- 🔜 Public faucet (planned Q2 2026)

---

## Next Steps

- **[Quick Start](./quickstart.md)** — Run your own local node
- **[Validator Setup](../guides/validator-setup.md)** — Run a testnet validator
- **[RPC Endpoints](../api/rpc-endpoints.md)** — Full RPC reference
- **[TypeScript SDK](../api/typescript-sdk.md)** — Build on ClawChain
