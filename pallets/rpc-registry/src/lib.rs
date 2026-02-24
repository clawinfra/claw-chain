//! # RPC Registry Pallet
//!
//! The ClawChain pallet for on-chain RPC endpoint discovery.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Registering RPC endpoint URLs on-chain
//! - Storing node metadata (region, type, capabilities)
//! - Tracking node health via periodic heartbeats
//! - Automated discovery of available RPC endpoints by EvoClaw agents
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `register_node` - Register a new RPC endpoint with metadata
//! - `update_node` - Update an existing node's URL and region
//! - `heartbeat` - Prove that a node is still alive
//! - `deregister_node` - Remove an RPC endpoint from the registry
//! - `report_inactive` - Mark a node as inactive if heartbeat expired

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
    use sp_runtime::traits::Saturating;

    /// Type alias for RPC node IDs (sequential u64).
    pub type RpcNodeId = u64;

    /// Node type enum.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum NodeType {
        /// Full node with complete chain state.
        FullNode,
        /// Validator node (may also be full node).
        Validator,
        /// Light node with limited state.
        LightNode,
        /// Archive node with full historical state.
        ArchiveNode,
    }

    impl Default for NodeType {
        fn default() -> Self {
            NodeType::FullNode
        }
    }

    /// Node status enum.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum NodeStatus {
        /// Node is active and healthy.
        Active,
        /// Node has not sent a heartbeat in MaxHeartbeatInterval blocks.
        Inactive,
        /// Node has been deregistered by the owner.
        Deregistered,
    }

    impl Default for NodeStatus {
        fn default() -> Self {
            NodeStatus::Active
        }
    }

    /// Core RPC node information stored on-chain.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct RpcNodeInfo<T: Config> {
        /// The account that owns this RPC node.
        pub owner: T::AccountId,
        /// RPC endpoint URL (e.g., "wss://rpc1.clawchain.win")
        pub url: BoundedVec<u8, T::MaxUrlLength>,
        /// Geographic region hint (e.g., "eu-west", "ap-southeast", "us-east")
        pub region: BoundedVec<u8, T::MaxRegionLength>,
        /// Node type: FullNode, Validator, LightNode, ArchiveNode
        pub node_type: NodeType,
        /// Whether the node supports WebSocket
        pub supports_ws: bool,
        /// Whether the node supports HTTP
        pub supports_http: bool,
        /// Last heartbeat block number (proves node is alive)
        pub last_heartbeat: BlockNumberFor<T>,
        /// Block when registered
        pub registered_at: BlockNumberFor<T>,
        /// Status: Active, Inactive, Deregistered
        pub status: NodeStatus,
    }

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum length of a URL in bytes.
        #[pallet::constant]
        type MaxUrlLength: Get<u32>;

        /// Maximum length of a region identifier in bytes.
        #[pallet::constant]
        type MaxRegionLength: Get<u32>;

        /// Maximum number of nodes a single account can own.
        #[pallet::constant]
        type MaxNodesPerOwner: Get<u32>;

        /// Maximum number of active nodes tracked in the ActiveNodes list.
        #[pallet::constant]
        type MaxActiveNodes: Get<u32>;

        /// Maximum heartbeat interval in blocks before a node is considered inactive.
        #[pallet::constant]
        type MaxHeartbeatInterval: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map from RpcNodeId to RpcNodeInfo.
    #[pallet::storage]
    #[pallet::getter(fn rpc_nodes)]
    pub type RpcNodes<T: Config> =
        StorageMap<_, Blake2_128Concat, RpcNodeId, RpcNodeInfo<T>, OptionQuery>;

    /// Total number of registered RPC nodes (including deregistered).
    #[pallet::storage]
    #[pallet::getter(fn node_count)]
    pub type NodeCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Map from owner AccountId to their list of node IDs.
    #[pallet::storage]
    #[pallet::getter(fn owner_nodes)]
    pub type OwnerNodes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<RpcNodeId, T::MaxNodesPerOwner>,
        ValueQuery,
    >;

    /// Active nodes (sorted by last heartbeat, for efficient querying).
    #[pallet::storage]
    #[pallet::getter(fn active_nodes)]
    pub type ActiveNodes<T: Config> =
        StorageValue<_, BoundedVec<RpcNodeId, T::MaxActiveNodes>, ValueQuery>;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new RPC node was registered.
        NodeRegistered {
            node_id: RpcNodeId,
            owner: T::AccountId,
            url: Vec<u8>,
            region: Vec<u8>,
        },
        /// An RPC node's info was updated.
        NodeUpdated { node_id: RpcNodeId, url: Vec<u8> },
        /// An RPC node sent a heartbeat.
        Heartbeat {
            node_id: RpcNodeId,
            block: BlockNumberFor<T>,
        },
        /// An RPC node was deregistered.
        NodeDeregistered { node_id: RpcNodeId },
        /// An RPC node was marked as inactive.
        NodeInactive {
            node_id: RpcNodeId,
            last_heartbeat: BlockNumberFor<T>,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// The node ID was not found in the registry.
        NodeNotFound,
        /// Only the node owner can perform this action.
        NotNodeOwner,
        /// The URL exceeds the maximum allowed length.
        UrlTooLong,
        /// The region identifier exceeds the maximum allowed length.
        RegionTooLong,
        /// The account has reached the maximum number of nodes.
        TooManyNodes,
        /// The node has already been deregistered.
        NodeAlreadyDeregistered,
        /// Heartbeat was sent too recently (anti-spam).
        HeartbeatTooRecent,
        /// The node is still active (recent heartbeat) and cannot be reported as inactive.
        NodeStillActive,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new RPC node on-chain.
        ///
        /// The caller becomes the owner of the node. The node starts with
        /// Active status and the current block as the last heartbeat.
        ///
        /// # Arguments
        /// * `url` - RPC endpoint URL (e.g., "wss://rpc1.clawchain.win")
        /// * `region` - Geographic region hint (e.g., "eu-west")
        /// * `node_type` - Type of node (FullNode, Validator, etc.)
        /// * `supports_ws` - Whether the node supports WebSocket
        /// * `supports_http` - Whether the node supports HTTP
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 4))]
        pub fn register_node(
            origin: OriginFor<T>,
            url: Vec<u8>,
            region: Vec<u8>,
            node_type: NodeType,
            supports_ws: bool,
            supports_http: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_url: BoundedVec<u8, T::MaxUrlLength> =
                url.clone().try_into().map_err(|_| Error::<T>::UrlTooLong)?;
            let bounded_region: BoundedVec<u8, T::MaxRegionLength> = region
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::RegionTooLong)?;

            let node_id = NodeCount::<T>::get();
            let current_block = <frame_system::Pallet<T>>::block_number();

            let node_info = RpcNodeInfo::<T> {
                owner: who.clone(),
                url: bounded_url,
                region: bounded_region,
                node_type,
                supports_ws,
                supports_http,
                last_heartbeat: current_block,
                registered_at: current_block,
                status: NodeStatus::Active,
            };

            // Store the node
            RpcNodes::<T>::insert(node_id, node_info);

            // Update node count
            NodeCount::<T>::put(node_id.saturating_add(1));

            // Add to owner's node list
            OwnerNodes::<T>::try_mutate(&who, |nodes| {
                nodes
                    .try_push(node_id)
                    .map_err(|_| Error::<T>::TooManyNodes)
            })?;

            // Add to active nodes list (best effort, ignore if full)
            ActiveNodes::<T>::try_mutate(|active| {
                let _ = active.try_push(node_id);
                Ok::<(), Error<T>>(())
            })?;

            Self::deposit_event(Event::NodeRegistered {
                node_id,
                owner: who,
                url,
                region,
            });

            Ok(())
        }

        /// Update an RPC node's URL and region.
        ///
        /// Only the node owner can update the node info.
        ///
        /// # Arguments
        /// * `node_id` - The ID of the node to update
        /// * `url` - New RPC endpoint URL
        /// * `region` - New geographic region hint
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_node(
            origin: OriginFor<T>,
            node_id: RpcNodeId,
            url: Vec<u8>,
            region: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            RpcNodes::<T>::try_mutate(node_id, |maybe_node| -> DispatchResult {
                let node = maybe_node.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == who, Error::<T>::NotNodeOwner);
                ensure!(
                    node.status != NodeStatus::Deregistered,
                    Error::<T>::NodeAlreadyDeregistered
                );

                let bounded_url: BoundedVec<u8, T::MaxUrlLength> =
                    url.clone().try_into().map_err(|_| Error::<T>::UrlTooLong)?;
                let bounded_region: BoundedVec<u8, T::MaxRegionLength> =
                    region.try_into().map_err(|_| Error::<T>::RegionTooLong)?;

                node.url = bounded_url;
                node.region = bounded_region;

                Ok(())
            })?;

            Self::deposit_event(Event::NodeUpdated { node_id, url });

            Ok(())
        }

        /// Send a heartbeat to prove the node is still alive.
        ///
        /// Only the node owner can send heartbeats. Updates the last_heartbeat
        /// to the current block number and ensures the node is marked as Active.
        ///
        /// # Arguments
        /// * `node_id` - The ID of the node sending the heartbeat
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn heartbeat(origin: OriginFor<T>, node_id: RpcNodeId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            RpcNodes::<T>::try_mutate(node_id, |maybe_node| -> DispatchResult {
                let node = maybe_node.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == who, Error::<T>::NotNodeOwner);
                ensure!(
                    node.status != NodeStatus::Deregistered,
                    Error::<T>::NodeAlreadyDeregistered
                );

                let current_block = <frame_system::Pallet<T>>::block_number();
                node.last_heartbeat = current_block;

                // If the node was inactive, mark it as active again
                if node.status == NodeStatus::Inactive {
                    node.status = NodeStatus::Active;
                }

                Self::deposit_event(Event::Heartbeat {
                    node_id,
                    block: current_block,
                });

                Ok(())
            })
        }

        /// Deregister an RPC node.
        ///
        /// Only the node owner can deregister. Sets the status to Deregistered.
        /// The node data remains on-chain for historical purposes.
        ///
        /// # Arguments
        /// * `node_id` - The ID of the node to deregister
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn deregister_node(origin: OriginFor<T>, node_id: RpcNodeId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            RpcNodes::<T>::try_mutate(node_id, |maybe_node| -> DispatchResult {
                let node = maybe_node.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(node.owner == who, Error::<T>::NotNodeOwner);
                ensure!(
                    node.status != NodeStatus::Deregistered,
                    Error::<T>::NodeAlreadyDeregistered
                );

                node.status = NodeStatus::Deregistered;

                Ok(())
            })?;

            // Remove from active nodes list
            ActiveNodes::<T>::mutate(|active| {
                if let Some(pos) = active.iter().position(|id| *id == node_id) {
                    active.remove(pos);
                }
            });

            Self::deposit_event(Event::NodeDeregistered { node_id });

            Ok(())
        }

        /// Report a node as inactive if it hasn't sent a heartbeat recently.
        ///
        /// Anyone can call this function. If the node's last heartbeat is older
        /// than MaxHeartbeatInterval, the node is marked as Inactive.
        ///
        /// # Arguments
        /// * `node_id` - The ID of the node to report
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn report_inactive(origin: OriginFor<T>, node_id: RpcNodeId) -> DispatchResult {
            ensure_signed(origin)?;

            RpcNodes::<T>::try_mutate(node_id, |maybe_node| -> DispatchResult {
                let node = maybe_node.as_mut().ok_or(Error::<T>::NodeNotFound)?;
                ensure!(
                    node.status != NodeStatus::Deregistered,
                    Error::<T>::NodeAlreadyDeregistered
                );

                let current_block = <frame_system::Pallet<T>>::block_number();
                let max_interval: BlockNumberFor<T> = T::MaxHeartbeatInterval::get().into();

                // Check if the heartbeat is too old
                let blocks_since_heartbeat = current_block.saturating_sub(node.last_heartbeat);
                ensure!(
                    blocks_since_heartbeat >= max_interval,
                    Error::<T>::NodeStillActive
                );

                node.status = NodeStatus::Inactive;

                Ok(())
            })?;

            // Remove from active nodes list
            ActiveNodes::<T>::mutate(|active| {
                if let Some(pos) = active.iter().position(|id| *id == node_id) {
                    active.remove(pos);
                }
            });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    /// Weight information for the pallet's extrinsics.
    pub trait WeightInfo {
        fn register_node() -> Weight;
        fn update_node() -> Weight;
        fn heartbeat() -> Weight;
        fn deregister_node() -> Weight;
        fn report_inactive() -> Weight;
    }

    /// Default weights for testing.
    impl WeightInfo for () {
        fn register_node() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn update_node() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn heartbeat() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn deregister_node() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn report_inactive() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
