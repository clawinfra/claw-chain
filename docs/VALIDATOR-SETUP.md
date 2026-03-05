# Validator Key Setup & Rotation

Quick-reference guide for generating validator keys, adding them to the chain spec, and rotating session keys on ClawChain.

> For the full validator setup guide (hardware, systemd, monitoring), see [guides/validator-setup.md](guides/validator-setup.md).

---

## Prerequisites

- ClawChain node binary installed (`clawchain-node --version`)
- Access to a running node's RPC endpoint (default: `http://localhost:9944`)
- [Polkadot.js Apps](https://polkadot.js.org/apps/) or `subxt` CLI for on-chain submissions

---

## 1. Generate Validator Keys

ClawChain validators need three key pairs:

| Key | Scheme | Purpose |
|-----|--------|---------|
| **Stash** | sr25519 | Holds bonded CLAW (cold storage) |
| **Controller** | sr25519 | Signs operational transactions |
| **Session keys** | sr25519 (Aura) + ed25519 (GRANDPA) | Block production + finality |

### Generate Stash & Controller Keys

```bash
# Generate stash key (sr25519)
clawchain-node key generate --scheme sr25519 --output-type json > stash-key.json

# Generate controller key (sr25519)
clawchain-node key generate --scheme sr25519 --output-type json > controller-key.json
```

Each command outputs:
```json
{
  "secretPhrase": "word1 word2 ... word12",
  "secretSeed": "0x...",
  "publicKey": "0x...",
  "ss58Address": "5Grw..."
}
```

> 🔐 **Back up secret phrases offline immediately.** Never store stash secrets on the validator server.

### Generate Session Keys (Aura + GRANDPA)

**Option A — Via RPC (recommended, node must be running):**

```bash
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
    http://localhost:9944 | jq -r '.result'
# Returns: 0xaabbcc... (hex-encoded session key blob)
```

**Option B — Manual key insertion:**

```bash
# Generate Aura key (sr25519)
clawchain-node key generate --scheme sr25519 --output-type json > aura-key.json

# Generate GRANDPA key (ed25519)
clawchain-node key generate --scheme ed25519 --output-type json > grandpa-key.json

# Insert into node keystore
clawchain-node key insert \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --scheme sr25519 \
    --suri "$(jq -r .secretPhrase aura-key.json)" \
    --key-type aura

clawchain-node key insert \
    --base-path /var/lib/clawchain \
    --chain /etc/clawchain/clawchain-testnet.json \
    --scheme ed25519 \
    --suri "$(jq -r .secretPhrase grandpa-key.json)" \
    --key-type gran
```

### Verify Keys Are Loaded

```bash
# List keys in keystore
ls /var/lib/clawchain/chains/clawchain_testnet/keystore/
# Should show files for aura and gran key types

# Verify via RPC
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_hasSessionKeys","params":["0xYOUR_SESSION_KEYS_HEX"]}' \
    http://localhost:9944
# Returns: {"result": true}
```

---

## 2. Add Validator Keys to Chain Spec

To include a new validator in genesis (for new network bootstrapping):

### Step 1 — Export a human-readable chain spec

```bash
clawchain-node build-spec --chain local > chain-spec.json
```

### Step 2 — Edit the genesis authorities

In `chain-spec.json`, add your validator's session keys to the `session.keys` array:

```json
{
  "session": {
    "keys": [
      [
        "5GrwStashAddress...",
        "5GrwStashAddress...",
        {
          "aura": "5AuraPublicKey...",
          "grandpa": "5GrandpaPublicKey..."
        }
      ]
    ]
  }
}
```

Also add the stash account to initial balances:

```json
{
  "balances": {
    "balances": [
      ["5GrwStashAddress...", 10000000000000000]
    ]
  }
}
```

### Step 3 — Convert to raw format

```bash
clawchain-node build-spec \
    --chain chain-spec.json \
    --raw > chain-spec-raw.json
```

### Step 4 — Distribute the raw spec

All validators must use the same raw chain spec file. Distribute via:
- GitHub repository (`specs/` directory)
- Direct transfer to validator operators

---

## 3. Rotate Session Keys

Rotate session keys periodically (every 3–6 months) or after any suspected compromise.

### Step 1 — Generate new keys on the running node

```bash
NEW_KEYS=$(curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' \
    http://localhost:9944 | jq -r '.result')

echo "New session keys: $NEW_KEYS"
```

### Step 2 — Submit new keys on-chain

**Via Polkadot.js Apps:**
1. Navigate to **Network → Staking → Accounts**
2. Click **Set Session Key** next to your validator stash
3. Paste the hex `$NEW_KEYS` value
4. Sign with your **controller** account

**Via CLI (`subxt` or custom script):**
```bash
# Using polkadot-js API (Node.js)
node -e "
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
(async () => {
    const api = await ApiPromise.create({ provider: new WsProvider('ws://localhost:9944') });
    const keyring = new Keyring({ type: 'sr25519' });
    const controller = keyring.addFromUri('//ControllerSeed');
    const tx = api.tx.session.setKeys('$NEW_KEYS', '0x');
    await tx.signAndSend(controller);
    console.log('Session keys updated on-chain');
    process.exit(0);
})();
"
```

### Step 3 — Wait for activation

- New keys are **queued** and take effect after the current session ends (typically 2 sessions)
- **Do NOT restart the node or delete old keys** until new keys are active

### Step 4 — Verify activation

```bash
curl -s -H "Content-Type: application/json" \
    --data '{"id":1,"jsonrpc":"2.0","method":"author_hasSessionKeys","params":["'"$NEW_KEYS"'"]}' \
    http://localhost:9944
# Should return: {"result": true}
```

### Rotation Checklist

- [ ] Generate new keys via `author_rotateKeys`
- [ ] Submit `session.setKeys` on-chain with controller
- [ ] Wait 2 sessions for activation
- [ ] Verify with `author_hasSessionKeys`
- [ ] Confirm validator is still producing blocks
- [ ] Securely delete any exported key files

---

## Security Best Practices

1. **Never expose RPC port 9944 publicly** on a validator — use a firewall or reverse proxy
2. **Use separate machines** for stash key management and validator operation
3. **Enable NTP** to prevent clock drift (causes missed blocks)
4. **Monitor key expiry** — set calendar reminders for rotation schedule
5. **Test rotation on testnet first** before mainnet
6. **Keep offline backups** of stash and controller mnemonics in encrypted storage

---

## See Also

- [Full Validator Setup Guide](guides/validator-setup.md) — hardware, systemd, monitoring, troubleshooting
- [Authority Rotation](authority-rotation.md) — on-chain authority set management
- [Deployment Guide](guides/deploy-node.md) — Docker and Podman deployment

---

*Last updated: 2026-03-05 | ClawChain Validator Key Setup*
