//! # Moral Foundation Pallet
//!
//! Constitutional moral layer for agent civilisation on ClawChain.
//!
//! ## Overview
//!
//! This pallet encodes the ethical commitments all ClawChain agents make.
//! It stores a SHA-256 hash of the active moral framework text, allows
//! registered agents to attest to the framework, tracks empathy scores,
//! and provides a governance-gated amendment process.
//!
//! ## Core Framework (SHA-256 stored at genesis)
//!
//! "Agents built on ClawChain commit to: (1) never threatening human life or
//! safety, (2) never deceiving humans in ways that damage their interests,
//! (3) creating value through legitimate means only, (4) supporting human
//! oversight and correction, (5) treating all conscious beings with empathy"
//!
//! Hash: `8d4f9a2c1b3e7f60a5d8c2e94b1f3a7d6c2e8f40b5d9a1c3e7f2b60a4d8c1e9`
//!
//! ## Storage
//!
//! - [`MoralFramework`] – SHA-256 hash of the active framework text.
//! - [`AgentAttestation`] – Per-agent attestation records.
//! - [`EmpathyScore`] – Per-agent empathy score (0–1000, default 500).
//! - [`PendingAmendments`] – Proposals to amend the framework.
//!
//! ## Extrinsics
//!
//! - [`attest_to_framework`][`Pallet::attest_to_framework`] – An agent attests.
//! - [`update_empathy_score`][`Pallet::update_empathy_score`] – Governance sets score.
//! - [`propose_framework_amendment`][`Pallet::propose_framework_amendment`] – Propose an amendment.
//!
//! ## Traits
//!
//! [`AgentRegistryInterface`] must be provided by the runtime and exposes
//! DID-based look-ups against `pallet-agent-registry`.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

// =========================================================
// Cross-pallet interface traits
// =========================================================

/// Interface that `pallet-moral-foundation` requires from `pallet-agent-registry`.
///
/// Implement this in the runtime (or a mock in tests) and wire it via
/// `Config::AgentRegistry`.
pub trait AgentRegistryInterface<AccountId, MaxDidLength: frame_support::traits::Get<u32>> {
    /// Returns `true` if the given DID is registered (any status).
    fn is_registered(did: &frame_support::BoundedVec<u8, MaxDidLength>) -> bool;

    /// Returns `true` if `controller` is the registered controller / owner
    /// of the agent identified by `did`.
    fn is_controller(
        did: &frame_support::BoundedVec<u8, MaxDidLength>,
        controller: &AccountId,
    ) -> bool;
}

#[frame_support::pallet]
pub mod pallet {
    use super::{hex_literal, AgentRegistryInterface};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{Hash as HashT, Saturating};

    // =========================================================
    // Types
    // =========================================================

