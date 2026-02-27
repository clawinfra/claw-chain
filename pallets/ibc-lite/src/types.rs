//! IBC-lite types and data structures.

use super::Config;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use sp_core::H256;
use sp_std::prelude::*;

// =========================================================
// Type Aliases
// =========================================================

pub type Sequence = u64;
pub type AgentId = u64;
pub type ChannelId<T> = BoundedVec<u8, <T as Config>::MaxChannelIdLen>;
pub type ChainId<T> = BoundedVec<u8, <T as Config>::MaxChainIdLen>;
pub type RemoteAgentId<T> = BoundedVec<u8, <T as Config>::MaxChannelIdLen>;

// =========================================================
// Channel State
// =========================================================

/// Channel state machine.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ChannelState {
    /// Channel open has been initiated, awaiting confirmation.
    Init,
    /// Channel is fully open and operational.
    Open,
    /// Close has been initiated, awaiting confirmation.
    CloseInit,
    /// Channel is closed.
    Closed,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self::Init
    }
}

// =========================================================
// Channel Info
// =========================================================

/// Channel information stored on-chain.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct ChannelInfo<T: Config> {
    /// Channel identifier on this chain.
    pub channel_id: ChannelId<T>,
    /// Counterparty chain identifier.
    pub counterparty_chain_id: ChainId<T>,
    /// Counterparty channel identifier.
    pub counterparty_channel_id: ChannelId<T>,
    /// Current channel state.
    pub state: ChannelState,
    /// Block number when the channel was created.
    pub created_at: BlockNumberFor<T>,
    /// Block number when the channel was closed (if applicable).
    pub closed_at: Option<BlockNumberFor<T>>,
    /// Whether packets are ordered (false = unordered only in v1).
    pub ordered: bool,
}

// =========================================================
// Packet
// =========================================================

/// A cross-chain IBC-lite packet.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Packet<T: Config> {
    /// Monotonically increasing sequence number for this channel.
    pub sequence: Sequence,
    /// Source channel identifier on this chain.
    pub src_channel_id: ChannelId<T>,
    /// Destination channel identifier on counterparty chain.
    pub dst_channel_id: ChannelId<T>,
    /// Destination chain identifier.
    pub dst_chain_id: ChainId<T>,
    /// Source agent ID (None if sent by an account, not an agent).
    pub src_agent_id: Option<AgentId>,
    /// Destination agent ID on the counterparty chain.
    pub dst_agent_id: Option<RemoteAgentId<T>>,
    /// Typed payload.
    pub payload: PacketPayload<T>,
    /// Block number after which this packet is considered timed out.
    pub timeout_height: BlockNumberFor<T>,
    /// Packet creation block number.
    pub created_at: BlockNumberFor<T>,
}

// =========================================================
// Packet Payload
// =========================================================

/// Typed payload variants for agent messages.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub enum PacketPayload<T: Config> {
    /// Raw bytes (forward-compatible for future types).
    Raw(BoundedVec<u8, T::MaxPayloadLen>),
    /// Agent task delegation — delegate a task to a remote agent.
    TaskDelegate {
        task_id: u64,
        requester_agent: AgentId,
        assignee_hint: Option<RemoteAgentId<T>>,
        payload_hash: H256,
    },
    /// Agent reputation event — propagate reputation change cross-chain.
    ReputationUpdate {
        agent_id: AgentId,
        delta: i32,
        reason_hash: H256,
    },
    /// Cross-chain DID proof — prove agent identity across chains.
    DidProof {
        did: BoundedVec<u8, T::MaxPayloadLen>,
        proof_hash: H256,
    },
    /// Acknowledgement result payload.
    Ack {
        success: bool,
        error_code: Option<u32>,
        data: BoundedVec<u8, T::MaxPayloadLen>,
    },
}

// =========================================================
// Receipt Status
// =========================================================

/// Packet receipt status.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReceiptStatus {
    /// Packet has been received.
    Received,
    /// Packet has been processed.
    Processed,
}

// =========================================================
// Ack Status
// =========================================================

/// Packet acknowledgement status.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AckStatus {
    /// Whether the packet was successfully processed.
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_state_default() {
        assert_eq!(ChannelState::default(), ChannelState::Init);
    }
}
