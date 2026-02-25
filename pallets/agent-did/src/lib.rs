//! # Agent DID Pallet
//!
//! W3C-compliant Decentralized Identifier (DID) system for ClawChain agents.
//!
//! ## Overview
//!
//! DID format: `did:claw:{AccountId}`
//! DID documents with service endpoints and verification methods.
//! Full lifecycle: register, update, deactivate.
//!
//! ## Dispatchable Functions
//!
//! - `register_did` - Register a new DID document
//! - `update_did` - Update the DID context/metadata
//! - `deactivate_did` - Permanently deactivate a DID (irreversible)
//! - `add_service_endpoint` - Add a service endpoint
//! - `remove_service_endpoint` - Remove a service endpoint

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

    // =========================================================
    // Types
    // =========================================================

    /// A service endpoint in a DID document (W3C DID Core §5.4).
    ///
    /// # Note on derives
    ///
    /// `Clone`, `Eq`, and `PartialEq` are intentionally omitted: Rust's derive
    /// macros propagate bounds to ALL type parameters of field types, so deriving
    /// `Clone` on `BoundedVec<u8, T::MaxServiceIdLength>` would require
    /// `T::MaxServiceIdLength: Clone` — but `ConstU32<N>` does not implement `Clone`.
    /// Storage only requires `Encode + Decode + TypeInfo + MaxEncodedLen`.
    ///
    /// `DecodeWithMemTracking` is a **marker trait** (`pub trait DecodeWithMemTracking: Decode {}`).
    /// No derive macro exists for structs; we provide a manual empty impl.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ServiceEndpoint<T: Config> {
        /// Fragment identifier (e.g. `#rpc`, `#storage`).
        pub id: BoundedVec<u8, T::MaxServiceIdLength>,
        /// Service type string (e.g. `JsonRpcService`).
        pub service_type: BoundedVec<u8, T::MaxServiceTypeLength>,
        /// Endpoint URI.
        pub endpoint: BoundedVec<u8, T::MaxEndpointLength>,
    }

    /// Manual `DecodeWithMemTracking` impl for `ServiceEndpoint`.
    /// `DecodeWithMemTracking` is a pure marker trait; we implement it as an empty impl.
    impl<T: Config> codec::DecodeWithMemTracking for ServiceEndpoint<T> {}

    /// A verification method in a DID document (W3C DID Core §5.2).
    ///
    /// Same derive reasoning as `ServiceEndpoint` — only storage traits are needed.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct VerificationMethod<T: Config> {
        /// Fragment identifier (e.g. `#key-1`).
        pub id: BoundedVec<u8, T::MaxKeyIdLength>,
        /// Key type (e.g. `Ed25519VerificationKey2020`).
        pub key_type: BoundedVec<u8, T::MaxKeyTypeLength>,
        /// Public key in multibase format.
        pub public_key_multibase: BoundedVec<u8, T::MaxKeyLength>,
    }

    /// Manual `DecodeWithMemTracking` impl for `VerificationMethod`.
    impl<T: Config> codec::DecodeWithMemTracking for VerificationMethod<T> {}

    /// Core DID document stored on-chain (W3C DID Core §6).
    ///
    /// The DID itself is computed deterministically as `did:claw:{controller}`;
    /// it is never stored redundantly.
    ///
    /// Service endpoints and verification methods are stored in separate
    /// `StorageDoubleMap`s to avoid nested-generic derive issues.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DIDDocument<T: Config> {
        /// Owning account.
        pub controller: T::AccountId,
        /// JSON-LD context or metadata bytes.
        pub context: BoundedVec<u8, T::MaxContextLength>,
        /// Block number when the DID was registered.
        pub created: BlockNumberFor<T>,
        /// Block number of the last update.
        pub updated: BlockNumberFor<T>,
        /// Whether the DID has been deactivated (irreversible).
        pub deactivated: bool,
        /// Cached count of service endpoints (enforces MaxServiceEndpoints).
        pub service_endpoint_count: u32,
        /// Cached count of verification methods (enforces MaxVerificationMethods).
        pub verification_method_count: u32,
    }

    /// Manual `DecodeWithMemTracking` impl for `DIDDocument`.
    impl<T: Config> codec::DecodeWithMemTracking for DIDDocument<T> {}

    // =========================================================
    // Config
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        /// Max byte length of the DID context/metadata field.
        #[pallet::constant]
        type MaxContextLength: Get<u32>;
        /// Max byte length of a service endpoint id fragment.
        #[pallet::constant]
        type MaxServiceIdLength: Get<u32>;
        /// Max byte length of a service endpoint type string.
        #[pallet::constant]
        type MaxServiceTypeLength: Get<u32>;
        /// Max byte length of a service endpoint URL.
        #[pallet::constant]
        type MaxEndpointLength: Get<u32>;
        /// Max number of service endpoints per DID.
        #[pallet::constant]
        type MaxServiceEndpoints: Get<u32>;
        /// Max byte length of a verification method id fragment.
        #[pallet::constant]
        type MaxKeyIdLength: Get<u32>;
        /// Max byte length of a verification method type string.
        #[pallet::constant]
        type MaxKeyTypeLength: Get<u32>;
        /// Max byte length of a public key (multibase encoding).
        #[pallet::constant]
        type MaxKeyLength: Get<u32>;
        /// Max number of verification methods per DID.
        #[pallet::constant]
        type MaxVerificationMethods: Get<u32>;
    }

    // =========================================================
    // Pallet
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    /// DID documents keyed by controller AccountId.
    #[pallet::storage]
    #[pallet::getter(fn did_document)]
    pub type DIDDocuments<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, DIDDocument<T>, OptionQuery>;

    /// Service endpoints: (controller, id_fragment) → ServiceEndpoint.
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

    /// Verification methods: (controller, id_fragment) → VerificationMethod.
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

    /// Count of active (non-deactivated) DIDs.
    #[pallet::storage]
    #[pallet::getter(fn did_count)]
    pub type DIDCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DIDRegistered {
            controller: T::AccountId,
        },
        DIDUpdated {
            controller: T::AccountId,
        },
        DIDDeactivated {
            controller: T::AccountId,
        },
        ServiceEndpointAdded {
            controller: T::AccountId,
            endpoint_id: Vec<u8>,
        },
        ServiceEndpointRemoved {
            controller: T::AccountId,
            endpoint_id: Vec<u8>,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        DIDAlreadyExists,
        DIDNotFound,
        NotController,
        DIDDeactivated,
        ContextTooLong,
        ServiceIdTooLong,
        ServiceTypeTooLong,
        EndpointTooLong,
        TooManyServiceEndpoints,
        ServiceEndpointAlreadyExists,
        ServiceEndpointNotFound,
        KeyIdTooLong,
        KeyTypeTooLong,
        KeyTooLong,
        TooManyVerificationMethods,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new W3C DID document.
        ///
        /// Creates `did:claw:{caller}`. One DID per account.
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
            let now = <frame_system::Pallet<T>>::block_number();

            DIDDocuments::<T>::insert(
                &who,
                DIDDocument::<T> {
                    controller: who.clone(),
                    context: bounded_context,
                    created: now,
                    updated: now,
                    deactivated: false,
                    service_endpoint_count: 0,
                    verification_method_count: 0,
                },
            );
            DIDCount::<T>::mutate(|n| *n = n.saturating_add(1));
            Self::deposit_event(Event::DIDRegistered { controller: who });
            Ok(())
        }

        /// Update the context/metadata of the caller's DID document.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_did(origin: OriginFor<T>, context: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);
                doc.context = context.try_into().map_err(|_| Error::<T>::ContextTooLong)?;
                doc.updated = <frame_system::Pallet<T>>::block_number();
                Ok(())
            })?;
            Self::deposit_event(Event::DIDUpdated { controller: who });
            Ok(())
        }

        /// Permanently deactivate the caller's DID. Irreversible.
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
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn add_service_endpoint(
            origin: OriginFor<T>,
            id: Vec<u8>,
            service_type: Vec<u8>,
            endpoint: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bounded_id: BoundedVec<u8, T::MaxServiceIdLength> = id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ServiceIdTooLong)?;
            let bounded_type: BoundedVec<u8, T::MaxServiceTypeLength> = service_type
                .try_into()
                .map_err(|_| Error::<T>::ServiceTypeTooLong)?;
            let bounded_ep: BoundedVec<u8, T::MaxEndpointLength> = endpoint
                .try_into()
                .map_err(|_| Error::<T>::EndpointTooLong)?;

            DIDDocuments::<T>::try_mutate(&who, |maybe_doc| -> DispatchResult {
                let doc = maybe_doc.as_mut().ok_or(Error::<T>::DIDNotFound)?;
                ensure!(doc.controller == who, Error::<T>::NotController);
                ensure!(!doc.deactivated, Error::<T>::DIDDeactivated);
                ensure!(
                    doc.service_endpoint_count < T::MaxServiceEndpoints::get(),
                    Error::<T>::TooManyServiceEndpoints
                );
                ensure!(
                    !ServiceEndpoints::<T>::contains_key(&who, &bounded_id),
                    Error::<T>::ServiceEndpointAlreadyExists
                );
                ServiceEndpoints::<T>::insert(
                    &who,
                    &bounded_id,
                    ServiceEndpoint::<T> {
                        id: bounded_id.clone(),
                        service_type: bounded_type,
                        endpoint: bounded_ep,
                    },
                );
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
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn remove_service_endpoint(origin: OriginFor<T>, id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let bounded_id: BoundedVec<u8, T::MaxServiceIdLength> = id
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::ServiceIdTooLong)?;

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

    // =========================================================
    // Weight Info
    // =========================================================

    pub trait WeightInfo {
        fn register_did() -> Weight;
        fn update_did() -> Weight;
        fn deactivate_did() -> Weight;
        fn add_service_endpoint() -> Weight;
        fn remove_service_endpoint() -> Weight;
    }

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
