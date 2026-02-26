# Authority Rotation — PoA Bootstrap and NPoS Transition

This document describes how ClawChain manages PoA (Proof of Authority)
validators during the bootstrap phase and how the network will transition to
fully permissionless NPoS (Nominated Proof of Stake) as it matures.

---

## Table of Contents

1. [Overview](#overview)
2. [PoA Phase — Adding and Removing Authorities](#poa-phase)
3. [Key Rotation for Individual Validators](#key-rotation)
4. [Sudo Authority Management](#sudo-authority-management)
5. [Monitoring Finality During Rotation](#monitoring-finality-during-rotation)
6. [PoA → NPoS Transition Path](#transition-path)
7. [Emergency Procedures](#emergency-procedures)
8. [Security Checklist](#security-checklist)

---

## Overview {#overview}

ClawChain launches in **PoA mode** with a small set of pre-approved validator
nodes controlled by the core team and early partners.  During this phase:

- Authority list is maintained via `sudo` on-chain calls.
- Finality is provided by GRANDPA with the approved set.
- Staking pallet is active but `invulnerables` list prevents validators from
  being kicked by staking logic.

Once the network demonstrates stability and a sufficient validator ecosystem
exists, the `sudo` key will be destroyed and the NPoS election mechanism will
take over.

---

## PoA Phase — Adding and Removing Authorities {#poa-phase}

### Adding a New Authority

1. **Generate keys** for the new validator:

   ```bash
   clawchain-node generate-keys --output /tmp/new-validator-keys.json
   ```

2. **Share the public keys** (NOT the secret phrase) with the sudo holder.
   The new validator sends:
   - `aura.public_key` (sr25519, 0x-hex)
   - `grandpa.public_key` (ed25519, 0x-hex)
   - `aura.ss58_address` (stash / controller account)

3. **Fund the stash account** so it can bond the minimum stake (`STASH`).

4. **Update `authorities.json`** to include the new entry:

   ```json
   {
     "authorities": [
       {
         "name": "existing-validator",
         "stash": "5Grw...",
         "controller": "5Grw...",
         "aura": "0xd435...",
         "grandpa": "0x88dc..."
       },
       {
         "name": "new-validator",
         "stash": "5FHn...",
         "controller": "5FHn...",
         "aura": "0xabcd...",
         "grandpa": "0xef01..."
       }
     ],
     "sudo": "5GrwvaEF...",
     "endowed_accounts": [...]
   }
   ```

   > **Note**: `authorities.json` is NOT the chain-spec — it is the runtime
   > input used to generate a chain-spec or to propose on-chain extrinsics.
   > Modifying this file does not affect a running chain without a governance
   > transaction.

5. **Submit the `session::set_keys` extrinsic** from the new validator's
   controller account to register the new session keys on-chain.

6. **Submit `sudo(staking::force_new_era())`** or
   `sudo(staking::force_rotate_session())` to rotate into the new validator
   set on the next session boundary.

7. **Confirm finality** using the monitoring commands below.

### Removing an Authority

1. **Submit `sudo(staking::force_unstake(stash))`** to immediately remove the
   validator from the active set.
2. The validator will be excluded on the next era / session rotation.
3. Verify finality continues with the remaining set.
4. Update `authorities.json` and re-generate the chain-spec if a fresh genesis
   is ever needed.

> ⚠️ **Minimum validator count**: ClawChain requires at least
> `staking.minimumValidatorCount` (currently 1) active validators at all
> times. Never remove all validators simultaneously.

---

## Key Rotation for Individual Validators {#key-rotation}

Key rotation is recommended:

- Annually as a routine security measure.
- Immediately if a key may have been compromised.
- Before hardware decommission.

### Procedure

1. **Generate new keys** on the new/rotated hardware:

   ```bash
   clawchain-node generate-keys --output /tmp/rotated-keys.json
   ```

2. **Insert the new keys** into the keystore:

   ```bash
   # Aura (sr25519)
   clawchain-node key insert \
     --base-path /var/lib/clawchain/data \
     --keystore-path /var/lib/clawchain/keystore \
     --chain mainnet \
     --scheme Sr25519 \
     --key-type aura \
     --suri "<new Aura secret phrase>"

   # GRANDPA (ed25519)
   clawchain-node key insert \
     --base-path /var/lib/clawchain/data \
     --keystore-path /var/lib/clawchain/keystore \
     --chain mainnet \
     --scheme Ed25519 \
     --key-type gran \
     --suri "<new GRANDPA secret phrase>"
   ```

3. **Submit `session::set_keys`** from the controller account to register the
   new keys on-chain. The new keys take effect on the next session boundary —
   the old keys must remain in the keystore until the session boundary passes.

4. **Monitor finality** to confirm the rotation succeeded.

5. **Securely destroy** the old secret phrase after confirming the new keys are
   active.

---

## Sudo Authority Management {#sudo-authority-management}

### Current Sudo Usage

During the PoA phase, the `sudo` key can:

- Add / remove validators (`staking::force_unstake`, `session::*`)
- Schedule runtime upgrades (`system::set_code`)
- Transfer the sudo key itself (`sudo::set_key`)

### Securing the Sudo Key

- The sudo key should be a **hardware wallet** (Ledger / air-gapped machine).
- Consider a **multisig** account (e.g. 3-of-5 core team members) for the
  sudo role instead of a single hot key.
- Rotations require the existing sudo key to sign `sudo::set_key(new_key)`.

### Transferring Sudo

```
# On-chain, signed by current sudo account:
sudo::set_key(new_sudo_account)
```

After the call, the old sudo key has **no** further authority.  Broadcast the
new sudo account publicly so the community can verify.

### Removing Sudo (PoA → NPoS cutover)

When ready to hand control to NPoS governance:

```
# Signed by the current sudo account — irreversible!
sudo::sudo_unchecked_weight(
    sudo::remove_key(),
    Weight::MAX,
)
```

Once removed, no single account can make privileged changes.  All governance
must proceed through the on-chain democracy / council mechanisms.

---

## Monitoring Finality During Rotation {#monitoring-finality-during-rotation}

### Check Current Finalized Block

```bash
# Via WebSocket RPC (adjust the endpoint as needed)
wscat -c ws://localhost:9944 -x \
  '{"id":1,"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[]}'
```

### Watch GRANDPA Logs

```bash
journalctl -u clawchain -f | grep -E "grandpa|Finalized|target"
```

Expected healthy output includes lines like:
```
Finalized #12345 (0xabcd...)
GRANDPA voter: target #12346
```

### Prometheus Metrics to Watch

| Metric | Meaning |
|--------|---------|
| `substrate_finality_grandpa_precommits_total` | Precommit messages received |
| `substrate_finality_grandpa_prevotes_total`   | Prevote messages received |
| `substrate_block_height{status="finalized"}`  | Latest finalized block height |
| `substrate_block_height{status="best"}`       | Latest best block height |

A healthy network keeps `best - finalized` below ~3 blocks.  A stalled
finalizer shows no increase in the finalized metric.

### Alert Thresholds

| Condition | Action |
|-----------|--------|
| Finality lag > 10 blocks | Investigate connectivity |
| Finality lag > 100 blocks | Emergency key-holder call |
| Finality stalled > 5 min | Initiate emergency validator rotation |

---

## PoA → NPoS Transition Path {#transition-path}

The transition proceeds in three phases:

### Phase 1 — Permissioned NPoS (current target)

- NPoS election pallet is deployed and configured.
- Nominators can bond tokens and nominate validators.
- Sudo whitelist controls who can become a validator candidate.
- GRANDPA and Aura continue as consensus.

### Phase 2 — Open Nominations

- Whitelist removed; any adequately bonded account can become a candidate.
- Election runs on-chain via `pallet-election-provider-multi-phase`.
- Sudo is retained for emergency use only.

### Phase 3 — Full Decentralisation

- Sudo key removed (see [Removing Sudo](#removing-sudo)).
- Runtime upgrades require governance referendum.
- Validator set changes are fully permissionless.

### Pre-Transition Checklist

Before removing sudo and opening NPoS:

- [ ] Minimum bonding amount is economically meaningful (prevents Sybil attacks).
- [ ] Nomination limits are set (`max_nominators_count`, `max_nominations`).
- [ ] Slash logic is tested (equivocation detection, validator misbehaviour).
- [ ] Treasury funded to ~20% of total supply.
- [ ] At least 10 independent validator candidates tested on testnet.
- [ ] Community governance vote passed (off-chain signalling minimum).
- [ ] Runtime upgrade audited by a third party.
- [ ] Emergency response plan documented.

---

## Emergency Procedures {#emergency-procedures}

### Finality Stalled

1. Identify which validators are offline (missing heartbeats / missing votes).
2. Contact the offline validator operator.
3. If the operator cannot respond within 30 minutes, sudo-force-remove the
   validator (`staking::force_unstake`) and rotate the session.
4. Restart with remaining validators.

### Compromised Validator Key

1. Take the affected node **offline immediately**.
2. The operator submits `session::purge_keys()` from the controller account to
   de-register the session keys.
3. Submit `sudo(staking::force_unstake(stash))` to remove from the active set.
4. Generate new keys and rotate using the procedure above.
5. Investigate whether any equivocation occurred and whether slashing is
   needed.

### Compromised Sudo Key

1. If the sudo key has been compromised but not yet misused:
   - Immediately transfer sudo to a safe multisig account.
   - Rotate the hardware wallet or air-gapped machine.
2. If the attacker has already used the compromised sudo key:
   - Coordinate a community halt (all validators stop producing blocks).
   - Fork the chain from the last legitimate block via an emergency runtime upgrade.
   - This is a last resort and requires broad community consensus.

---

## Security Checklist {#security-checklist}

For each validator node:

- [ ] Secret phrases stored in encrypted, offline vault (e.g. Bitwarden offline, VeraCrypt).
- [ ] Keystore directory (`/var/lib/clawchain/keystore`) has mode `750`, owned by `clawchain:clawchain`.
- [ ] `authorities.json` has mode `640`, never committed to version control.
- [ ] SSH access to the validator host restricted to named public keys.
- [ ] Firewall: only P2P port (30333) exposed publicly. RPC port (9944) firewalled.
- [ ] Automatic OS security updates enabled.
- [ ] Node monitoring alerts configured (see [Prometheus Metrics](#monitoring)).
- [ ] Key rotation date recorded and reminder set for annual review.
- [ ] Recovery procedure documented and tested by a team member who did NOT
      write this documentation.
