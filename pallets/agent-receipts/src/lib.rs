//! # Agent Receipts Pallet (ProvenanceChain)
//!
//! Verifiable on-chain receipts for AI agent activity attestation.
//!
//! ## Overview
//!
//! Every time an EvoClaw AI agent takes an action, it can emit an on-chain receipt
//! containing: agent_id, action_type, input_hash, output_hash, metadata, and
//! block_number. This makes autonomous agent decisions auditable and verifiable
//! by anyone — creating a cryptographic provenance trail.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `submit_receipt` - Submit a new activity receipt for an agent
//! - `clear_old_receipts` - Prune old receipts before a given nonce

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;

    /// Bounded agent identifier type.
    pub type AgentIdOf<T> = BoundedVec<u8, <T as Config>::MaxAgentIdLen>;

    /// A verifiable receipt of an AI agent action.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct AgentReceipt<T: Config> {
        /// The agent that performed this action.
        pub agent_id: AgentIdOf<T>,
        /// The type of action (e.g. "trade", "message", "tool_call").
        pub action_type: BoundedVec<u8, T::MaxActionTypeLen>,
        /// SHA-256 hash of the action inputs.
        pub input_hash: H256,
        /// SHA-256 hash of the action outputs.
        pub output_hash: H256,
        /// Optional JSON metadata / context.
        pub metadata: BoundedVec<u8, T::MaxMetadataLen>,
        /// Block number when this receipt was recorded.
        pub block_number: BlockNumberFor<T>,
        /// Timestamp (milliseconds since UNIX epoch) — informational, set by caller.
        pub timestamp: u64,
    }

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum length of an agent ID in bytes.
        #[pallet::constant]
        type MaxAgentIdLen: Get<u32>;

        /// Maximum length of an action type string in bytes.
        #[pallet::constant]
        type MaxActionTypeLen: Get<u32>;

        /// Maximum length of receipt metadata in bytes.
        #[pallet::constant]
        type MaxMetadataLen: Get<u32>;

        /// Maximum number of receipts to clear in a single call (DoS protection).
        #[pallet::constant]
        type MaxClearBatchSize: Get<u64>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map from (AgentId, nonce) to AgentReceipt.
    #[pallet::storage]
    #[pallet::getter(fn receipts)]
    pub type Receipts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AgentIdOf<T>,
        Blake2_128Concat,
        u64,
        AgentReceipt<T>,
        OptionQuery,
    >;

    /// Per-agent nonce — the next receipt index for a given agent.
    #[pallet::storage]
    #[pallet::getter(fn agent_nonce)]
    pub type AgentNonce<T: Config> = StorageMap<_, Blake2_128Concat, AgentIdOf<T>, u64, ValueQuery>;

    /// Global receipt counter (total receipts ever submitted).
    #[pallet::storage]
    #[pallet::getter(fn receipt_count)]
    pub type ReceiptCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new receipt was submitted on-chain.
        ReceiptSubmitted {
            agent_id: Vec<u8>,
            nonce: u64,
            action_type: Vec<u8>,
            block_number: BlockNumberFor<T>,
        },
        /// Old receipts were cleared for an agent.
        ReceiptsCleared { agent_id: Vec<u8>, count: u64 },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// The agent ID exceeds the maximum allowed length.
        AgentIdTooLong,
        /// The action type exceeds the maximum allowed length.
        ActionTypeTooLong,
        /// The metadata exceeds the maximum allowed length.
        MetadataTooLong,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a receipt attesting to an agent action.
        ///
        /// Stores the receipt on-chain, increments the agent's nonce, and
        /// increments the global receipt counter.
        ///
        /// # Arguments
        /// * `agent_id` - Identifier of the agent that performed the action
        /// * `action_type` - Short label for the action (e.g. "trade", "tool_call")
        /// * `input_hash` - H256 hash of the action's inputs
        /// * `output_hash` - H256 hash of the action's outputs
        /// * `metadata` - Optional JSON context
        /// * `timestamp` - Caller-provided UNIX timestamp (ms)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 3))]
        pub fn submit_receipt(
            origin: OriginFor<T>,
            agent_id: Vec<u8>,
            action_type: Vec<u8>,
            input_hash: H256,
            output_hash: H256,
            metadata: Vec<u8>,
            timestamp: u64,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let bounded_agent_id: AgentIdOf<T> = agent_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::AgentIdTooLong)?;
            let bounded_action_type: BoundedVec<u8, T::MaxActionTypeLen> = action_type
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ActionTypeTooLong)?;
            let bounded_metadata: BoundedVec<u8, T::MaxMetadataLen> = metadata
                .try_into()
                .map_err(|_| Error::<T>::MetadataTooLong)?;

            let current_block = <frame_system::Pallet<T>>::block_number();
            let nonce = AgentNonce::<T>::get(&bounded_agent_id);

            let receipt = AgentReceipt::<T> {
                agent_id: bounded_agent_id.clone(),
                action_type: bounded_action_type,
                input_hash,
                output_hash,
                metadata: bounded_metadata,
                block_number: current_block,
                timestamp,
            };

            // Store the receipt
            Receipts::<T>::insert(&bounded_agent_id, nonce, receipt);

            // Increment per-agent nonce
            AgentNonce::<T>::insert(&bounded_agent_id, nonce.saturating_add(1));

            // Increment global counter
            ReceiptCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ReceiptSubmitted {
                agent_id,
                nonce,
                action_type,
                block_number: current_block,
            });

            Ok(())
        }

        /// Clear (prune) old receipts for an agent up to a given nonce.
        ///
        /// Removes all receipts with nonce < `before_nonce`. Any signed caller
        /// can invoke this as a public-good pruning helper.
        ///
        /// # Arguments
        /// * `agent_id` - The agent whose receipts to prune
        /// * `before_nonce` - Remove all receipts with nonce strictly less than this
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn clear_old_receipts(
            origin: OriginFor<T>,
            agent_id: Vec<u8>,
            before_nonce: u64,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let bounded_agent_id: AgentIdOf<T> = agent_id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::AgentIdTooLong)?;

            let max_batch = T::MaxClearBatchSize::get();
            let effective_range = before_nonce.min(max_batch);

            let mut cleared: u64 = 0;
            for nonce in 0..effective_range {
                if Receipts::<T>::contains_key(&bounded_agent_id, nonce) {
                    Receipts::<T>::remove(&bounded_agent_id, nonce);
                    cleared = cleared.saturating_add(1);
                }
            }

            Self::deposit_event(Event::ReceiptsCleared {
                agent_id,
                count: cleared,
            });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    /// Weight information for the pallet's extrinsics.
    pub trait WeightInfo {
        fn submit_receipt() -> Weight;
        fn clear_old_receipts() -> Weight;
    }

    /// Default weights for testing.
    impl WeightInfo for () {
        fn submit_receipt() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn clear_old_receipts() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
