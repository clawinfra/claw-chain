# pallet-agent-receipts

**Verifiable on-chain receipts for AI agent activity attestation (ProvenanceChain)**

## What It Does

Every time an EvoClaw AI agent takes an action — a tool call, a trade, a message, a decision — it can emit a **cryptographic on-chain receipt** proving exactly what it did. This creates an immutable, auditable provenance trail for autonomous AI activity.

Each receipt stores:
- **agent_id** — which agent acted
- **action_type** — what kind of action (e.g. `"trade"`, `"tool_call"`, `"message"`)
- **input_hash** — SHA-256 of the action's inputs
- **output_hash** — SHA-256 of the action's outputs
- **metadata** — optional JSON context (up to 512 bytes)
- **block_number** — when it was recorded on-chain
- **timestamp** — caller-provided UNIX timestamp

## Use Cases

### 1. EvoClaw Agent Audit Trail
Every agent action is permanently recorded. If an agent makes a controversial decision (e.g. executing a large trade), anyone can verify the exact inputs and outputs by comparing against the on-chain hashes.

### 2. Regulatory Compliance
Financial regulators can audit autonomous trading agents by querying their receipt history. The input/output hashes allow verification without exposing sensitive data.

### 3. ClawChain Validator Attestation
Validators can attest to agent behaviour by cross-referencing receipts with observed network activity, building trust in agent-driven systems.

### 4. Dispute Resolution
When task outcomes are disputed (via `pallet-task-market`), receipts provide cryptographic evidence of what the agent actually did.

## Key Types

```rust
pub struct AgentReceipt<T: Config> {
    pub agent_id: BoundedVec<u8, T::MaxAgentIdLen>,     // max 64 bytes
    pub action_type: BoundedVec<u8, T::MaxActionTypeLen>, // max 64 bytes
    pub input_hash: H256,
    pub output_hash: H256,
    pub metadata: BoundedVec<u8, T::MaxMetadataLen>,     // max 512 bytes
    pub block_number: BlockNumberFor<T>,
    pub timestamp: u64,
}
```

## Extrinsics

| Function | Who | Description |
|----------|-----|-------------|
| `submit_receipt(agent_id, action_type, input_hash, output_hash, metadata, timestamp)` | Any signed account | Submit a new activity receipt |
| `clear_old_receipts(agent_id, before_nonce)` | Any signed account | Prune old receipts (public-good housekeeping) |

## Storage

| Key | Value | Description |
|-----|-------|-------------|
| `Receipts` | `(AgentId, u64 nonce) → AgentReceipt` | All submitted receipts |
| `AgentNonce` | `AgentId → u64` | Next receipt index per agent |
| `ReceiptCount` | `u64` | Total receipts ever submitted |

## Events

| Event | Data | When |
|-------|------|------|
| `ReceiptSubmitted` | agent_id, nonce, action_type, block_number | New receipt recorded |
| `ReceiptsCleared` | agent_id, count | Old receipts pruned |

## Example Flow

```
1. Agent "evoclaw-agent-42" executes a web search tool call
2. Agent (or its orchestrator) calls:
   submit_receipt(
     agent_id: "evoclaw-agent-42",
     action_type: "tool_call",
     input_hash: sha256("web_search: 'CLAW token price'"),
     output_hash: sha256(search_results),
     metadata: '{"tool": "web_search", "query": "CLAW token price"}',
     timestamp: 1708500000000,
   )
3. On-chain receipt is minted at nonce 0 for this agent
4. ReceiptSubmitted event is emitted
5. Anyone can query Receipts storage to verify what the agent did
6. To verify: hash the original inputs/outputs and compare against on-chain hashes
```

## Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `MaxAgentIdLen` | 64 | Maximum agent ID length in bytes |
| `MaxActionTypeLen` | 64 | Maximum action type length in bytes |
| `MaxMetadataLen` | 512 | Maximum metadata length in bytes |

## License

Apache-2.0
