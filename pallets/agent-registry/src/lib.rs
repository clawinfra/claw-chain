//! # Agent Registry Pallet
//!
//! The core ClawChain pallet for managing agent identities on-chain.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Registering autonomous agents with decentralized identifiers (DIDs)
//! - Storing agent metadata (name, type, capabilities)
//! - Tracking agent reputation scores (0-10000 basis points)
//! - Managing agent lifecycle (Active, Suspended, Deregistered)
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `register_agent` - Register a new agent with a DID and metadata
//! - `update_metadata` - Update an agent's metadata
//! - `update_reputation` - Adjust an agent's reputation score
//! - `deregister_agent` - Remove an agent from the registry
//! - `set_agent_status` - Change an agent's status

#![cfg_attr(not(feature = "std"), no_std)]

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

    /// Type alias for agent IDs (sequential u64).
    pub type AgentId = u64;

    /// Agent status enum.
    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum AgentStatus {
        /// Agent is active and operational.
        Active,
        /// Agent has been suspended (e.g., for misbehaviour).
        Suspended,
        /// Agent has been deregistered by the owner.
        Deregistered,
    }

    impl Default for AgentStatus {
        fn default() -> Self {
            AgentStatus::Active
        }
    }

    /// Core agent information stored on-chain.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct AgentInfo<T: Config> {
        /// The account that owns this agent.
        pub owner: T::AccountId,
        /// Decentralized identifier (DID) for the agent.
        pub did: BoundedVec<u8, T::MaxDidLength>,
        /// JSON metadata (name, type, capabilities, etc.).
        pub metadata: BoundedVec<u8, T::MaxMetadataLength>,
        /// Reputation score in basis points (0-10000).
        pub reputation: u32,
        /// Block number when the agent was registered.
        pub registered_at: BlockNumberFor<T>,
        /// Block number of the agent's last activity.
        pub last_active: BlockNumberFor<T>,
        /// Current status of the agent.
        pub status: AgentStatus,
    }

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum length of a DID in bytes.
        #[pallet::constant]
        type MaxDidLength: Get<u32>;

        /// Maximum length of metadata in bytes.
        #[pallet::constant]
        type MaxMetadataLength: Get<u32>;

        /// Maximum number of agents a single account can own.
        #[pallet::constant]
        type MaxAgentsPerOwner: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map from AgentId to AgentInfo.
    #[pallet::storage]
    #[pallet::getter(fn agent_registry)]
    pub type AgentRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, AgentId, AgentInfo<T>, OptionQuery>;

    /// Total number of registered agents (including deregistered).
    #[pallet::storage]
    #[pallet::getter(fn agent_count)]
    pub type AgentCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Map from owner AccountId to their list of agent IDs.
    #[pallet::storage]
    #[pallet::getter(fn owner_agents)]
    pub type OwnerAgents<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<AgentId, T::MaxAgentsPerOwner>,
        ValueQuery,
    >;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new agent was registered.
        AgentRegistered {
            agent_id: AgentId,
            owner: T::AccountId,
            did: Vec<u8>,
        },
        /// An agent's metadata was updated.
        AgentUpdated {
            agent_id: AgentId,
            metadata: Vec<u8>,
        },
        /// An agent's reputation score changed.
        ReputationChanged {
            agent_id: AgentId,
            old_score: u32,
            new_score: u32,
        },
        /// An agent was deregistered.
        AgentDeregistered { agent_id: AgentId },
        /// An agent's status was changed.
        AgentStatusChanged {
            agent_id: AgentId,
            status: AgentStatus,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// The agent ID was not found in the registry.
        AgentNotFound,
        /// Only the agent owner can perform this action.
        NotAgentOwner,
        /// The DID exceeds the maximum allowed length.
        DidTooLong,
        /// The metadata exceeds the maximum allowed length.
        MetadataTooLong,
        /// The account has reached the maximum number of agents.
        TooManyAgents,
        /// The agent has already been deregistered.
        AgentAlreadyDeregistered,
        /// Reputation score would overflow (max 10000).
        ReputationOverflow,
        /// Reputation score would underflow (min 0).
        ReputationUnderflow,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new agent on-chain.
        ///
        /// The caller becomes the owner of the agent. The agent starts with
        /// a reputation score of 5000 (50%) and Active status.
        ///
        /// # Arguments
        /// * `did` - Decentralized identifier for the agent
        /// * `metadata` - JSON metadata (name, type, capabilities)
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 3))]
        pub fn register_agent(
            origin: OriginFor<T>,
            did: Vec<u8>,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_did: BoundedVec<u8, T::MaxDidLength> =
                did.clone().try_into().map_err(|_| Error::<T>::DidTooLong)?;
            let bounded_metadata: BoundedVec<u8, T::MaxMetadataLength> =
                metadata.try_into().map_err(|_| Error::<T>::MetadataTooLong)?;

            let agent_id = AgentCount::<T>::get();
            let current_block = <frame_system::Pallet<T>>::block_number();

            let agent_info = AgentInfo::<T> {
                owner: who.clone(),
                did: bounded_did,
                metadata: bounded_metadata,
                reputation: 5000, // Start at 50%
                registered_at: current_block,
                last_active: current_block,
                status: AgentStatus::Active,
            };

            // Store the agent
            AgentRegistry::<T>::insert(agent_id, agent_info);

            // Update agent count
            AgentCount::<T>::put(agent_id.saturating_add(1));

            // Add to owner's agent list
            OwnerAgents::<T>::try_mutate(&who, |agents| {
                agents
                    .try_push(agent_id)
                    .map_err(|_| Error::<T>::TooManyAgents)
            })?;

            Self::deposit_event(Event::AgentRegistered {
                agent_id,
                owner: who,
                did,
            });

            Ok(())
        }

        /// Update an agent's metadata.
        ///
        /// Only the agent owner can update the metadata.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_metadata(
            origin: OriginFor<T>,
            agent_id: AgentId,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            AgentRegistry::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.owner == who, Error::<T>::NotAgentOwner);
                ensure!(
                    agent.status != AgentStatus::Deregistered,
                    Error::<T>::AgentAlreadyDeregistered
                );

                let bounded_metadata: BoundedVec<u8, T::MaxMetadataLength> =
                    metadata.clone().try_into().map_err(|_| Error::<T>::MetadataTooLong)?;

                agent.metadata = bounded_metadata;
                agent.last_active = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::AgentUpdated {
                agent_id,
                metadata,
            });

            Ok(())
        }

        /// Update an agent's reputation score.
        ///
        /// Can be called by anyone (in production, this would be restricted to
        /// a reputation oracle or governance). The delta is applied to the current
        /// score, clamped to 0-10000.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_reputation(
            origin: OriginFor<T>,
            agent_id: AgentId,
            delta: i32,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            AgentRegistry::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(
                    agent.status != AgentStatus::Deregistered,
                    Error::<T>::AgentAlreadyDeregistered
                );

                let old_score = agent.reputation;
                let new_score = if delta >= 0 {
                    old_score.saturating_add(delta as u32).min(10000)
                } else {
                    old_score.saturating_sub(delta.unsigned_abs())
                };
                agent.reputation = new_score;
                agent.last_active = <frame_system::Pallet<T>>::block_number();

                Self::deposit_event(Event::ReputationChanged {
                    agent_id,
                    old_score,
                    new_score,
                });

                Ok(())
            })
        }

        /// Deregister an agent.
        ///
        /// Only the agent owner can deregister. Sets the status to Deregistered.
        /// The agent data remains on-chain for historical purposes.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn deregister_agent(origin: OriginFor<T>, agent_id: AgentId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            AgentRegistry::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.owner == who, Error::<T>::NotAgentOwner);
                ensure!(
                    agent.status != AgentStatus::Deregistered,
                    Error::<T>::AgentAlreadyDeregistered
                );

                agent.status = AgentStatus::Deregistered;
                agent.last_active = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::AgentDeregistered { agent_id });

            Ok(())
        }

        /// Set an agent's status.
        ///
        /// Only the agent owner can change the status.
        /// Cannot change status of a deregistered agent.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_agent_status(
            origin: OriginFor<T>,
            agent_id: AgentId,
            status: AgentStatus,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            AgentRegistry::<T>::try_mutate(agent_id, |maybe_agent| -> DispatchResult {
                let agent = maybe_agent.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                ensure!(agent.owner == who, Error::<T>::NotAgentOwner);
                ensure!(
                    agent.status != AgentStatus::Deregistered,
                    Error::<T>::AgentAlreadyDeregistered
                );

                agent.status = status.clone();
                agent.last_active = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::AgentStatusChanged { agent_id, status });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    /// Weight information for the pallet's extrinsics.
    pub trait WeightInfo {
        fn register_agent() -> Weight;
        fn update_metadata() -> Weight;
        fn update_reputation() -> Weight;
        fn deregister_agent() -> Weight;
        fn set_agent_status() -> Weight;
    }

    /// Default weights for testing.
    impl WeightInfo for () {
        fn register_agent() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn update_metadata() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn update_reputation() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn deregister_agent() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn set_agent_status() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
