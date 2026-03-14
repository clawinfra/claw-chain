# ClawChain Mainnet Genesis

## Status: DRAFT — Pending founder approval on token distribution

---

## Chain Parameters

| Parameter | Value |
|-----------|-------|
| Chain ID | `clawchain_mainnet` |
| Token Symbol | `CLAW` |
| Token Decimals | 18 |
| SS58 Format | 42 |
| Consensus | Aura (block production) + GRANDPA (finality) |

---

## ⚠️ Pending Decisions (requires founder input)

### 1. Total Supply
Proposed: **100,000,000 CLAW** (100M)
Options: 10M / 100M / 1B — needs decision.

### 2. Token Distribution
```
[ ] Foundation/Treasury:    ??%
[ ] Validator rewards pool: ??%
[ ] Team/Development:       ??%
[ ] Community/Ecosystem:    ??%
[ ] Founder:                ??%
```

### 3. Initial Validators
Currently 3 validators on 1 VPS (testnet).
Mainnet requires minimum 3 separate physical nodes.
Pending: Hetzner provisioning for V2 and V3 nodes.

### 4. Sudo/Governance Bootstrap
- Initial sudo key: who controls it?
- When to transition to on-chain governance?

### 5. Boot Nodes
Will be populated once mainnet nodes are provisioned.

---

## Next Steps
1. Founder confirms token distribution
2. Provision 2 additional Hetzner VPS nodes
3. Generate new validator keys for mainnet (separate from testnet keys)
4. Build final raw chain spec: `./target/release/clawchain build-spec --chain=chain-spec/clawchain-mainnet.json --raw > chain-spec/clawchain-mainnet-raw.json`
5. Distribute raw spec to all validator operators
6. Set genesis timestamp and launch
