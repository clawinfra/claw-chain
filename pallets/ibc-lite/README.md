# pallet-ibc-lite

**ClawChain IBC-lite Pallet** — Simplified cross-chain messaging for agent communication.

## Overview

IBC-lite is a simplified, opinionated subset of Cosmos IBC tailored for ClawChain's agent-messaging needs. It provides:

- **Channel management** — Simplified 2-step open/close handshake
- **Packet flow** — Send, receive, acknowledge, and timeout packets
- **Trusted relayers** — Multi-sig relayer set for Phase 1 (upgradable to light clients in Phase 2)
- **Agent-native payloads** — Task delegation, reputation updates, DID proofs
- **Cross-chain agent identity** — Map remote agents to local agents

## Architecture

### Design Principles

1. **Trusted first, trustless later** — Phase 1 uses trusted multi-sig relayers; Phase 2 adds light client proofs
2. **Agent-native** — Packet payloads are typed for agent messages
3. **Minimal state** — Only store what's needed for security (commitments, receipts, acks)
4. **FRAME-idiomatic** — Follows exact style of existing ClawChain pallets

### Scope vs Full Cosmos IBC

| Feature | IBC-lite | Full Cosmos IBC |
|---|---|---|
| Channels (simplified open/close) | ✅ | ✅ |
| Packet send/receive/ack | ✅ | ✅ |
| Timeout handling | ✅ | ✅ |
| Agent-specific message types | ✅ | ❌ |
| Trusted multi-sig relayers | ✅ (Phase 1) | ❌ |
| Light client proofs | ❌ (Phase 2+) | ✅ |
| Full ICS-02 client lifecycle | ❌ | ✅ |
| ICS-03 connection handshake (4-way) | ❌ → simplified 2-way | ✅ |
| ICS-04 channel ordering (ordered/unordered) | Unordered only | Both |
| ICS-20 fungible token transfer | ❌ (separate bridge pallet) | ✅ |

## Usage

### Channel Lifecycle

```rust
// 1. Open a channel (governance/sudo only)
IbcLite::open_channel(
    RawOrigin::Root.into(),
    b"counterparty-chain".to_vec(),
    b"counterparty-channel".to_vec(),
)?;

// 2. Channel is now in Init state; relayer confirms it becomes Open
// (In production, relayer observes ChannelOpened event and confirms)

// 3. Close a channel
IbcLite::close_channel_init(
    RawOrigin::Root.into(),
    b"channel-0".to_vec(),
)?;

// 4. Relayer confirms close
IbcLite::close_channel_confirm(
    RawOrigin::Signed(relayer_account).into(),
    b"channel-0".to_vec(),
)?;
```

### Sending Packets

```rust
// Send a raw payload
IbcLite::send_packet(
    RawOrigin::Signed(account).into(),
    b"channel-0".to_vec(),
    b"counterparty-chain".to_vec(),
    b"counterparty-channel".to_vec(),
    None, // dst_agent_id
    PacketPayload::Raw(b"hello world".to_vec().try_into().unwrap()),
)?;

// Send a task delegation
IbcLite::send_packet(
    RawOrigin::Signed(account).into(),
    b"channel-0".to_vec(),
    b"counterparty-chain".to_vec(),
    b"counterparty-channel".to_vec(),
    Some(b"remote-agent-42".to_vec()),
    PacketPayload::TaskDelegate {
        task_id: 12345,
        requester_agent: 1,
        assignee_hint: Some(b"remote-agent-42".to_vec().try_into().unwrap()),
        payload_hash: H256::from([0u8; 32]),
    },
)?;
```

### Relayer Operations

```rust
// Add a trusted relayer (governance/sudo only)
IbcLite::add_relayer(
    RawOrigin::Root.into(),
    relayer_account,
)?;

// Receive a packet (relayer only)
IbcLite::receive_packet(
    RawOrigin::Signed(relayer_account).into(),
    packet,
)?;

// Acknowledge a packet (relayer only)
IbcLite::acknowledge_packet(
    RawOrigin::Signed(relayer_account).into(),
    b"channel-0".to_vec(),
    sequence,
    PacketPayload::Ack {
        success: true,
        error_code: None,
        data: vec![].try_into().unwrap(),
    },
)?;
```

