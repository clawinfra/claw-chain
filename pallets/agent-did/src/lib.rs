//! # Agent DID Pallet
//!
//! On-chain W3C DID document management for ClawChain agents.
//!
//! ## Overview
//!
//! This pallet implements the `did:claw` DID method for agent identity:
//! - DID format: `did:claw:{AccountId}`
//! - Storage: DID documents, service endpoints, verification methods
//! - Lifecycle: register → update → deactivate
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `register_did` — Create a new DID document for the caller
//! - `update_did` — Update the DID document controller or metadata
//! - `deactivate_did` — Permanently deactivate a DID document
//! - `add_service_endpoint` — Add a service endpoint to a DID document
//! - `remove_service_endpoint` — Remove a service endpoint

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    // ========== Types ==========

    /// Sequential ID for service endpoints within a DID document.
    pub type ServiceEndpointId = u32;

    // ========== Data Structures ==========

    /// Status of a DID document.
    #[derive(
        Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum DidStatus {
        /// DID document is active.
        Active,
        /// DID document has been permanently deactivated.
        Deactivated,
    }

    impl Default for DidStatus {
        fn default() -> Self {
            DidStatus::Active
        }
    }

    /// A W3C DID document stored on-chain.
    ///
    /// Represents `did:claw:{AccountId}` where the AccountId is the map key.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DidDocument<T: Config> {
        /// The account that controls this DID document.
        pub controller: T::AccountId,
        /// Optional JSON-LD context or metadata (e.g. linked service roots).
        pub metadata: BoundedVec<u8, T::MaxMetadataLength>,
        /// DID document status.
        pub status: DidStatus,
        /// Block number when the DID was registered.
        pub registered_at: BlockNumberFor<T>,
        /// Block number of the last update.
        pub updated_at: BlockNumberFor<T>,
        /// Monotonic counter for service endpoint IDs.
        pub next_service_id: ServiceEndpointId,
    }

    // Manual impl required: DecodeWithMemTracking is a marker trait for types
    // that implement Decode. The derive macro does not handle generic structs
    // with Config bounds reliably across all codec versions.
    impl<T: Config> codec::DecodeWithMemTracking for DidDocument<T> {}

    /// A service endpoint attached to a DID document.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ServiceEndpoint<T: Config> {
        /// Human-readable service type (e.g. "AgentMessaging", "RpcNode").
        pub service_type: BoundedVec<u8, T::MaxServiceTypeLength>,
        /// Service endpoint URL or multiaddr.
        pub service_url: BoundedVec<u8, T::MaxServiceUrlLength>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for ServiceEndpoint<T> {}

    // ========== Pallet ==========

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of service endpoints per DID document.
        #[pallet::constant]
        type MaxServiceEndpoints: Get<u32>;

        /// Maximum byte length of a service type string.
        #[pallet::constant]
        type MaxServiceTypeLength: Get<u32>;

        /// Maximum byte length of a service URL string.
        #[pallet::constant]
        type MaxServiceUrlLength: Get<u32>;

        /// Maximum byte length of DID document metadata.
        #[pallet::constant]
        type MaxMetadataLength: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// DID documents, keyed by the account that owns them.
    ///
    /// `did:claw:{AccountId}` ↔ DidDocument
    #[pallet::storage]
    #[pallet::getter(fn did_documents)]
    pub type DidDocuments<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, DidDocument<T>, OptionQuery>;

    /// Service endpoints attached to a DID document.
    ///
    /// (AccountId, ServiceEndpointId) → ServiceEndpoint
    #[pallet::storage]
    #[pallet::getter(fn service_endpoints)]
    pub type ServiceEndpoints<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        ServiceEndpointId,
        ServiceEndpoint<T>,
        OptionQuery,
    >;

    /// Count of active service endpoints per DID (to enforce the cap).
    #[pallet::storage]
    #[pallet::getter(fn service_endpoint_count)]
    pub type ServiceEndpointCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new DID document was registered.
        DidRegistered { who: T::AccountId },
        /// A DID document was updated.
        DidUpdated { who: T::AccountId },
        /// A DID document was deactivated.
        DidDeactivated { who: T::AccountId },
        /// A service endpoint was added to a DID document.
        ServiceEndpointAdded {
            who: T::AccountId,
            endpoint_id: ServiceEndpointId,
        },
        /// A service endpoint was removed from a DID document.
        ServiceEndpointRemoved {
            who: T::AccountId,
            endpoint_id: ServiceEndpointId,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// Caller already has a registered DID document.
        DidAlreadyRegistered,
        /// No DID document found for this account.
        DidNotFound,
        /// The DID document has been deactivated and cannot be modified.
        DidDeactivated,
        /// The service endpoint ID does not exist on this DID.
        ServiceEndpointNotFound,
        /// Maximum number of service endpoints reached.
        TooManyServiceEndpoints,
        /// Metadata exceeds the maximum allowed length.
        MetadataTooLong,
        /// Service type string exceeds the maximum allowed length.
        ServiceTypeTooLong,
        /// Service URL exceeds the maximum allowed length.
        ServiceUrlTooLong,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new DID document for the caller.
        ///
        /// The DID will be `did:claw:{caller}`. Each account may only register
        /// one DID document. The `metadata` field accepts raw bytes (e.g. JSON-LD).
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn register_did(
            origin: OriginFor<T>,
            metadata: alloc::vec::Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !DidDocuments::<T>::contains_key(&who),
                Error::<T>::DidAlreadyRegistered
            );

            let bounded_metadata: BoundedVec<u8, T::MaxMetadataLength> =
                metadata.try_into().map_err(|_| Error::<T>::MetadataTooLong)?;

            let now = frame_system::Pallet::<T>::block_number();

            let doc = DidDocument {
                controller: who.clone(),
                metadata: bounded_metadata,
                status: DidStatus::Active,
                registered_at: now,
                updated_at: now,
                next_service_id: 0,
            };

            DidDocuments::<T>::insert(&who, doc);

            Self::deposit_event(Event::DidRegistered { who });
            Ok(())
        }

        /// Update the metadata of the caller's DID document.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_did(
            origin: OriginFor<T>,
            new_metadata: alloc::vec::Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            DidDocuments::<T>::try_mutate(&who, |maybe_doc| {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DidNotFound)?;
                ensure!(doc.status == DidStatus::Active, Error::<T>::DidDeactivated);

                let bounded: BoundedVec<u8, T::MaxMetadataLength> =
                    new_metadata.try_into().map_err(|_| Error::<T>::MetadataTooLong)?;

                doc.metadata = bounded;
                doc.updated_at = frame_system::Pallet::<T>::block_number();
                Ok(())
            })?;

            Self::deposit_event(Event::DidUpdated { who });
            Ok(())
        }

        /// Permanently deactivate the caller's DID document.
        ///
        /// A deactivated DID cannot be re-activated or modified. Service
        /// endpoints may still be queried for audit purposes.
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn deactivate_did(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            DidDocuments::<T>::try_mutate(&who, |maybe_doc| {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DidNotFound)?;
                ensure!(doc.status == DidStatus::Active, Error::<T>::DidDeactivated);

                doc.status = DidStatus::Deactivated;
                doc.updated_at = frame_system::Pallet::<T>::block_number();
                Ok(())
            })?;

            Self::deposit_event(Event::DidDeactivated { who });
            Ok(())
        }

        /// Add a service endpoint to the caller's DID document.
        ///
        /// Returns the new endpoint's `ServiceEndpointId`.
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
        pub fn add_service_endpoint(
            origin: OriginFor<T>,
            service_type: alloc::vec::Vec<u8>,
            service_url: alloc::vec::Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let endpoint_id = DidDocuments::<T>::try_mutate(&who, |maybe_doc| {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DidNotFound)?;
                ensure!(doc.status == DidStatus::Active, Error::<T>::DidDeactivated);

                let count = ServiceEndpointCount::<T>::get(&who);
                ensure!(
                    count < T::MaxServiceEndpoints::get(),
                    Error::<T>::TooManyServiceEndpoints
                );

                let id = doc.next_service_id;
                doc.next_service_id = id.saturating_add(1);
                doc.updated_at = frame_system::Pallet::<T>::block_number();

                Ok::<ServiceEndpointId, DispatchError>(id)
            })?;

            let bounded_type: BoundedVec<u8, T::MaxServiceTypeLength> =
                service_type.try_into().map_err(|_| Error::<T>::ServiceTypeTooLong)?;

            let bounded_url: BoundedVec<u8, T::MaxServiceUrlLength> =
                service_url.try_into().map_err(|_| Error::<T>::ServiceUrlTooLong)?;

            let endpoint = ServiceEndpoint {
                service_type: bounded_type,
                service_url: bounded_url,
            };

            ServiceEndpoints::<T>::insert(&who, endpoint_id, endpoint);
            ServiceEndpointCount::<T>::mutate(&who, |c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ServiceEndpointAdded {
                who,
                endpoint_id,
            });
            Ok(())
        }

        /// Remove a service endpoint from the caller's DID document.
        #[pallet::call_index(4)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_service_endpoint(
            origin: OriginFor<T>,
            endpoint_id: ServiceEndpointId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify DID exists and is active.
            let doc = DidDocuments::<T>::get(&who).ok_or(Error::<T>::DidNotFound)?;
            ensure!(doc.status == DidStatus::Active, Error::<T>::DidDeactivated);

            // Verify endpoint exists.
            ensure!(
                ServiceEndpoints::<T>::contains_key(&who, endpoint_id),
                Error::<T>::ServiceEndpointNotFound
            );

            ServiceEndpoints::<T>::remove(&who, endpoint_id);
            ServiceEndpointCount::<T>::mutate(&who, |c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ServiceEndpointRemoved {
                who,
                endpoint_id,
            });
            Ok(())
        }
    }

    // ========== Weight Trait ==========

    /// Weight functions for pallet extrinsics.
    pub trait WeightInfo {
        fn register_did() -> Weight;
        fn update_did() -> Weight;
        fn deactivate_did() -> Weight;
        fn add_service_endpoint() -> Weight;
        fn remove_service_endpoint() -> Weight;
    }

    /// Default no-op weights (for testing / benchmarking later).
    impl WeightInfo for () {
        fn register_did() -> Weight { Weight::zero() }
        fn update_did() -> Weight { Weight::zero() }
        fn deactivate_did() -> Weight { Weight::zero() }
        fn add_service_endpoint() -> Weight { Weight::zero() }
        fn remove_service_endpoint() -> Weight { Weight::zero() }
    }
}
