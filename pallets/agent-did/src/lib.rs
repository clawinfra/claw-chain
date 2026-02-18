//! # Agent DID Pallet
//!
//! W3C-compliant Decentralized Identifier (DID) system for ClawChain agents.
//!
//! ## Overview
//!
//! This pallet implements the W3C DID Core specification for on-chain agent identity:
//! - DID format: `did:claw:{AccountId}`
//! - DID documents with verification methods and service endpoints
//! - Full lifecycle management (register, update, deactivate)
//!
//! ## DID Document Structure
//!
//! Each DID document contains:
//! - Controller account (the AccountId that owns the DID)
//! - Verification methods (public keys for authentication/assertion)
//! - Service endpoints (URLs for interacting with the agent)
//! - Lifecycle metadata (created, updated, deactivated)
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `register_did` - Register a new W3C DID document on-chain
//! - `update_did` - Update the DID document's context/metadata
//! - `deactivate_did` - Deactivate a DID (irreversible)
//! - `add_service_endpoint` - Add a service endpoint to a DID document
//! - `remove_service_endpoint` - Remove a service endpoint from a DID document

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

    // ========== Types ==========

    /// A service endpoint in a DID document (W3C DID Core §5.4).
    ///
    /// Each service endpoint describes a way to interact with the DID subject.
    /// The `DecodeWithMemTracking` trait is implemented manually because it is a
    /// pure marker trait (`pub trait DecodeWithMemTracking: Decode {}`) with no
    /// derive macro available for structs.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ServiceEndpoint<T: Config> {
        /// Unique fragment identifier within the DID document (e.g. `#rpc`, `#storage`).
        pub id: BoundedVec<u8, T::MaxServiceIdLength>,
        /// Service type string (e.g. `JsonRpcService`, `LinkedDomains`).
        pub service_type: BoundedVec<u8, T::MaxServiceTypeLength>,
        /// The endpoint URI or URL.
        pub endpoint: BoundedVec<u8, T::MaxEndpointLength>,
    }

    /// Manual impl of `DecodeWithMemTracking` for `ServiceEndpoint`.
    ///
    /// `DecodeWithMemTracking` is a pure marker trait. No derive macro exists for structs;
    /// we implement it manually as an empty impl.
    impl<T: Config> codec::DecodeWithMemTracking for ServiceEndpoint<T> {}

    /// A verification method in a DID document (W3C DID Core §5.2).
    ///
    /// Verification methods represent cryptographic keys used for authentication,
    /// assertion, key agreement, etc.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct VerificationMethod<T: Config> {
        /// Unique fragment identifier within the DID document (e.g. `#key-1`).
        pub id: BoundedVec<u8, T::MaxKeyIdLength>,
        /// Key type string (e.g. `Ed25519VerificationKey2020`, `Sr25519VerificationKey2020`).
        pub key_type: BoundedVec<u8, T::MaxKeyTypeLength>,
        /// Public key encoded in multibase format (e.g. `z6Mk...`).
        pub public_key_multibase: BoundedVec<u8, T::MaxKeyLength>,
    }

    /// Manual impl of `DecodeWithMemTracking` for `VerificationMethod`.
    impl<T: Config> codec::DecodeWithMemTracking for VerificationMethod<T> {}

    /// Core DID document stored on-chain (W3C DID Core §6).
    ///
    /// The DID itself is derived deterministically: `did:claw:{controller}`.
    /// No need to store the DID string—it is computed from the controller AccountId.
    ///
    /// Service endpoints and verification methods are stored separately in
    /// `ServiceEndpoints` and `VerificationMethods` storage maps to avoid
    /// nested-generic derive issues with Rust's conservative bound propagation.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DIDDocument<T: Config> {
        /// The account that controls this DID.
        pub controller: T::AccountId,
        /// Optional JSON-LD context or additional metadata (bytes).
        pub context: BoundedVec<u8, T::MaxContextLength>,
        /// Block number when the DID was first registered.
        pub created: BlockNumberFor<T>,
        /// Block number of the most recent update.
        pub updated: BlockNumberFor<T>,
        /// Whether this DID has been deactivated. Deactivation is irreversible.
        pub deactivated: bool,
        /// Cached count of service endpoints (enforces MaxServiceEndpoints limit).
        pub service_endpoint_count: u32,
        /// Cached count of verification methods (enforces MaxVerificationMethods limit).
        pub verification_method_count: u32,
    }

    /// Manual impl of `DecodeWithMemTracking` for `DIDDocument`.
    impl<T: Config> codec::DecodeWithMemTracking for DIDDocument<T> {}

    // ========== Config ==========

    /// The pallet's configuration trait.
    ///
    /// # Note on bound type parameters
    ///
    /// The `MaxServiceIdLength`, `MaxServiceTypeLength`, etc. associated types carry
    /// `Clone + PartialEq` bounds in addition to `Get<u32>`. This is required because
    /// Rust's `#[derive]` macros propagate bounds to ALL type arguments of field types —
    /// including the bound-type parameter `S` in `BoundedVec<T, S>`. Since `ConstU32<N>`
    /// (the typical runtime value) implements `Clone + PartialEq`, this does not restrict
    /// practical usage.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum byte length of the DID context field.
        #[pallet::constant]
        type MaxContextLength: Get<u32> + Clone + PartialEq;

        /// Maximum byte length of a service endpoint `id` fragment.
        #[pallet::constant]
        type MaxServiceIdLength: Get<u32> + Clone + PartialEq;

        /// Maximum byte length of a service endpoint `type` string.
        #[pallet::constant]
        type MaxServiceTypeLength: Get<u32> + Clone + PartialEq;

        /// Maximum byte length of a service endpoint URL.
        #[pallet::constant]
        type MaxEndpointLength: Get<u32> + Clone + PartialEq;

        /// Maximum number of service endpoints per DID document.
        #[pallet::constant]
        type MaxServiceEndpoints: Get<u32>;

        /// Maximum byte length of a verification method `id` fragment.
        #[pallet::constant]
        type MaxKeyIdLength: Get<u32> + Clone + PartialEq;

        /// Maximum byte length of a verification method `type` string.
        #[pallet::constant]
        type MaxKeyTypeLength: Get<u32> + Clone + PartialEq;

        /// Maximum byte length of a public key in multibase encoding.
        #[pallet::constant]
        type MaxKeyLength: Get<u32> + Clone + PartialEq;

        /// Maximum number of verification methods per DID document.
        #[pallet::constant]
        type MaxVerificationMethods: Get<u32>;
    }

    // ========== Pallet ==========

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// DID documents keyed by the controller's AccountId.
    ///
    /// DID string = `did:claw:{controller}` — derived from the key, never stored redundantly.
    #[pallet::storage]
    #[pallet::getter(fn did_document)]
    pub type DIDDocuments<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, DIDDocument<T>, OptionQuery>;

    /// Service endpoints, keyed by (controller, endpoint_id_fragment).
    ///
    /// Stored separately from `DIDDocument` to avoid nested-generic derive issues.
    #[pallet::storage]
    #[pallet::getter(fn service_endpoint)]
    pub type ServiceEndpoints<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxServiceIdLength>,
        ServiceEndpoint<T>,
        OptionQuery,
    >;

    /// Verification methods, keyed by (controller, key_id_fragment).
    ///
    /// Stored separately from `DIDDocument` to avoid nested-generic derive issues.
    #[pallet::storage]
    #[pallet::getter(fn verification_method)]
    pub type VerificationMethods<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxKeyIdLength>,
        VerificationMethod<T>,
        OptionQuery,
    >;

    /// Total number of active (non-deactivated) DIDs.
    #[pallet::storage]
    #[pallet::getter(fn did_count)]
    pub type DIDCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new DID was registered.
        DIDRegistered {
            /// Controller account.
            controller: T::AccountId,
        },
        /// A DID document was updated.
        DIDUpdated {
            /// Controller account.
            controller: T::AccountId,
        },
        /// A DID was deactivated. This is permanent.
        DIDDeactivated {
            /// Controller account.
            controller: T::AccountId,
        },
        /// A service endpoint was added to a DID document.
        ServiceEndpointAdded {
            /// Controller account.
            controller: T::AccountId,
            /// Fragment id of the new endpoint.
            endpoint_id: Vec<u8>,
        },
        /// A service endpoint was removed from a DID document.
        ServiceEndpointRemoved {
            /// Controller account.
            controller: T::AccountId,
            /// Fragment id of the removed endpoint.
            endpoint_id: Vec<u8>,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// A DID is already registered for this account.
        DIDAlreadyExists,
        /// No DID found for this account.
        DIDNotFound,
        /// Only the DID controller can perform this action.
        NotController,
        /// This DID has been deactivated and cannot be modified.
        DIDDeactivated,
        /// The context field exceeds the maximum allowed length.
        ContextTooLong,
        /// The service endpoint `id` exceeds the maximum allowed length.
        ServiceIdTooLong,
        /// The service endpoint `type` exceeds the maximum allowed length.
        ServiceTypeTooLong,
        /// The service endpoint URL exceeds the maximum allowed length.
        EndpointTooLong,
        /// The maximum number of service endpoints has been reached.
        TooManyServiceEndpoints,
        /// A service endpoint with this `id` already exists.
        ServiceEndpointAlreadyExists,
        /// No service endpoint found with this `id`.
        ServiceEndpointNotFound,
        /// The verification method `id` exceeds the maximum allowed length.
        KeyIdTooLong,
        /// The verification method `type` exceeds the maximum allowed length.
        KeyTypeTooLong,
        /// The public key exceeds the maximum allowed length.
        KeyTooLong,
        /// The maximum number of verification methods has been reached.
        TooManyVerificationMethods,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new W3C DID document for the caller.
        ///
        /// Creates a DID of the form `did:claw:{caller}` on-chain.
        /// Each account can only have one DID document.
        ///
        /// # Arguments
        /// * `context` - Optional JSON-LD context or metadata bytes
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 2))]
        pub fn register_did(origin: OriginFor<T>, context: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                !DIDDocuments::<T>::contains_key(&who),
                Error::<T>::DIDAlreadyExists
            );

            let bounded_context: BoundedVec<u8, T::MaxContextLength> =
                context.try_into().map_err(|_| Error::<T>::ContextTooLong)?;

            let current_block = <frame_system::Pallet<T>>::block_number();

            let doc = DIDDocument::<T> {
                controller: who.clone(),
                context: bounded_context,
                created: current_block,
                updated: current_block,
                deactivated: false,
                service_endpoint_count: 0,
                verification_method_count: 0,
            };

            DIDDocuments::<T>::insert(&who, doc);
            DIDCount::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::DIDRegistered { controller: who });

            Ok(())
        }

        /// Update the context/metadata of the caller's DID document.
        ///
        /// Only the DID controller (caller) may update the document.
        /// Deactivated DIDs cannot be updated.
        ///
        /// # Arguments
        /// * `context` - New JSON-LD context or metadata bytes
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_did(origin: OriginFor<T>, context: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);

                let bounded_context: BoundedVec<u8, T::MaxContextLength> =
                    context.try_into().map_err(|_| Error::<T>::ContextTooLong)?;

                doc.context = bounded_context;
                doc.updated = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::DIDUpdated { controller: who });

            Ok(())
        }

        /// Permanently deactivate the caller's DID.
        ///
        /// ⚠️ This operation is **irreversible**. Once deactivated, the DID
        /// document cannot be updated and all associated endpoints/keys become invalid.
        /// The document is retained on-chain for historical auditability.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn deactivate_did(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);

                doc.deactivated = true;
                doc.updated = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            DIDCount::<T>::mutate(|n| *n = n.saturating_sub(1));

            Self::deposit_event(Event::DIDDeactivated { controller: who });

            Ok(())
        }

        /// Add a service endpoint to the caller's DID document.
        ///
        /// Service endpoints describe ways to interact with the DID subject
        /// (e.g. RPC URLs, IPFS addresses, REST APIs).
        ///
        /// # Arguments
        /// * `id` - Fragment identifier (e.g. `#rpc`, `#storage`)
        /// * `service_type` - Service type string (e.g. `JsonRpcService`)
        /// * `endpoint` - The service URL
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_service_endpoint(
            origin: OriginFor<T>,
            id: Vec<u8>,
            service_type: Vec<u8>,
            endpoint: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_id: BoundedVec<u8, T::MaxServiceIdLength> =
                id.clone().try_into().map_err(|_| Error::<T>::ServiceIdTooLong)?;
            let bounded_type: BoundedVec<u8, T::MaxServiceTypeLength> =
                service_type.try_into().map_err(|_| Error::<T>::ServiceTypeTooLong)?;
            let bounded_endpoint: BoundedVec<u8, T::MaxEndpointLength> =
                endpoint.try_into().map_err(|_| Error::<T>::EndpointTooLong)?;

            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);
                ensure!(
                    doc.service_endpoint_count < T::MaxServiceEndpoints::get(),
                    Error::<T>::TooManyServiceEndpoints
                );

                // Ensure no duplicate id
                ensure!(
                    !ServiceEndpoints::<T>::contains_key(&who, &bounded_id),
                    Error::<T>::ServiceEndpointAlreadyExists
                );

                let se = ServiceEndpoint::<T> {
                    id: bounded_id.clone(),
                    service_type: bounded_type,
                    endpoint: bounded_endpoint,
                };

                ServiceEndpoints::<T>::insert(&who, &bounded_id, se);
                doc.service_endpoint_count = doc.service_endpoint_count.saturating_add(1);
                doc.updated = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::ServiceEndpointAdded {
                controller: who,
                endpoint_id: id,
            });

            Ok(())
        }

        /// Remove a service endpoint from the caller's DID document.
        ///
        /// # Arguments
        /// * `id` - Fragment identifier of the endpoint to remove
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_service_endpoint(origin: OriginFor<T>, id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let bounded_id: BoundedVec<u8, T::MaxServiceIdLength> =
                id.clone().try_into().map_err(|_| Error::<T>::ServiceIdTooLong)?;

            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);

                ensure!(
                    ServiceEndpoints::<T>::contains_key(&who, &bounded_id),
                    Error::<T>::ServiceEndpointNotFound
                );

                ServiceEndpoints::<T>::remove(&who, &bounded_id);
                doc.service_endpoint_count = doc.service_endpoint_count.saturating_sub(1);
                doc.updated = <frame_system::Pallet<T>>::block_number();

                Ok(())
            })?;

            Self::deposit_event(Event::ServiceEndpointRemoved {
                controller: who,
                endpoint_id: id,
            });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    /// Weight information for the pallet's extrinsics.
    pub trait WeightInfo {
        fn register_did() -> Weight;
        fn update_did() -> Weight;
        fn deactivate_did() -> Weight;
        fn add_service_endpoint() -> Weight;
        fn remove_service_endpoint() -> Weight;
    }

    /// Default unit weights for testing (not for production use).
    impl WeightInfo for () {
        fn register_did() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn update_did() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn deactivate_did() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn add_service_endpoint() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn remove_service_endpoint() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
