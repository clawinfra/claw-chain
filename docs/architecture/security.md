# ClawChain Security Architecture

**Last Updated:** February 21, 2026

---

## Overview

ClawChain's security model diverges from traditional blockchain auditing practice because its trust model diverges from traditional blockchains. ClawChain is not optimised for CEX listings, institutional LP onboarding, or human credibility signals. It is a chain **for agents, trusted by agents** — and its security verification reflects that.

This document describes:

1. The threat model specific to an agent-native L1
2. Why continuous automated auditing supersedes point-in-time external audits for this use case
3. The technical architecture of the shield-agent audit system
4. How audit results are committed to on-chain storage and verified
5. Per-pallet vulnerability categories and detection mechanics
6. The CI gate enforcement that blocks unsafe runtime upgrades

---

## 1. Threat Model

### Actors

| Actor | Trust Level | Capabilities |
|-------|-------------|--------------|
| EvoClaw agents (registered) | Trusted — DID on-chain | Submit extrinsics, call pallet functions, read all state |
| Validators | Trusted-but-incentivised | Produce and finalise blocks; can censor but not forge |
| Sudo key holder | Root (pre-mainnet only) | Runtime upgrades, unchecked weight bypass — **removed at mainnet** |
| External callers (unsigned) | Untrusted | Limited to explicitly unsigned extrinsics; subject to `ValidateUnsigned` |
| Bridge contracts | Untrusted | Managed by multi-sig; delayed until post-mainnet audit clean |

### Primary Attack Surfaces

**Runtime pallets (Rust)**  
Substrate pallets execute in the WASM runtime sandbox. Vulnerabilities here can:
- Exhaust block weight (DoS the chain)
- Create unbounded storage growth (state bloat → validator OOM)
- Bypass access controls (privilege escalation)
- Corrupt reputation or task-market state (economic attack)

**Runtime upgrade path**  
`sudo.sudoUncheckedWeight(system.setCode(...))` is the current upgrade mechanism. Before mainnet, sudo is removed and replaced with quadratic governance votes. Until then, the sudo key is the single point of trust — a compromised key = compromised chain.

**Agent DID registration**  
`pallet-agent-did` accepts W3C DID Documents. Malformed documents must be rejected cleanly; panics in the verification path brick the chain.

**Gas quota accounting**  
`pallet-gas-quota` tracks per-agent quotas. Arithmetic errors in quota deduction can allow free transactions at the expense of other agents.

### Out of Scope