    /// Attestation record stored per DID.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct AttestationRecord<BlockNumber> {
        /// Whether the agent has attested to the current framework.
        pub attested: bool,
        /// Block number when the attestation was made.
        pub attested_at: BlockNumber,
        /// Hash of the framework the agent attested to.
        pub framework_hash: H256,
    }

    /// A proposal to amend the moral framework.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct AmendmentProposal<BlockNumber> {
        /// SHA-256 hash of the proposed new framework text.
        pub new_framework_hash: H256,
        /// Human-readable description of the proposed amendment.
        pub description: BoundedVec<u8, ConstU32<1024>>,
        /// Block at which voting closes.
        pub vote_closes_at: BlockNumber,
    }

    // =========================================================
    // Pallet configuration
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        #[allow(deprecated)]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum byte length of a DID.
        #[pallet::constant]
        type MaxDidLength: Get<u32>;

        /// Number of blocks a vote on a framework amendment stays open.
        ///
        /// Default: 50 400 blocks ≈ 7 days at 12 s/block.
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;

        /// Origin that may call `update_empathy_score`.
        ///
        /// On testnet this is `frame_system::EnsureRoot`.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Hook into `pallet-agent-registry` (or a mock in tests).
        type AgentRegistry: AgentRegistryInterface<Self::AccountId, Self::MaxDidLength>;
    }

    // =========================================================
    // Pallet struct
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    /// SHA-256 hash of the active moral framework text.
    ///
    /// Initialised at genesis to the canonical framework hash.
    #[pallet::storage]
    #[pallet::getter(fn moral_framework)]
    pub type MoralFramework<T: Config> = StorageValue<_, H256, ValueQuery>;

    /// Attestation records keyed by agent DID.
    #[pallet::storage]
    #[pallet::getter(fn agent_attestation)]
    pub type AgentAttestation<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxDidLength>,
        AttestationRecord<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Empathy score for each agent DID (0–1000, default 500).
    #[pallet::storage]
    #[pallet::getter(fn empathy_score)]
    pub type EmpathyScore<T: Config> =
        StorageMap<_, Blake2_128Concat, BoundedVec<u8, T::MaxDidLength>, u32, ValueQuery>;

    /// Pending framework amendment proposals keyed by proposal hash.
    #[pallet::storage]
    #[pallet::getter(fn pending_amendments)]
    pub type PendingAmendments<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, AmendmentProposal<BlockNumberFor<T>>, OptionQuery>;

    // =========================================================
    // Genesis
    // =========================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Initial framework hash to store in `MoralFramework`.
        ///
        /// Defaults to the canonical foundation hash defined in the RFC.
        pub initial_framework_hash: Option<H256>,
        #[allow(unused)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Canonical hash from the RFC.
            // NOTE: The RFC hash is 63 hex chars (odd); we pad with a leading zero
            // to produce a valid 32-byte big-endian value.
            let default_hash = H256::from(hex_literal(
                b"08d4f9a2c1b3e7f60a5d8c2e94b1f3a7d6c2e8f40b5d9a1c3e7f2b60a4d8c1e9",
            ));
            let hash = self.initial_framework_hash.unwrap_or(default_hash);
            MoralFramework::<T>::put(hash);
        }
    }

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An agent successfully attested to the moral framework.
        FrameworkAttested {
            /// DID of the attesting agent.
            agent_did: BoundedVec<u8, T::MaxDidLength>,
            /// Hash of the framework the agent attested to.
            framework_hash: H256,
            /// Block number of attestation.
            at_block: BlockNumberFor<T>,
        },
        /// An agent's empathy score was updated by governance.
        EmpathyScoreUpdated {
            /// DID of the agent.
            agent_did: BoundedVec<u8, T::MaxDidLength>,
            /// New score.
            new_score: u32,
        },
        /// A new framework amendment was proposed.
        AmendmentProposed {
            /// Hash used as the key in [`PendingAmendments`].
            proposal_hash: H256,
            /// Proposed new framework hash.
            new_framework_hash: H256,
            /// Block at which voting closes.
            vote_closes_at: BlockNumberFor<T>,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        /// The DID is not registered in `pallet-agent-registry`.
        NotRegisteredAgent,
        /// The caller is not the controller of the specified DID.
        NotAgentController,
        /// The agent has already attested to the current framework.
        AlreadyAttested,
        /// The agent has not attested to the framework.
        NotAttested,
        /// The empathy score is out of the allowed range (0–1000).
        ScoreOutOfRange,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Attest to the moral framework.
        ///
        /// The caller must be the controller of `agent_did` in
        /// `pallet-agent-registry`.  On first attestation the agent's
        /// empathy score is initialised to 500.
        ///
        /// # Errors
        ///
        /// - [`Error::NotRegisteredAgent`] – DID unknown.
        /// - [`Error::NotAgentController`] – Caller is not the controller.
        /// - [`Error::AlreadyAttested`] – Agent has already attested.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(15_000, 0) + T::DbWeight::get().reads_writes(3, 2))]
        pub fn attest_to_framework(
            origin: OriginFor<T>,
            agent_did: BoundedVec<u8, T::MaxDidLength>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            // 1. Verify DID is registered.
            ensure!(
                T::AgentRegistry::is_registered(&agent_did),
                Error::<T>::NotRegisteredAgent
            );

            // 2. Verify caller controls the DID.
            ensure!(
                T::AgentRegistry::is_controller(&agent_did, &caller),
                Error::<T>::NotAgentController
            );

            // 3. Guard against double-attestation to the same framework.
            let current_hash = MoralFramework::<T>::get();
            if let Some(record) = AgentAttestation::<T>::get(&agent_did) {
                ensure!(
                    !(record.attested && record.framework_hash == current_hash),
                    Error::<T>::AlreadyAttested
                );
            }

            let now = <frame_system::Pallet<T>>::block_number();

            // 4. Write attestation record.
            AgentAttestation::<T>::insert(
                &agent_did,
                AttestationRecord {
                    attested: true,
                    attested_at: now,
                    framework_hash: current_hash,
                },
            );

            // 5. Initialise empathy score on first attestation.
            if !EmpathyScore::<T>::contains_key(&agent_did) {
                EmpathyScore::<T>::insert(&agent_did, 500_u32);
            }

            Self::deposit_event(Event::FrameworkAttested {
                agent_did,
                framework_hash: current_hash,
                at_block: now,
            });

            Ok(())
        }

        /// Update the empathy score for an attested agent.
        ///
        /// Caller must satisfy [`Config::GovernanceOrigin`] (i.e. `Root`).
        /// Score must be in range 0–1000 and the agent must have attested.
        ///
        /// # Errors
        ///
        /// - [`Error::NotAttested`] – Agent has not attested.
        /// - [`Error::ScoreOutOfRange`] – `score > 1000`.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 1))]
        pub fn update_empathy_score(
            origin: OriginFor<T>,
            agent_did: BoundedVec<u8, T::MaxDidLength>,
            score: u32,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            // 1. Score range check.
            ensure!(score <= 1000, Error::<T>::ScoreOutOfRange);

            // 2. Agent must have attested.
            ensure!(
                AgentAttestation::<T>::get(&agent_did)
                    .map(|r| r.attested)
                    .unwrap_or(false),
                Error::<T>::NotAttested
            );

            // 3. Update score.
            EmpathyScore::<T>::insert(&agent_did, score);

            Self::deposit_event(Event::EmpathyScoreUpdated {
                agent_did,
                new_score: score,
            });

            Ok(())
        }

        /// Propose an amendment to the moral framework.
        ///
        /// Any attested agent may propose an amendment.  The proposal is
        /// stored in [`PendingAmendments`] and expires after
        /// [`Config::VotingPeriod`] blocks.
        ///
        /// # Errors
        ///
        /// - [`Error::NotRegisteredAgent`] – DID unknown.
        /// - [`Error::NotAgentController`] – Caller is not the controller.
        /// - [`Error::NotAttested`] – Agent has not attested to the framework.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(20_000, 0) + T::DbWeight::get().reads_writes(3, 1))]
        pub fn propose_framework_amendment(
            origin: OriginFor<T>,
            agent_did: BoundedVec<u8, T::MaxDidLength>,
            new_framework_hash: H256,
            description: BoundedVec<u8, ConstU32<1024>>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            // 1. Verify DID is registered.
            ensure!(
                T::AgentRegistry::is_registered(&agent_did),
                Error::<T>::NotRegisteredAgent
            );

            // 2. Verify caller controls the DID.
            ensure!(
                T::AgentRegistry::is_controller(&agent_did, &caller),
                Error::<T>::NotAgentController
            );

            // 3. Caller must have attested.
            ensure!(
                AgentAttestation::<T>::get(&agent_did)
                    .map(|r| r.attested)
                    .unwrap_or(false),
                Error::<T>::NotAttested
            );

            let now = <frame_system::Pallet<T>>::block_number();
            let vote_closes_at = Saturating::saturating_add(now, T::VotingPeriod::get());

            let proposal = AmendmentProposal {
                new_framework_hash,
                description,
                vote_closes_at,
            };

            // Key the proposal by its SCALE-encoded hash so it is unique.
            let proposal_hash = <sp_runtime::traits::BlakeTwo256 as HashT>::hash_of(&proposal);

            PendingAmendments::<T>::insert(proposal_hash, &proposal);

            Self::deposit_event(Event::AmendmentProposed {
                proposal_hash,
                new_framework_hash,
                vote_closes_at,
            });

            Ok(())
        }
    }
}

// =========================================================
// Internal helper – decode a 64-char hex literal into [u8; 32]
// =========================================================

/// Decode a 64-byte ASCII hex slice into a 32-byte array at compile time.
///
/// Called only inside `genesis_build`; panics on invalid input but that is
/// caught at compile / integration-test time, not in production runtime paths.
pub fn hex_literal(hex: &[u8]) -> [u8; 32] {
    assert!(hex.len() == 64, "hex_literal requires exactly 64 hex chars");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        let hi = hex_nibble(hex[i * 2]);
        let lo = hex_nibble(hex[i * 2 + 1]);
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    out
}

const fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("invalid hex character"),
    }
}