## Configuration

### Config Trait

```rust
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type WeightInfo: WeightInfo;
    type RelayerManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    type MaxRelayers: Get<u32>;
    type MaxChannelsPerChain: Get<u32>;
    type MaxChannelIdLen: Get<u32>;
    type MaxChainIdLen: Get<u32>;
    type MaxPayloadLen: Get<u32>;
    type MaxPendingPackets: Get<u32>;
    type PacketTimeoutBlocks: Get<u32>;
    type AgentRegistry: AgentRegistryInterface<Self::AccountId>;
}
```

### Constants

| Constant | Type | Description |
|---|---|---|
| `MaxRelayers` | `u32` | Maximum number of trusted relayers (e.g., 10) |
| `MaxChannelsPerChain` | `u32` | Maximum channels per chain (e.g., 100) |
| `MaxChannelIdLen` | `u32` | Max channel ID bytes (e.g., 128) |
| `MaxChainIdLen` | `u32` | Max chain ID bytes (e.g., 128) |
| `MaxPayloadLen` | `u32` | Max packet payload bytes (e.g., 4096) |
| `MaxPendingPackets` | `u32` | Max pending unacked packets (e.g., 1000) |
| `PacketTimeoutBlocks` | `u32` | Timeout in blocks (e.g., 100) |

## Storage

| Item | Type | Description |
|---|---|---|
| `Channels` | Map | Channel information keyed by channel ID |
| `ChannelsByChain` | Map | Channel IDs grouped by counterparty chain |
| `SendSequences` | Map | Next send sequence per channel |
| `RecvSequences` | Map | Next receive sequence per channel |
| `AckSequences` | Map | Next ack sequence per channel |
| `PacketCommitments` | DoubleMap | Packet commitments (deleted on ack) |
| `PacketReceipts` | DoubleMap | Packet receipts (prevents replay) |
| `PacketAcknowledgements` | DoubleMap | Packet ack results |
| `TrustedRelayers` | Value | Set of trusted relayer accounts |
| `CrossChainAgentMap` | DoubleMap | Maps (chain, remote_agent) → local_agent |

## Events

| Event | Description |
|---|---|
| `ChannelOpened` | New channel created |
| `ChannelCloseInitiated` | Channel close initiated |
| `ChannelClosed` | Channel closed |
| `PacketSent` | Packet sent to counterparty |
| `PacketReceived` | Packet received from relayer |
| `PacketAcknowledged` | Packet acknowledged |
| `PacketTimeout` | Packet timed out |
| `RelayerAdded` | New relayer added |
| `RelayerRemoved` | Relayer removed |
| `CrossChainAgentRegistered` | Cross-chain agent mapping created |

## Errors

| Error | Description |
|---|---|
| `ChannelNotFound` | Channel does not exist |
| `ChannelNotOpen` | Channel is not in Open state |
| `NotTrustedRelayer` | Caller is not a trusted relayer |
| `PacketAlreadyReceived` | Replay attack detected |
| `PacketTimedOut` | Packet timeout exceeded |
| `AgentNotFound` | Local agent does not exist |
| `CrossChainAgentAlreadyMapped` | Agent mapping already exists |

## Testing

Run unit tests:

```bash
cargo test --package pallet-ibc-lite
```

Run benchmarks:

```bash
cargo test --package pallet-ibc-lite --features runtime-benchmarks
```

## Phase Breakdown

### Phase 1 — Core IBC-lite (Trusted Multi-Sig) ✅
- Channel lifecycle (open/close)
- Packet send/receive/ack
- Trusted relayer set
- Cross-chain agent mapping

### Phase 1.5 — Multi-Sig Threshold (Optional)
- Require M-of-N relayer confirmations per packet
- Increases security without light clients

### Phase 2 — Trustless Light Client (Future)
- Light client state storage
- Merkle proof verification
- GRANDPA light client for ClawChain↔ClawChain
- Ethereum Merkle proof verifier for ETH bridge

## License

Apache 2.0

---

**See Also:**
- [ADR-005: ETH Bridge Design](../../docs/adr/005-eth-bridge.md)
- [pallet-agent-registry](../agent-registry)
- [pallet-agent-did](../agent-did)