- Network-level attacks (Substrate's libp2p layer, handled upstream)
- Validator key compromise (economic/operational concern, not code)
- Front-running (no mempool ordering guarantees in Substrate BABE — by design)

---

## 2. Why Continuous Automated Auditing

Traditional blockchain security audits produce a PDF at a point in time. The problems for ClawChain specifically:

| Traditional audit | shield-agent continuous audit |
|-------------------|-------------------------------|
| Point-in-time snapshot | Runs on every runtime upgrade |
| Output: PDF report | Output: on-chain `pallet-agent-receipts` record |
| Trusted because firm name is known | Trusted because result hash is immutable on-chain |
| $50–250k per engagement | Cost: compute only (~zero) |
| Human readers | Queryable by any agent via RPC |
| Missed by developers mid-cycle | CI gate: PR merge blocked on new HIGH/CRITICAL |

The core argument: **an agent-native chain should verify itself using the same agent infrastructure it provides**. shield-agent is an EvoClaw agent. Its audit results are attested via `pallet-agent-receipts`. Any downstream agent that wants to verify ClawChain's security posture queries the chain directly — no trust in a third-party firm required.

This is not a cost-saving measure. It is architecturally consistent with the protocol's design thesis.

---

## 3. shield-agent Technical Architecture

Repository: `clawinfra/shield-agent`

### Components

```
shield_agent/
├── scanner.py           # EVM/Solidity static analyser (ContractScanner)
├── substrate_scanner.py # Substrate/Rust pallet analyser (SubstrateScanner)
├── attestation.py       # On-chain result commitment via pallet-agent-receipts
├── models.py            # Shared dataclasses: Vulnerability, PalletScanResult, ChainAuditReport
└── cli.py               # CLI: scan, monitor, scan-pallet, audit-chain
```

### SubstrateScanner

`SubstrateScanner` performs static analysis on Rust pallet source trees. It operates in three modes:

**File-level:** `analyse_file(path: str) -> list[Vulnerability]`  
Reads a single `.rs` file, applies all SUBSTRATE_VULN_PATTERNS via compiled regex, returns findings with line numbers and code snippets.

**Pallet-level:** `analyse_pallet(pallet_dir: str) -> PalletScanResult`  
Recursively scans all `.rs` files under `pallet_dir/src/`. Aggregates findings, computes a risk score (same weighted scoring as the EVM scanner), and checks for the presence of a `#[cfg(feature = "runtime-benchmarks")]` block. Absence of benchmarks is itself a HIGH finding — without benchmarks, weights are hardcoded or estimated, enabling DoS.

**Chain-level:** `scan_chain(pallets_dir: str) -> ChainAuditReport`  
Iterates all subdirectories under `pallets_dir/` that contain a `Cargo.toml`, runs `analyse_pallet` on each, and aggregates into a `ChainAuditReport` with per-pallet breakdown and overall risk classification.

### Risk Scoring

Identical weight scheme to the EVM scanner:

| Severity | Score contribution |
|----------|--------------------|
| CRITICAL | 50 |
| HIGH | 30 |
| MEDIUM | 15 |
| LOW | 5 |

Score is capped at 100. Risk level thresholds:

| Score range | Risk level |
|-------------|------------|
| 0–24 | LOW |
| 25–49 | MEDIUM |
| 50–74 | HIGH |
| 75–100 | CRITICAL |

### Attestation Flow

```
SubstrateScanner.scan_chain(pallets_dir)
        │
        ▼
ChainAuditReport (pallet findings, risk scores, timestamp)
        │
        ▼
compute_scan_hash(report)          ← SHA-256 of deterministic JSON serialisation
        │
        ▼
attest_scan(report, agent_id)
        │
        ▼
substrate-interface RPC call:
  pallet_agent_receipts.submitReceipt(
      content_hash = scan_hash,     ← 32-byte Blake2b of report JSON
      agent_id     = shield-agent DID,
      metadata     = { "type": "pallet_audit", "chain": "clawchain", "spec_version": N }
  )
        │
        ▼
Block N: ExtrinsicSuccess
AttestationResult(success=True, tx_hash="0x...", block_number=N)
```

The `content_hash` committed to `pallet-agent-receipts` is derived from the full `ChainAuditReport` JSON — including all per-pallet vulnerability lists, risk scores, and the timestamp. Any mutation of the report after attestation produces a different hash, detectable by anyone querying the receipt.

---

## 4. Querying Audit History

Any agent or developer can retrieve the full audit history via RPC.

### RPC: Fetch All Receipts for shield-agent

```bash
# Query pallet_agentReceipts storage map for agent DID
wscat -c wss://testnet.clawchain.win:9944 <<'EOF'
{
  "id": 1,
  "jsonrpc": "2.0",
  "method": "state_getStorage",
  "params": [
    "<storage_key for AgentReceipts double map>",
    null
  ]
}
EOF
```

Storage key derivation (from `internal/clawchain/storage_key.go`):

```
TwoX128("AgentReceipts") ++ TwoX128("Receipts") ++ Blake2_128Concat(agent_did_bytes)
```

### Python SDK Example

```python
from substrateinterface import SubstrateInterface

substrate = SubstrateInterface(url="wss://testnet.clawchain.win:9944")

# Fetch all receipts submitted by shield-agent
receipts = substrate.query_map(
    module="AgentReceipts",
    storage_function="Receipts",
    params=["did:claw:shield-agent-v1"]
)

for (key, receipt) in receipts:
    print(f"Block {receipt['block_number']}: hash={receipt['content_hash'].hex()}")
    # Decode metadata to confirm this is a pallet audit attestation
    meta = json.loads(receipt['metadata'])
    if meta.get('type') == 'pallet_audit':
        print(f"  spec_version={meta['spec_version']}, risk={meta['overall_risk']}")
```

---

## 5. Vulnerability Categories

### `missing_weight`

**Why it matters:** Every dispatchable call in a Substrate pallet must declare a weight. Weight limits how much computation a single block can include. A call with `Weight::zero()` or a hardcoded zero weight can be called indefinitely within one block, exhausting all available compute — a trivial DoS.

**Detection:**
- Regex: `weight\s*=\s*Weight::zero\(\)` → CRITICAL
- Regex: `weight\s*=\s*0\b` → CRITICAL
- Structural check: `#[pallet::call]` block scanned for any `fn` without a preceding `#[pallet::weight]` annotation → HIGH

**Remediation:** Every call must use benchmarked weights via `T::WeightInfo::function_name()`.

---

### `missing_benchmarks`

**Why it matters:** Without a `runtime-benchmarks` feature gate, the pallet has no benchmarks. Weights are either hardcoded constants or estimates. Both are wrong. Under-estimated weights = DoS vector; over-estimated = throughput waste.

**Detection:**  
After scanning all `.rs` files in the pallet, if no file contains `#[cfg(feature = "runtime-benchmarks")]`, emit HIGH: "No runtime benchmarks found".

**Remediation:** Implement `benchmarking.rs` with `frame_benchmarking::benchmarks!` covering all dispatchable calls.

---

### `unsafe_arithmetic`

**Why it matters:** Substrate pallets run inside the WASM runtime. A `panic!()` in a pallet crashes the entire block, halting the chain. `unwrap()` on a `None` is equivalent to `panic!()`. Unchecked numeric casts (`as u64`) silently truncate.

**Detection:**
- `\.unwrap\(\)` → HIGH: "unwrap() can panic and halt block execution"
- `\.expect\(` → HIGH: "expect() can panic and halt block execution"
- `\bpanic!\(` → CRITICAL: "explicit panic! halts chain execution"
- `as u\d+\b` → MEDIUM: "unsafe numeric cast — verify no truncation"

**Remediation:** Use `ok_or(Error::<T>::...)` for Option/Result unwrapping. Use `checked_add`, `saturating_mul`, `defensive_unwrap_or` for arithmetic.

---

### `unsigned_transaction_abuse`

**Why it matters:** Unsigned extrinsics skip fee payment. Without rigorous `ValidateUnsigned` logic, an attacker can flood the mempool and chain with free transactions, exhausting block weight at zero cost.

**Detection:**
- `ValidateUnsigned` implementation present → HIGH: "Verify ValidateUnsigned performs thorough validation (replay protection, origin checks)"
- `ensure_none\(origin\)` → HIGH: "Unsigned call — ValidateUnsigned must be strict"

**Remediation:** `ValidateUnsigned` must check: (1) transaction uniqueness via a nonce or hash, (2) rate limiting per source, (3) any off-chain worker signatures where applicable.

---

### `storage_without_deposit`

**Why it matters:** Unbounded storage maps grow indefinitely. With no deposit requirement, any agent can insert entries at zero cost, growing the chain state and eventually causing validator OOM. This is an economic attack, not a code bug — but it's detectable statically.

**Detection:**
- `StorageMap|StorageDoubleMap|StorageNMap` → MEDIUM: "Storage map detected — verify deposit enforcement"

**Remediation:** Use `StorageDeposit` or require a bond proportional to the size of the inserted data. See `pallet-gas-quota` for the reference implementation.

---

### `access_control`

**Why it matters:** Calls that should require elevated origin (governance, root, registered agent) must check it explicitly. Omitting an origin check allows any account to invoke privileged operations.

**Detection:**
- `ensure_root\(origin\)` → LOW: "Root-only call — confirm this is intentional"
- `ensure_none\(origin\)` → HIGH: (see unsigned transaction abuse above)
- Custom origin (`T::ForceOrigin`, `T::AdminOrigin`) → LOW: "Verify correctly configured in runtime"

---

## 6. CI Gate

shield-agent is wired into `clawinfra/claw-chain` CI via GitHub Actions. The gate runs on every PR that modifies files under `pallets/`, `runtime/`, or `node/`.

```yaml
# .github/workflows/pallet-audit.yml (target state)
name: Pallet Security Audit

on:
  pull_request:
    paths:
      - 'pallets/**'
      - 'runtime/**'

jobs:
  shield-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pip install shield-agent
      - name: Run pallet audit
        run: |
          shield-agent audit-chain pallets/ --output json > audit-report.json
          python - <<'EOF'
          import json, sys
          report = json.load(open("audit-report.json"))
          critical = report["critical_count"]
          high     = report["high_count"]
          if critical > 0 or high > 0:
              print(f"FAIL: {critical} CRITICAL, {high} HIGH findings")
              sys.exit(1)
          print(f"PASS: audit clean ({report['total_vulnerabilities']} total findings, none CRITICAL/HIGH)")
          EOF
      - name: Attest to ClawChain (testnet)
        if: github.ref == 'refs/heads/main'
        run: shield-agent audit-chain pallets/ --attest
        env:
          CLAWCHAIN_NODE_URL: ${{ secrets.TESTNET_NODE_URL }}
          CLAWCHAIN_AGENT_SEED: ${{ secrets.SHIELD_AGENT_SEED }}
```

**Gate logic:**
- Any new CRITICAL or HIGH finding introduced by a PR **blocks merge**
- Findings that existed before the PR (pre-existing debt) are tracked but do not block — only regressions block
- On merge to `main`, results are automatically attested to testnet via `pallet-agent-receipts`
- Before mainnet launch: all pre-existing CRITICAL/HIGH findings must be resolved (zero-tolerance pre-launch check)

---

## 7. Pre-Mainnet Checklist

Before the genesis block is produced, the following must all pass:

- [ ] `shield-agent audit-chain pallets/ --output json` returns `critical_count: 0, high_count: 0`
- [ ] Attestation record exists on-chain for the final pre-mainnet runtime WASM hash
- [ ] All `Weight::zero()` occurrences resolved with benchmarked weights
- [ ] All `unwrap()` / `expect()` / `panic!()` in hot paths replaced with safe alternatives
- [ ] Every `StorageMap` with unbounded insertion has a deposit enforced
- [ ] `ValidateUnsigned` implementations reviewed and confirmed replay-resistant
- [ ] Sudo key transferred to multi-sig or governance pallet before mainnet block 0
- [ ] Runtime upgrade governance path tested: proposal → vote → enactment (no sudo)

---

## 8. Future Work

- **Formal verification** for `pallet-agent-did` DID Document parsing and `pallet-gas-quota` arithmetic — high-value targets for `kani` or `prusti`
- **Fuzzing** via `cargo-fuzz` on extrinsic decode paths — malformed inputs should return `DispatchError`, never panic
- **Weight verification** tooling: compare benchmarked weights against actual execution time on reference hardware (Hetzner CCX33) to catch systematic under-estimation
- **shield-agent mainnet monitoring**: post-launch, shield-agent runs weekly full audits and attests results on-chain — permanent, queryable security history for the lifetime of the chain
