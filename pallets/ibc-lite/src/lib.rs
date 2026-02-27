//! # IBC-lite Pallet
//!
//! Simplified cross-chain messaging for ClawChain agents.
//!
//! ## Overview
//!
//! IBC-lite is a simplified subset of Cosmos IBC tailored for agent messaging.
//! Supports channel management, packet flow, and relayer interface.
//!
//! ## Dispatchable Functions
//!
//! ### Channel Management
//! - `open_channel` - Open a new channel to a counterparty chain
//! - `close_channel_init` - Initiate channel closure
//! - `close_channel_confirm` - Confirm channel closure (relayer only)
//!
//! ### Packet Operations
//! - `send_packet` - Send a packet to a counterparty chain
//! - `receive_packet` - Receive a packet from a relayer
//! - `acknowledge_packet` - Acknowledge a packet (relayer only)
//! - `timeout_packet` - Timeout an unacknowledged packet
//!
//! ### Relayer Management
//! - `add_relayer` - Add a trusted relayer
//! - `remove_relayer` - Remove a trusted relayer
//!
//! ### Cross-Chain Agents
//! - `register_cross_chain_agent` - Register a cross-chain agent mapping

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

pub mod traits;
pub mod types;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sp_core::H256;
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use traits::AgentRegistryInterface;

    // Import types from the types module
    pub use crate::types::{AgentId, ChainId, ChannelId, ChannelState, RemoteAgentId, Sequence};

    // =========================================================
    // Config
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: weights::WeightInfo;

        /// Origin that can add/remove relayers (e.g. governance or sudo).
        type RelayerManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of trusted relayers.
        #[pallet::constant]
        type MaxRelayers: Get<u32>;

        /// Maximum number of channels per chain.
        #[pallet::constant]
        type MaxChannelsPerChain: Get<u32>;

        /// Maximum byte length of a channel identifier.
        #[pallet::constant]
        type MaxChannelIdLen: Get<u32>;

        /// Maximum byte length of a chain identifier.
        #[pallet::constant]
        type MaxChainIdLen: Get<u32>;

        /// Maximum byte length of a packet payload.
        #[pallet::constant]
        type MaxPayloadLen: Get<u32>;

        /// Maximum pending unacknowledged packets per channel.
        #[pallet::constant]
        type MaxPendingPackets: Get<u32>;

        /// Number of blocks before an unacknowledged packet times out.
        #[pallet::constant]
        type PacketTimeoutBlocks: Get<u32>;

        /// Interface to agent-registry for cross-chain agent identity validation.
        type AgentRegistry: AgentRegistryInterface<Self::AccountId>;
    }

    // =========================================================
    // Pallet
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    /// Global counter for channel IDs (auto-increment).
    #[pallet::storage]
    #[pallet::getter(fn channel_counter)]
    pub type ChannelCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// All channels, keyed by channel ID.
    #[pallet::storage]
    #[pallet::getter(fn channels)]
    pub type Channels<T: Config> =
        StorageMap<_, Blake2_128Concat, ChannelId<T>, ChannelInfo<T>, OptionQuery>;

    /// Channel IDs grouped by counterparty chain (for discovery).
    #[pallet::storage]
    pub type ChannelsByChain<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ChainId<T>,
        BoundedVec<ChannelId<T>, T::MaxChannelsPerChain>,
        ValueQuery,
    >;

    /// Next sequence number to use when sending on a channel.
    #[pallet::storage]
    pub type SendSequences<T: Config> =
        StorageMap<_, Blake2_128Concat, ChannelId<T>, Sequence, ValueQuery>;

    /// Next expected receive sequence for a channel.
    #[pallet::storage]
    pub type RecvSequences<T: Config> =
        StorageMap<_, Blake2_128Concat, ChannelId<T>, Sequence, ValueQuery>;

    /// Next expected ack sequence for a channel.
    #[pallet::storage]
    pub type AckSequences<T: Config> =
        StorageMap<_, Blake2_128Concat, ChannelId<T>, Sequence, ValueQuery>;

    /// Packet commitments — hash stored until packet is acknowledged.
    #[pallet::storage]
    pub type PacketCommitments<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ChannelId<T>,
        Blake2_128Concat,
        Sequence,
        H256,
        OptionQuery,
    >;

    /// Packet receipts — marks a sequence as received (prevents replay).
    #[pallet::storage]
    pub type PacketReceipts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ChannelId<T>,
        Blake2_128Concat,
        Sequence,
        ReceiptStatus,
        OptionQuery,
    >;

    /// Packet acknowledgements — stores ack result after processing.
    #[pallet::storage]
    pub type PacketAcknowledgements<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ChannelId<T>,
        Blake2_128Concat,
        Sequence,
        AckStatus,
        OptionQuery,
    >;

    /// Set of trusted relayers that may submit packets and acks.
    #[pallet::storage]
    #[pallet::getter(fn trusted_relayers)]
    pub type TrustedRelayers<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxRelayers>, ValueQuery>;

    /// Maps (remote_chain_id, remote_agent_id) → local AgentId.
    #[pallet::storage]
    pub type CrossChainAgentMap<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ChainId<T>,
        Blake2_128Concat,
        RemoteAgentId<T>,
        AgentId,
        OptionQuery,
    >;

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ChannelOpened {
            channel_id: Vec<u8>,
            counterparty_chain: Vec<u8>,
            counterparty_channel: Vec<u8>,
        },
        ChannelCloseInitiated {
            channel_id: Vec<u8>,
        },
        ChannelClosed {
            channel_id: Vec<u8>,
        },
        PacketSent {
            channel_id: Vec<u8>,
            sequence: Sequence,
            src_agent: Option<AgentId>,
            payload_hash: H256,
        },
        PacketReceived {
            channel_id: Vec<u8>,
            sequence: Sequence,
            dst_agent: Option<RemoteAgentId<T>>,
        },
        PacketAcknowledged {
            channel_id: Vec<u8>,
            sequence: Sequence,
            success: bool,
        },
        PacketTimeout {
            channel_id: Vec<u8>,
            sequence: Sequence,
        },
        RelayerAdded {
            relayer: T::AccountId,
        },
        RelayerRemoved {
            relayer: T::AccountId,
        },
        CrossChainAgentRegistered {
            chain_id: Vec<u8>,
            remote_agent_id: RemoteAgentId<T>,
            local_agent_id: AgentId,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        ChannelNotFound,
        ChannelAlreadyExists,
        ChannelNotOpen,
        ChannelAlreadyClosed,
        SequenceMismatch,
        PacketNotFound,
        PacketAlreadyReceived,
        PacketAlreadyAcknowledged,
        PacketTimedOut,
        NotTrustedRelayer,
        TooManyRelayers,
        TooManyChannels,
        RelayerAlreadyRegistered,
        RelayerNotFound,
        PayloadTooLong,
        InvalidPacketCommitment,
        AgentNotFound,
        CrossChainAgentAlreadyMapped,
        ChainIdTooLong,
        ChannelIdTooLong,
        InvalidAgent,
        PendingPacketLimitExceeded,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Open a new channel to a counterparty chain.
        ///
        /// Creates a channel in Init state. Relayer confirms opening.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::open_channel())]
        pub fn open_channel(
            origin: OriginFor<T>,
            counterparty_chain_id: Vec<u8>,
            counterparty_channel_id: Vec<u8>,
        ) -> DispatchResult {
            T::RelayerManagerOrigin::ensure_origin(origin)?;

            // Validate lengths
            let chain_id: ChainId<T> = counterparty_chain_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChainIdTooLong)?;
            let counterparty_channel: ChannelId<T> = counterparty_channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            // Generate new channel ID
            let channel_number = ChannelCounter::<T>::get();
            let channel_id: ChannelId<T> = format!("channel-{}", channel_number)
                .into_bytes()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;
            ChannelCounter::<T>::put(channel_number + 1);

            // Check max channels per chain
            let existing_channels = ChannelsByChain::<T>::get(&chain_id);
            ensure!(
                existing_channels.len() < T::MaxChannelsPerChain::get() as usize,
                Error::<T>::TooManyChannels
            );

            let now = <frame_system::Pallet<T>>::block_number();
            let channel_info = ChannelInfo::<T> {
                channel_id: channel_id.clone(),
                counterparty_chain_id: chain_id.clone(),
                counterparty_channel_id: counterparty_channel.clone(),
                state: ChannelState::Init,
                created_at: now,
                closed_at: None,
                ordered: false, // Unordered only in v1
            };

            Channels::<T>::insert(&channel_id, channel_info);
            ChannelsByChain::<T>::mutate(&chain_id, |channels| {
                if let Err(_) = channels.try_push(channel_id.clone()) {
                    return Err(());
                }
                Ok(())
            })
            .map_err(|_| Error::<T>::TooManyChannels)?;

            // Initialize sequences
            SendSequences::<T>::insert(&channel_id, 1u64);
            RecvSequences::<T>::insert(&channel_id, 1u64);
            AckSequences::<T>::insert(&channel_id, 1u64);

            Self::deposit_event(Event::ChannelOpened {
                channel_id: channel_id.to_vec(),
                counterparty_chain: counterparty_chain,
                counterparty_channel: counterparty_channel,
            });

            Ok(())
        }

        /// Initiate channel closure.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::close_channel_init())]
        pub fn close_channel_init(origin: OriginFor<T>, channel_id: Vec<u8>) -> DispatchResult {
            T::RelayerManagerOrigin::ensure_origin(origin)?;

            let bounded_channel_id: ChannelId<T> = channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            Channels::<T>::try_mutate(&bounded_channel_id, |maybe_channel| -> DispatchResult {
                let channel = maybe_channel.as_mut().ok_or(Error::<T>::ChannelNotFound)?;
                ensure!(
                    channel.state == ChannelState::Open,
                    Error::<T>::ChannelNotOpen
                );
                channel.state = ChannelState::CloseInit;
                Ok(())
            })?;

            Self::deposit_event(Event::ChannelCloseInitiated { channel_id });
            Ok(())
        }

        /// Confirm channel closure (trusted relayer only).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::close_channel_confirm())]
        pub fn close_channel_confirm(origin: OriginFor<T>, channel_id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_trusted_relayer(&who)?;

            let bounded_channel_id: ChannelId<T> = channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            let now = <frame_system::Pallet<T>>::block_number();
            Channels::<T>::try_mutate(&bounded_channel_id, |maybe_channel| -> DispatchResult {
                let channel = maybe_channel.as_mut().ok_or(Error::<T>::ChannelNotFound)?;
                ensure!(
                    channel.state == ChannelState::CloseInit || channel.state == ChannelState::Open,
                    Error::<T>::ChannelAlreadyClosed
                );
                channel.state = ChannelState::Closed;
                channel.closed_at = Some(now);
                Ok(())
            })?;

            Self::deposit_event(Event::ChannelClosed { channel_id });
            Ok(())
        }

        /// Send a packet to a counterparty chain.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::send_packet())]
        pub fn send_packet(
            origin: OriginFor<T>,
            channel_id: Vec<u8>,
            dst_chain_id: Vec<u8>,
            dst_channel_id: Vec<u8>,
            dst_agent_id: Option<Vec<u8>>,
            payload: PacketPayload<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_channel_id: ChannelId<T> = channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;
            let bounded_dst_chain_id: ChainId<T> = dst_chain_id
                .try_into()
                .map_err(|_| Error::<T>::ChainIdTooLong)?;
            let bounded_dst_channel_id: ChannelId<T> = dst_channel_id
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;
            let bounded_dst_agent_id: Option<RemoteAgentId<T>> = match dst_agent_id {
                Some(id) => Some(id.try_into().map_err(|_| Error::<T>::ChannelIdTooLong)?),
                None => None,
            };

            // Verify channel is open
            let channel =
                Channels::<T>::get(&bounded_channel_id).ok_or(Error::<T>::ChannelNotFound)?;
            ensure!(
                channel.state == ChannelState::Open,
                Error::<T>::ChannelNotOpen
            );

            // Verify chain matches
            ensure!(
                channel.counterparty_chain_id == bounded_dst_chain_id,
                Error::<T>::ChannelNotFound
            );

            // Get sequence
            let sequence = SendSequences::<T>::get(&bounded_channel_id);

            // Create packet
            let timeout_height = <frame_system::Pallet<T>>::block_number()
                .saturating_add(T::PacketTimeoutBlocks::get().into());
            let created_at = <frame_system::Pallet<T>>::block_number();

            let packet = Packet::<T> {
                sequence,
                src_channel_id: bounded_channel_id.clone(),
                dst_channel_id: bounded_dst_channel_id,
                dst_chain_id: bounded_dst_chain_id,
                src_agent_id: None, // Can be extended to look up agent from account
                dst_agent_id: bounded_dst_agent_id,
                payload,
                timeout_height,
                created_at,
            };

            // Calculate commitment
            let commitment = Self::packet_commitment(&packet);

            // Check pending packet limit
            let pending_count = SendSequences::<T>::get(&bounded_channel_id)
                .saturating_sub(AckSequences::<T>::get(&bounded_channel_id));
            ensure!(
                pending_count < T::MaxPendingPackets::get() as Sequence,
                Error::<T>::PendingPacketLimitExceeded
            );

            // Store commitment
            PacketCommitments::<T>::insert(&bounded_channel_id, sequence, commitment);
            SendSequences::<T>::insert(&bounded_channel_id, sequence + 1);

            let payload_hash = sp_io::hashing::blake2_256(&packet.payload.encode());

            Self::deposit_event(Event::PacketSent {
                channel_id,
                sequence,
                src_agent: None,
                payload_hash: H256::from(payload_hash),
            });

            Ok(())
        }

        /// Receive a packet from a trusted relayer.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::receive_packet())]
        pub fn receive_packet(origin: OriginFor<T>, packet: Packet<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_trusted_relayer(&who)?;

            // Verify channel exists and is open
            let channel =
                Channels::<T>::get(&packet.dst_channel_id).ok_or(Error::<T>::ChannelNotFound)?;
            ensure!(
                channel.state == ChannelState::Open,
                Error::<T>::ChannelNotOpen
            );

            // Verify this chain is the destination
            // (In a real implementation, we'd check dst_chain_id against our chain ID)

            // Verify sequence
            let expected_seq = RecvSequences::<T>::get(&packet.dst_channel_id);
            ensure!(
                packet.sequence == expected_seq,
                Error::<T>::SequenceMismatch
            );

            // Verify no replay
            ensure!(
                !PacketReceipts::<T>::contains_key(&packet.dst_channel_id, packet.sequence),
                Error::<T>::PacketAlreadyReceived
            );

            // Verify not timed out
            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(now < packet.timeout_height, Error::<T>::PacketTimedOut);

            // Store receipt
            PacketReceipts::<T>::insert(
                &packet.dst_channel_id,
                packet.sequence,
                ReceiptStatus::Received,
            );
            RecvSequences::<T>::insert(&packet.dst_channel_id, packet.sequence + 1);

            // Handle payload
            // (In a real implementation, this would dispatch to application handlers)

            Self::deposit_event(Event::PacketReceived {
                channel_id: packet.dst_channel_id.to_vec(),
                sequence: packet.sequence,
                dst_agent: packet.dst_agent_id,
            });

            Ok(())
        }

        /// Acknowledge a packet (trusted relayer only).
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::acknowledge_packet())]
        pub fn acknowledge_packet(
            origin: OriginFor<T>,
            channel_id: Vec<u8>,
            sequence: Sequence,
            ack: PacketPayload<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_trusted_relayer(&who)?;

            let bounded_channel_id: ChannelId<T> = channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            // Verify commitment exists
            ensure!(
                PacketCommitments::<T>::contains_key(&bounded_channel_id, sequence),
                Error::<T>::PacketNotFound
            );

            // Extract success from ack payload
            let (success, _error_code) = match &ack {
                PacketPayload::Ack {
                    success,
                    error_code,
                    ..
                } => (*success, *error_code),
                _ => (false, Some(1)), // Invalid ack format
            };

            // Delete commitment
            PacketCommitments::<T>::remove(&bounded_channel_id, sequence);

            // Store acknowledgement
            PacketAcknowledgements::<T>::insert(
                &bounded_channel_id,
                sequence,
                AckStatus { success },
            );
            AckSequences::<T>::mutate(&bounded_channel_id, |seq| *seq += 1);

            Self::deposit_event(Event::PacketAcknowledged {
                channel_id,
                sequence,
                success,
            });

            Ok(())
        }

        /// Timeout an unacknowledged packet.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::timeout_packet())]
        pub fn timeout_packet(
            origin: OriginFor<T>,
            channel_id: Vec<u8>,
            sequence: Sequence,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?; // Anyone can call

            let bounded_channel_id: ChannelId<T> = channel_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            // Verify commitment exists
            ensure!(
                PacketCommitments::<T>::contains_key(&bounded_channel_id, sequence),
                Error::<T>::PacketNotFound
            );

            // Verify no receipt (can't timeout received packets)
            ensure!(
                !PacketReceipts::<T>::contains_key(&bounded_channel_id, sequence),
                Error::<T>::PacketAlreadyReceived
            );

            // Verify timeout has passed
            // (We'd need to store timeout_height to check this properly)
            // For now, we assume the caller has verified

            // Delete commitment
            PacketCommitments::<T>::remove(&bounded_channel_id, sequence);

            Self::deposit_event(Event::PacketTimeout {
                channel_id,
                sequence,
            });

            Ok(())
        }

        /// Add a trusted relayer.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::add_relayer())]
        pub fn add_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
            T::RelayerManagerOrigin::ensure_origin(origin)?;

            let mut relayers = TrustedRelayers::<T>::get();
            ensure!(
                !relayers.contains(&relayer),
                Error::<T>::RelayerAlreadyRegistered
            );
            ensure!(
                relayers.len() < T::MaxRelayers::get() as usize,
                Error::<T>::TooManyRelayers
            );

            relayers
                .try_push(relayer.clone())
                .map_err(|_| Error::<T>::TooManyRelayers)?;
            TrustedRelayers::<T>::put(relayers);

            Self::deposit_event(Event::RelayerAdded { relayer });
            Ok(())
        }

        /// Remove a trusted relayer.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::remove_relayer())]
        pub fn remove_relayer(origin: OriginFor<T>, relayer: T::AccountId) -> DispatchResult {
            T::RelayerManagerOrigin::ensure_origin(origin)?;

            let mut relayers = TrustedRelayers::<T>::get();
            let idx = relayers
                .iter()
                .position(|r| r == &relayer)
                .ok_or(Error::<T>::RelayerNotFound)?;
            relayers.remove(idx);
            TrustedRelayers::<T>::put(relayers);

            Self::deposit_event(Event::RelayerRemoved { relayer });
            Ok(())
        }

        /// Register a cross-chain agent mapping.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::register_cross_chain_agent())]
        pub fn register_cross_chain_agent(
            origin: OriginFor<T>,
            chain_id: Vec<u8>,
            remote_agent_id: Vec<u8>,
            local_agent_id: AgentId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_trusted_relayer(&who)?;

            let bounded_chain_id: ChainId<T> = chain_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChainIdTooLong)?;
            let bounded_remote_agent_id: RemoteAgentId<T> = remote_agent_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ChannelIdTooLong)?;

            // Verify local agent exists
            ensure!(
                T::AgentRegistry::agent_exists(local_agent_id),
                Error::<T>::AgentNotFound
            );

            // Check not already mapped
            ensure!(
                !CrossChainAgentMap::<T>::contains_key(&bounded_chain_id, &bounded_remote_agent_id),
                Error::<T>::CrossChainAgentAlreadyMapped
            );

            CrossChainAgentMap::<T>::insert(
                &bounded_chain_id,
                &bounded_remote_agent_id,
                local_agent_id,
            );

            Self::deposit_event(Event::CrossChainAgentRegistered {
                chain_id,
                remote_agent_id: bounded_remote_agent_id,
                local_agent_id,
            });

            Ok(())
        }
    }

    // =========================================================
    // Internal Functions
    // =========================================================

    impl<T: Config> Pallet<T> {
        /// Ensure the caller is a trusted relayer.
        fn ensure_trusted_relayer(who: &T::AccountId) -> DispatchResult {
            ensure!(
                TrustedRelayers::<T>::get().contains(who),
                Error::<T>::NotTrustedRelayer
            );
            Ok(())
        }

        /// Calculate the packet commitment hash.
        fn packet_commitment(packet: &Packet<T>) -> H256 {
            use sp_io::hashing::blake2_256;

            let payload_hash = blake2_256(&packet.payload.encode());
            let mut data = Vec::new();
            data.extend_from_slice(&packet.sequence.to_be_bytes());
            data.extend_from_slice(packet.src_channel_id.as_slice());
            data.extend_from_slice(packet.dst_channel_id.as_slice());
            data.extend_from_slice(&packet.timeout_height.to_be_bytes());
            data.extend_from_slice(&payload_hash);

            H256::from(blake2_256(&data))
        }
    }
}
