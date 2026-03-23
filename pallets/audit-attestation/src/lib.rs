//! # Audit Attestation Pallet
//!
//! On-chain verifiable audit trail for ClawChain agents and contracts.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Storing cryptographically-signed audit attestations on-chain
//! - Verifying auditor identity via pallet-agent-registry
//! - Querying whether a target has been audited within a recency window
//! - Revoking attestations by the auditor or root
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `submit_attestation` - Submit a signed audit attestation for a target hash
//! - `revoke_attestation` - Revoke an existing attestation (auditor or root only)
//!
//! ### Public Functions (for cross-pallet / RPC calls)
//!
//! - `is_audited` - Check if a target has been audited within `max_age_blocks`

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// Interface for cross-pallet integration: checks whether an account is a
/// registered (active) agent in pallet-agent-registry.
pub trait AgentRegistryInterface<AccountId> {
    /// Returns `true` if `account` is a registered, active agent.
    fn is_registered_agent(account: &AccountId) -> bool;
}

#[frame_support::pallet]
pub mod pallet {
    use super::AgentRegistryInterface;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::Saturating;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;

    // =========================================================
    // Types
    // =========================================================

    /// Severity counts for findings in an audit report.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub struct SeverityCounts {
        /// Number of critical findings.
        pub critical: u8,
        /// Number of high-severity findings.
        pub high: u8,
        /// Number of medium-severity findings.
        pub medium: u8,
        /// Number of low-severity findings.
        pub low: u8,
    }

    /// A single audit attestation record stored on-chain.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct AttestationRecord<T: Config> {
        /// DID of the auditing agent.
        pub auditor_did: BoundedVec<u8, T::MaxDidLen>,
        /// On-chain account of the auditing agent.
        pub auditor_account: T::AccountId,
        /// Hash of the artefact / contract being attested.
        pub target_hash: H256,
        /// Hash of the off-chain findings summary document.
        pub findings_summary_hash: H256,
        /// Severity breakdown of findings.
        pub severity_counts: SeverityCounts,
        /// Block number at which the attestation was submitted.
        pub timestamp: BlockNumberFor<T>,
        /// Auditor's off-chain signature over the attestation payload.
        ///
        /// Payload: `target_hash || findings_summary_hash || encode(severity_counts) || block_number`
        pub auditor_signature: BoundedVec<u8, ConstU32<64>>,
    }

    // =========================================================
    // Config
    // =========================================================

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Maximum number of attestation hashes tracked per auditor.
        #[pallet::constant]
        type MaxAttestationsPerAuditor: Get<u32>;

        /// Maximum byte length of an auditor DID.
        #[pallet::constant]
        type MaxDidLen: Get<u32>;

        /// Interface to pallet-agent-registry for auditor identity validation.
        type AgentRegistry: AgentRegistryInterface<Self::AccountId>;
    }

    // =========================================================
    // Pallet struct
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    /// Primary store: `target_hash → AttestationRecord`.
    ///
    /// Keyed by the hash of the artefact being audited.  A new submission for
    /// the same `target_hash` from the *same* auditor overwrites the previous
    /// record (update semantics described in the RFC).
    #[pallet::storage]
    #[pallet::getter(fn attestations)]
    pub type Attestations<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, AttestationRecord<T>, OptionQuery>;

    /// Secondary index: `auditor_account → BoundedVec<target_hash>`.
    ///
    /// Allows efficient lookup of all targets audited by a given account.
    /// Bounded by `MaxAttestationsPerAuditor`.
    #[pallet::storage]
    #[pallet::getter(fn auditor_attestations)]
    pub type AuditorAttestations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<H256, T::MaxAttestationsPerAuditor>,
        ValueQuery,
    >;

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An audit attestation was successfully submitted.
        AttestationSubmitted {
            /// The auditor's account.
            auditor: T::AccountId,
            /// The target artefact hash.
            target_hash: H256,
            /// Block number of the attestation.
            block_number: BlockNumberFor<T>,
        },
        /// An audit attestation was revoked.
        AttestationRevoked {
            /// The auditor's account (original submitter).
            auditor: T::AccountId,
            /// The target artefact hash.
            target_hash: H256,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        /// The origin is not a registered active agent in pallet-agent-registry.
        AuditorNotRegistered,
        /// The supplied signature does not verify against the expected payload.
        InvalidSignature,
        /// No attestation found for the given target hash.
        AttestationNotFound,
        /// The origin is neither the attestation's auditor nor root.
        NotAuditor,
        /// The auditor has reached the maximum number of tracked attestations.
        TooManyAttestations,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit (or overwrite) a signed audit attestation for `target`.
        ///
        /// # Arguments
        /// * `target`       - Hash of the artefact being audited
        /// * `summary_hash` - Hash of the off-chain findings document
        /// * `severities`   - Breakdown of finding severities
        /// * `sig`          - Auditor signature over `(target || summary_hash || encode(severities) || block_number)`
        ///
        /// # Errors
        /// - `AuditorNotRegistered` if the origin is not a registered active agent
        /// - `InvalidSignature`     if `sig` does not verify
        /// - `TooManyAttestations`  if the auditor's list is already at capacity
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000, 0) + T::DbWeight::get().reads_writes(3, 3))]
        pub fn submit_attestation(
            origin: OriginFor<T>,
            target: H256,
            summary_hash: H256,
            severities: SeverityCounts,
            sig: BoundedVec<u8, ConstU32<64>>,
            auditor_did: BoundedVec<u8, T::MaxDidLen>,
        ) -> DispatchResult {
            let auditor = ensure_signed(origin)?;

            // ---- Guard: auditor must be a registered active agent ----
            ensure!(
                T::AgentRegistry::is_registered_agent(&auditor),
                Error::<T>::AuditorNotRegistered
            );

            let current_block = <frame_system::Pallet<T>>::block_number();

            // ---- Verify signature ----
            // Payload: target || summary_hash || SCALE-encode(severities) || SCALE-encode(block)
            ensure!(
                Self::verify_signature(
                    &auditor,
                    &target,
                    &summary_hash,
                    &severities,
                    current_block,
                    &sig
                ),
                Error::<T>::InvalidSignature
            );

            // ---- Update auditor index (add if new target, skip if already tracked) ----
            let already_tracked = AuditorAttestations::<T>::get(&auditor).contains(&target);
            if !already_tracked {
                AuditorAttestations::<T>::try_mutate(&auditor, |list| {
                    list.try_push(target)
                        .map_err(|_| Error::<T>::TooManyAttestations)
                })?;
            }

            // ---- Upsert attestation record ----
            let record = AttestationRecord::<T> {
                auditor_did,
                auditor_account: auditor.clone(),
                target_hash: target,
                findings_summary_hash: summary_hash,
                severity_counts: severities,
                timestamp: current_block,
                auditor_signature: sig,
            };
            Attestations::<T>::insert(target, record);

            Self::deposit_event(Event::AttestationSubmitted {
                auditor,
                target_hash: target,
                block_number: current_block,
            });

            Ok(())
        }

        /// Revoke an existing attestation.
        ///
        /// Only the original auditor or root may revoke.
        ///
        /// # Arguments
        /// * `target` - Hash of the artefact whose attestation should be revoked
        ///
        /// # Errors
        /// - `AttestationNotFound` if no attestation exists for `target`
        /// - `NotAuditor`          if the origin is neither the auditor nor root
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(30_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn revoke_attestation(origin: OriginFor<T>, target: H256) -> DispatchResult {
            // Accept either a signed origin or root.
            let caller_opt = Self::ensure_signed_or_root(origin)?;

            let record = Attestations::<T>::get(target).ok_or(Error::<T>::AttestationNotFound)?;

            // If signed (not root), verify caller is the original auditor.
            if let Some(ref caller) = caller_opt {
                ensure!(*caller == record.auditor_account, Error::<T>::NotAuditor);
            }

            let auditor = record.auditor_account.clone();

            // Remove from primary storage.
            Attestations::<T>::remove(target);

            // Remove from auditor index.
            AuditorAttestations::<T>::mutate(&auditor, |list| {
                list.retain(|h| *h != target);
            });

            Self::deposit_event(Event::AttestationRevoked {
                auditor,
                target_hash: target,
            });

            Ok(())
        }
    }

    // =========================================================
    // Public helper functions
    // =========================================================

    impl<T: Config> Pallet<T> {
        /// Returns `true` if `target` has a valid attestation that is no older
        /// than `max_age_blocks` blocks from the current block.
        ///
        /// Intended for use as an RPC endpoint and from other pallets.
        pub fn is_audited(target: H256, max_age_blocks: u32) -> bool {
            match Attestations::<T>::get(target) {
                None => false,
                Some(record) => {
                    let current_block = <frame_system::Pallet<T>>::block_number();
                    // Use saturating arithmetic to avoid any overflow path.
                    let age = current_block.saturating_sub(record.timestamp);
                    // Convert max_age_blocks (u32) to BlockNumber for comparison.
                    let max_age: BlockNumberFor<T> = max_age_blocks.into();
                    age <= max_age
                    // Note: BlockNumberFor<T> implements PartialOrd so this is safe.
                }
            }
        }

        // ---- Internal helpers ----

        /// Verify the auditor signature over the attestation payload.
        ///
        /// Payload bytes:
        ///   `target(32) || summary_hash(32) || SCALE(severities) || SCALE(block_number)`
        ///
        /// For on-chain verification we use a deterministic SCALE-encoded
        /// payload and `sp_io::crypto::sr25519_verify`.  The signature must be
        /// exactly 64 bytes (SR25519).
        fn verify_signature(
            auditor: &T::AccountId,
            target: &H256,
            summary_hash: &H256,
            severities: &SeverityCounts,
            block: BlockNumberFor<T>,
            sig: &BoundedVec<u8, ConstU32<64>>,
        ) -> bool {
            use codec::Encode;

            // Build the payload.
            let mut payload: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
            payload.extend_from_slice(target.as_bytes());
            payload.extend_from_slice(summary_hash.as_bytes());
            payload.extend_from_slice(&severities.encode());
            payload.extend_from_slice(&block.encode());

            // Signature must be exactly 64 bytes for SR25519.
            let sig_bytes: &[u8] = sig.as_ref();
            if sig_bytes.len() != 64 {
                return false;
            }

            // Convert AccountId to sr25519::Public.
            // We do this via the SCALE-encoded bytes of the AccountId.
            let account_bytes = auditor.encode();
            if account_bytes.len() != 32 {
                // Not a 32-byte AccountId — cannot be an sr25519 public key.
                return false;
            }

            let mut pub_key_bytes = [0u8; 32];
            pub_key_bytes.copy_from_slice(&account_bytes);

            let mut sig_arr = [0u8; 64];
            sig_arr.copy_from_slice(sig_bytes);

            let public = sp_core::sr25519::Public::from_raw(pub_key_bytes);
            let signature = sp_core::sr25519::Signature::from_raw(sig_arr);

            sp_io::crypto::sr25519_verify(&signature, &payload, &public)
        }

        /// Accept either a signed origin or root.
        ///
        /// Returns `Ok(Some(account))` for a signed origin, `Ok(None)` for root.
        fn ensure_signed_or_root(
            origin: OriginFor<T>,
        ) -> Result<Option<T::AccountId>, DispatchError> {
            match origin.into() {
                Ok(frame_system::RawOrigin::Root) => Ok(None),
                Ok(frame_system::RawOrigin::Signed(who)) => Ok(Some(who)),
                _ => Err(DispatchError::BadOrigin),
            }
        }
    }

    // =========================================================
    // WeightInfo trait
    // =========================================================

    pub trait WeightInfo {
        fn submit_attestation() -> Weight;
        fn revoke_attestation() -> Weight;
    }

    impl WeightInfo for () {
        fn submit_attestation() -> Weight {
            Weight::from_parts(50_000, 0)
        }

        fn revoke_attestation() -> Weight {
            Weight::from_parts(30_000, 0)
        }
    }
}
