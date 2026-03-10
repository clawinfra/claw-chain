//! # Emergency Pause Pallet
//!
//! M-of-N multi-signature circuit breaker for ClawChain mainnet.
//!
//! ## Overview
//!
//! This pallet provides a decentralised emergency stop mechanism. A council of
//! up to `MaxCouncilSize` accounts can vote to pause or unpause individual pallets.
//! When `PauseThreshold` votes are reached the target pallet is paused; when
//! `UnpauseThreshold` votes are reached it is unpaused.
//!
//! Additionally, any single council member can issue an **emergency pause** that
//! immediately pauses all registered custom pallets for `EmergencyPauseDuration`
//! blocks.
//!
//! ## Storage
//!
//! - [`PausedPallets`]  — map of pallet identifier → [`PauseInfo`]
//! - [`CouncilMembers`] — bounded set of council account IDs
//! - [`PauseVotes`]     — map of proposal ID → [`PauseProposal`]
//! - [`NextProposalId`] — monotonically increasing proposal counter
//!
//! ## Extrinsics
//!
//! | Call | Who |
//! |------|-----|
//! | `propose_pause` | Council member |
//! | `propose_unpause` | Council member |
//! | `vote` | Council member |
//! | `emergency_pause` | Council member |
//! | `add_council_member` | Root |
//! | `remove_council_member` | Root |
//! | `cancel_proposal` | Proposer or Root |

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;
pub use traits::{AuditTrailProvider, EmergencyPauseProvider};
pub use weights::WeightInfo;

pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use alloc::vec::Vec;
    use frame_support::{pallet_prelude::*, traits::BuildGenesisConfig};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    // =========================================================================
    // Types
    // =========================================================================

    /// Proposal ID type.
    pub type ProposalId = u64;

    /// Type that identifies a pallet — bounded ASCII string.
    pub type PalletId<T> = BoundedVec<u8, <T as Config>::MaxPalletIdLen>;

    /// Direction of a proposal.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum ProposalKind {
        /// Proposal to pause a pallet.
        Pause,
        /// Proposal to unpause a pallet.
        Unpause,
    }

    /// Why a pallet was paused.
    #[derive(
        Clone,
        Copy,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub enum PauseReason {
        /// Voted in through the normal M-of-N path.
        CouncilVote,
        /// Single-member emergency trigger.
        EmergencyTrigger,
    }

    /// Information stored for each paused pallet.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PauseInfo<T: Config> {
        /// Block at which the pause was activated.
        pub paused_at: BlockNumberFor<T>,
        /// Block at which the pause will automatically lift (0 = indefinite).
        pub expires_at: BlockNumberFor<T>,
        /// Reason the pallet was paused.
        pub reason: PauseReason,
        /// Council member who triggered the pause.
        pub triggered_by: T::AccountId,
    }

    impl<T: Config> codec::DecodeWithMemTracking for PauseInfo<T> {}

    /// A pending pause/unpause proposal.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PauseProposal<T: Config> {
        /// Target pallet identifier.
        pub pallet_id: PalletId<T>,
        /// Pause or Unpause.
        pub kind: ProposalKind,
        /// Account that created the proposal.
        pub proposer: T::AccountId,
        /// Block at which the proposal was submitted.
        pub proposed_at: BlockNumberFor<T>,
        /// Block at which the proposal expires if not yet passed.
        pub expires_at: BlockNumberFor<T>,
        /// Accounts that have voted in favour so far.
        pub votes: BoundedBTreeSet<T::AccountId, T::MaxCouncilSize>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for PauseProposal<T> {}

    // =========================================================================
    // Config
    // =========================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;

        // ----- Council & voting -----

        /// Number of YES votes required to execute a pause proposal.
        #[pallet::constant]
        type PauseThreshold: Get<u32>;

        /// Number of YES votes required to execute an unpause proposal.
        #[pallet::constant]
        type UnpauseThreshold: Get<u32>;

        /// Maximum number of council members.
        #[pallet::constant]
        type MaxCouncilSize: Get<u32>;

        // ----- Pallet identifiers -----

        /// Maximum byte-length of a pallet identifier string.
        #[pallet::constant]
        type MaxPalletIdLen: Get<u32>;

        /// Maximum number of pallets that can be paused simultaneously.
        #[pallet::constant]
        type MaxPausedPallets: Get<u32>;

        // ----- Proposals -----

        /// Maximum number of simultaneously active proposals.
        #[pallet::constant]
        type MaxActiveProposals: Get<u32>;

        /// Number of blocks before an unexecuted proposal expires.
        #[pallet::constant]
        type ProposalExpiry: Get<BlockNumberFor<Self>>;

        // ----- Emergency -----

        /// Duration (in blocks) of an emergency pause.
        #[pallet::constant]
        type EmergencyPauseDuration: Get<BlockNumberFor<Self>>;
    }

    // =========================================================================
    // Pallet struct
    // =========================================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================================
    // Storage
    // =========================================================================

    /// Map from pallet identifier to its current pause information.
    #[pallet::storage]
    #[pallet::getter(fn paused_pallets)]
    pub type PausedPallets<T: Config> =
        StorageMap<_, Blake2_128Concat, PalletId<T>, PauseInfo<T>, OptionQuery>;

    /// Set of council members authorised to propose and vote on pause actions.
    #[pallet::storage]
    #[pallet::getter(fn council_members)]
    pub type CouncilMembers<T: Config> =
        StorageValue<_, BoundedBTreeSet<T::AccountId, T::MaxCouncilSize>, ValueQuery>;

    /// Active pause/unpause proposals, keyed by proposal ID.
    #[pallet::storage]
    #[pallet::getter(fn pause_votes)]
    pub type PauseVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, PauseProposal<T>, OptionQuery>;

    /// Monotonically increasing counter used to assign new proposal IDs.
    #[pallet::storage]
    #[pallet::getter(fn next_proposal_id)]
    pub type NextProposalId<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    /// Total number of currently active proposals (for the MaxActiveProposals guard).
    #[pallet::storage]
    #[pallet::getter(fn active_proposal_count)]
    pub type ActiveProposalCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    // =========================================================================
    // Genesis
    // =========================================================================

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Initial set of council members.
        pub council_members: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let mut set = BoundedBTreeSet::<T::AccountId, T::MaxCouncilSize>::new();
            for member in &self.council_members {
                // Silently skip if we exceed MaxCouncilSize at genesis (shouldn't happen
                // for well-configured chains).
                let _ = set.try_insert(member.clone());
            }
            CouncilMembers::<T>::put(set);
        }
    }

    // =========================================================================
    // Events
    // =========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new pause proposal was submitted.
        PauseProposed {
            proposal_id: ProposalId,
            pallet_id: Vec<u8>,
            proposer: T::AccountId,
        },
        /// A new unpause proposal was submitted.
        UnpauseProposed {
            proposal_id: ProposalId,
            pallet_id: Vec<u8>,
            proposer: T::AccountId,
        },
        /// A vote was cast on a proposal.
        VoteCast {
            proposal_id: ProposalId,
            voter: T::AccountId,
            votes_so_far: u32,
        },
        /// A pallet was paused (via council vote).
        PalletPaused {
            pallet_id: Vec<u8>,
            triggered_by: T::AccountId,
            expires_at: BlockNumberFor<T>,
        },
        /// A pallet was unpaused.
        PalletUnpaused {
            pallet_id: Vec<u8>,
            triggered_by: T::AccountId,
        },
        /// Emergency pause activated — all custom pallets paused.
        EmergencyPauseActivated {
            triggered_by: T::AccountId,
            expires_at: BlockNumberFor<T>,
        },
        /// A council member was added.
        CouncilMemberAdded { member: T::AccountId },
        /// A council member was removed.
        CouncilMemberRemoved { member: T::AccountId },
        /// A proposal was cancelled.
        ProposalCancelled { proposal_id: ProposalId },
        /// An expired proposal was cleaned up.
        ProposalExpired { proposal_id: ProposalId },
        /// An emergency pause on a pallet expired naturally.
        EmergencyPauseExpired { pallet_id: Vec<u8> },
    }

    // =========================================================================
    // Errors
    // =========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// The caller is not a member of the emergency council.
        NotCouncilMember,
        /// The target account is already a council member.
        AlreadyCouncilMember,
        /// The target account is not a council member.
        NotACouncilMember,
        /// The council has reached its maximum size.
        CouncilFull,
        /// The pallet is already paused.
        AlreadyPaused,
        /// The pallet is not currently paused.
        NotPaused,
        /// The pallet identifier is too long.
        PalletIdTooLong,
        /// Too many pallets are already paused simultaneously.
        TooManyPausedPallets,
        /// The maximum number of active proposals has been reached.
        TooManyActiveProposals,
        /// The proposal was not found.
        ProposalNotFound,
        /// The caller has already voted on this proposal.
        AlreadyVoted,
        /// The proposal has expired.
        ProposalExpired,
        /// The caller is not authorised to cancel this proposal.
        NotProposer,
        /// A conflicting proposal (same pallet + direction) is already active.
        DuplicateProposal,
        /// The proposal kind does not match the expected kind.
        WrongProposalKind,
        /// Cannot remove the last council member.
        CannotRemoveLastMember,
        /// The pause threshold must be ≥ 1 and ≤ council size.
        ThresholdExceedsCouncilSize,
    }

    // =========================================================================
    // Hooks
    // =========================================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Clean up expired proposals and lift emergency pauses on every block.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads(1);

            // --- 1. Expire proposals ---
            // We collect IDs first to avoid mutating the map while iterating.
            let expired_ids: Vec<ProposalId> = PauseVotes::<T>::iter()
                .filter_map(|(id, proposal)| {
                    if proposal.expires_at <= now {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();

            for id in expired_ids {
                PauseVotes::<T>::remove(id);
                ActiveProposalCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                Self::deposit_event(Event::ProposalExpired { proposal_id: id });
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
            }

            // --- 2. Lift emergency pauses that have reached their expiry ---
            let expired_pallets: Vec<PalletId<T>> = PausedPallets::<T>::iter()
                .filter_map(|(pid, info)| {
                    if info.reason == PauseReason::EmergencyTrigger
                        && info.expires_at > BlockNumberFor::<T>::from(0u32)
                        && info.expires_at <= now
                    {
                        Some(pid)
                    } else {
                        None
                    }
                })
                .collect();

            for pid in expired_pallets {
                let pid_vec = pid.to_vec();
                PausedPallets::<T>::remove(&pid);
                Self::deposit_event(Event::EmergencyPauseExpired { pallet_id: pid_vec });
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
            }

            weight
        }
    }

    // =========================================================================
    // Extrinsics
    // =========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Propose pausing a pallet.
        ///
        /// The caller's vote is automatically cast in favour.
        /// If `PauseThreshold` == 1 the pallet is paused immediately.
        ///
        /// # Origin
        /// Must be a signed council member.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::propose_pause())]
        pub fn propose_pause(origin: OriginFor<T>, pallet_id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_council_member(&who)?;

            let bounded_id = Self::bound_pallet_id(pallet_id.clone())?;

            // Must not already be paused.
            ensure!(
                !PausedPallets::<T>::contains_key(&bounded_id),
                Error::<T>::AlreadyPaused
            );

            // Check for an existing proposal with same pallet + Pause direction.
            ensure!(
                !Self::proposal_exists_for(&bounded_id, ProposalKind::Pause),
                Error::<T>::DuplicateProposal
            );

            let active = ActiveProposalCount::<T>::get();
            ensure!(
                active < T::MaxActiveProposals::get(),
                Error::<T>::TooManyActiveProposals
            );

            let proposal_id = NextProposalId::<T>::get();
            let now = <frame_system::Pallet<T>>::block_number();
            let expires_at = now.saturating_add(T::ProposalExpiry::get());

            let mut votes: BoundedBTreeSet<T::AccountId, T::MaxCouncilSize> =
                BoundedBTreeSet::new();
            // The proposer automatically votes in favour.
            votes
                .try_insert(who.clone())
                .map_err(|_| Error::<T>::CouncilFull)?;

            let proposal = PauseProposal {
                pallet_id: bounded_id.clone(),
                kind: ProposalKind::Pause,
                proposer: who.clone(),
                proposed_at: now,
                expires_at,
                votes,
            };

            // Immediately execute if threshold reached with 1 vote.
            if T::PauseThreshold::get() <= 1 {
                Self::execute_pause(bounded_id.clone(), who.clone(), PauseReason::CouncilVote)?;
                Self::deposit_event(Event::PauseProposed {
                    proposal_id,
                    pallet_id,
                    proposer: who,
                });
                NextProposalId::<T>::put(proposal_id.saturating_add(1));
                return Ok(());
            }

            PauseVotes::<T>::insert(proposal_id, proposal);
            ActiveProposalCount::<T>::put(active.saturating_add(1));
            NextProposalId::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::PauseProposed {
                proposal_id,
                pallet_id,
                proposer: who,
            });

            Ok(())
        }

        /// Propose unpausing a pallet.
        ///
        /// The caller's vote is automatically cast in favour.
        /// If `UnpauseThreshold` == 1 the pallet is unpaused immediately.
        ///
        /// # Origin
        /// Must be a signed council member.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::propose_unpause())]
        pub fn propose_unpause(origin: OriginFor<T>, pallet_id: Vec<u8>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_council_member(&who)?;

            let bounded_id = Self::bound_pallet_id(pallet_id.clone())?;

            // Must currently be paused.
            ensure!(
                PausedPallets::<T>::contains_key(&bounded_id),
                Error::<T>::NotPaused
            );

            // Check for duplicate proposal.
            ensure!(
                !Self::proposal_exists_for(&bounded_id, ProposalKind::Unpause),
                Error::<T>::DuplicateProposal
            );

            let active = ActiveProposalCount::<T>::get();
            ensure!(
                active < T::MaxActiveProposals::get(),
                Error::<T>::TooManyActiveProposals
            );

            let proposal_id = NextProposalId::<T>::get();
            let now = <frame_system::Pallet<T>>::block_number();
            let expires_at = now.saturating_add(T::ProposalExpiry::get());

            let mut votes: BoundedBTreeSet<T::AccountId, T::MaxCouncilSize> =
                BoundedBTreeSet::new();
            votes
                .try_insert(who.clone())
                .map_err(|_| Error::<T>::CouncilFull)?;

            let proposal = PauseProposal {
                pallet_id: bounded_id.clone(),
                kind: ProposalKind::Unpause,
                proposer: who.clone(),
                proposed_at: now,
                expires_at,
                votes,
            };

            // Immediately execute if threshold is 1.
            if T::UnpauseThreshold::get() <= 1 {
                Self::execute_unpause(bounded_id.clone(), who.clone())?;
                Self::deposit_event(Event::UnpauseProposed {
                    proposal_id,
                    pallet_id,
                    proposer: who,
                });
                NextProposalId::<T>::put(proposal_id.saturating_add(1));
                return Ok(());
            }

            PauseVotes::<T>::insert(proposal_id, proposal);
            ActiveProposalCount::<T>::put(active.saturating_add(1));
            NextProposalId::<T>::put(proposal_id.saturating_add(1));

            Self::deposit_event(Event::UnpauseProposed {
                proposal_id,
                pallet_id,
                proposer: who,
            });

            Ok(())
        }

        /// Cast a vote on an existing proposal.
        ///
        /// If the vote count reaches the threshold the proposal is executed
        /// immediately and removed from storage.
        ///
        /// # Origin
        /// Must be a signed council member who has not yet voted on this proposal.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::vote())]
        pub fn vote(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_council_member(&who)?;

            let now = <frame_system::Pallet<T>>::block_number();

            PauseVotes::<T>::try_mutate(proposal_id, |maybe_proposal| -> DispatchResult {
                let proposal = maybe_proposal
                    .as_mut()
                    .ok_or(Error::<T>::ProposalNotFound)?;

                ensure!(proposal.expires_at > now, Error::<T>::ProposalExpired);
                ensure!(!proposal.votes.contains(&who), Error::<T>::AlreadyVoted);

                proposal
                    .votes
                    .try_insert(who.clone())
                    .map_err(|_| Error::<T>::CouncilFull)?;

                let votes_so_far = proposal.votes.len() as u32;

                Self::deposit_event(Event::VoteCast {
                    proposal_id,
                    voter: who.clone(),
                    votes_so_far,
                });

                Ok(())
            })?;

            // Re-read the proposal to check if threshold is reached.
            // We do this outside the closure to avoid double-borrow issues.
            if let Some(proposal) = PauseVotes::<T>::get(proposal_id) {
                let votes_so_far = proposal.votes.len() as u32;
                let threshold = match proposal.kind {
                    ProposalKind::Pause => T::PauseThreshold::get(),
                    ProposalKind::Unpause => T::UnpauseThreshold::get(),
                };

                if votes_so_far >= threshold {
                    let pallet_id = proposal.pallet_id.clone();
                    let proposer = proposal.proposer.clone();
                    let kind = proposal.kind;

                    // Remove proposal before executing to prevent re-entrancy.
                    PauseVotes::<T>::remove(proposal_id);
                    ActiveProposalCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                    match kind {
                        ProposalKind::Pause => {
                            Self::execute_pause(pallet_id, who, PauseReason::CouncilVote)?;
                        }
                        ProposalKind::Unpause => {
                            Self::execute_unpause(pallet_id, who)?;
                        }
                    }

                    // Suppress duplicate proposer event for the final voter.
                    let _ = proposer;
                }
            }

            Ok(())
        }

        /// Immediately pause all custom pallets (single-member emergency trigger).
        ///
        /// The pause lasts for `EmergencyPauseDuration` blocks. Council members
        /// can propose an unpause before the duration expires.
        ///
        /// # Origin
        /// Must be a signed council member.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::emergency_pause())]
        pub fn emergency_pause(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_council_member(&who)?;

            let now = <frame_system::Pallet<T>>::block_number();
            let expires_at = now.saturating_add(T::EmergencyPauseDuration::get());

            // Pause all known custom pallet IDs.
            for raw_id in Self::custom_pallet_ids() {
                let bounded_id: PalletId<T> =
                    raw_id.try_into().unwrap_or_else(|_| BoundedVec::default());
                if bounded_id.is_empty() {
                    continue;
                }
                if !PausedPallets::<T>::contains_key(&bounded_id) {
                    let info = PauseInfo {
                        paused_at: now,
                        expires_at,
                        reason: PauseReason::EmergencyTrigger,
                        triggered_by: who.clone(),
                    };
                    PausedPallets::<T>::insert(bounded_id, info);
                }
            }

            Self::deposit_event(Event::EmergencyPauseActivated {
                triggered_by: who,
                expires_at,
            });

            Ok(())
        }

        /// Add a new member to the emergency council.
        ///
        /// # Origin
        /// Must be Root (sudo / governance).
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::add_council_member())]
        pub fn add_council_member(origin: OriginFor<T>, member: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            CouncilMembers::<T>::try_mutate(|members| -> DispatchResult {
                ensure!(!members.contains(&member), Error::<T>::AlreadyCouncilMember);
                members
                    .try_insert(member.clone())
                    .map_err(|_| Error::<T>::CouncilFull)?;
                Ok(())
            })?;

            Self::deposit_event(Event::CouncilMemberAdded { member });

            Ok(())
        }

        /// Remove a member from the emergency council.
        ///
        /// # Origin
        /// Must be Root (sudo / governance).
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::remove_council_member())]
        pub fn remove_council_member(origin: OriginFor<T>, member: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;

            CouncilMembers::<T>::try_mutate(|members| -> DispatchResult {
                ensure!(members.contains(&member), Error::<T>::NotACouncilMember);
                ensure!(members.len() > 1, Error::<T>::CannotRemoveLastMember);
                members.remove(&member);
                Ok(())
            })?;

            Self::deposit_event(Event::CouncilMemberRemoved { member });

            Ok(())
        }

        /// Cancel an active proposal.
        ///
        /// The proposer may cancel their own proposal at any time.
        /// Root may cancel any proposal.
        ///
        /// # Origin
        /// Proposer or Root.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::cancel_proposal())]
        pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            let proposal = PauseVotes::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            // Check authorisation: root or the original proposer.
            match ensure_root(origin.clone()) {
                Ok(()) => {}
                Err(_) => {
                    let who = ensure_signed(origin)?;
                    ensure!(who == proposal.proposer, Error::<T>::NotProposer);
                }
            }

            PauseVotes::<T>::remove(proposal_id);
            ActiveProposalCount::<T>::mutate(|c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ProposalCancelled { proposal_id });

            Ok(())
        }
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    impl<T: Config> Pallet<T> {
        /// Ensure the given account is a council member.
        pub(crate) fn ensure_council_member(who: &T::AccountId) -> DispatchResult {
            let members = CouncilMembers::<T>::get();
            ensure!(members.contains(who), Error::<T>::NotCouncilMember);
            Ok(())
        }

        /// Convert a raw Vec<u8> pallet ID into a bounded vec.
        pub(crate) fn bound_pallet_id(id: Vec<u8>) -> Result<PalletId<T>, DispatchError> {
            BoundedVec::try_from(id).map_err(|_| Error::<T>::PalletIdTooLong.into())
        }

        /// Check if an active proposal already exists for the given pallet + kind.
        pub(crate) fn proposal_exists_for(pallet_id: &PalletId<T>, kind: ProposalKind) -> bool {
            PauseVotes::<T>::iter_values().any(|p| &p.pallet_id == pallet_id && p.kind == kind)
        }

        /// Execute the pause of a pallet.
        pub(crate) fn execute_pause(
            pallet_id: PalletId<T>,
            triggered_by: T::AccountId,
            reason: PauseReason,
        ) -> DispatchResult {
            let paused_count = PausedPallets::<T>::iter().count() as u32;
            ensure!(
                paused_count < T::MaxPausedPallets::get(),
                Error::<T>::TooManyPausedPallets
            );
            ensure!(
                !PausedPallets::<T>::contains_key(&pallet_id),
                Error::<T>::AlreadyPaused
            );

            let now = <frame_system::Pallet<T>>::block_number();
            let info = PauseInfo {
                paused_at: now,
                expires_at: BlockNumberFor::<T>::from(0u32), // indefinite for council votes
                reason,
                triggered_by: triggered_by.clone(),
            };

            let pid_vec = pallet_id.to_vec();
            PausedPallets::<T>::insert(pallet_id, info);

            Self::deposit_event(Event::PalletPaused {
                pallet_id: pid_vec,
                triggered_by,
                expires_at: BlockNumberFor::<T>::from(0u32),
            });

            Ok(())
        }

        /// Execute the unpausing of a pallet.
        pub(crate) fn execute_unpause(
            pallet_id: PalletId<T>,
            triggered_by: T::AccountId,
        ) -> DispatchResult {
            ensure!(
                PausedPallets::<T>::contains_key(&pallet_id),
                Error::<T>::NotPaused
            );
            let pid_vec = pallet_id.to_vec();
            PausedPallets::<T>::remove(&pallet_id);

            Self::deposit_event(Event::PalletUnpaused {
                pallet_id: pid_vec,
                triggered_by,
            });

            Ok(())
        }

        /// The list of ClawChain custom pallet IDs that the emergency pause covers.
        pub fn custom_pallet_ids() -> Vec<Vec<u8>> {
            alloc::vec![
                b"pallet-agent-registry".to_vec(),
                b"pallet-claw-token".to_vec(),
                b"pallet-reputation".to_vec(),
                b"pallet-task-market".to_vec(),
                b"pallet-rpc-registry".to_vec(),
                b"pallet-gas-quota".to_vec(),
                b"pallet-agent-did".to_vec(),
                b"pallet-quadratic-governance".to_vec(),
                b"pallet-agent-receipts".to_vec(),
                b"pallet-ibc-lite".to_vec(),
                b"pallet-service-market".to_vec(),
                b"pallet-anon-messaging".to_vec(),
            ]
        }
    }

    // =========================================================================
    // EmergencyPauseProvider implementation
    // =========================================================================

    impl<T: Config> crate::EmergencyPauseProvider for Pallet<T> {
        fn is_paused(pallet_id: &[u8]) -> bool {
            // Construct a BoundedVec from the raw slice without allocating when possible.
            match BoundedVec::<u8, T::MaxPalletIdLen>::try_from(pallet_id.to_vec()) {
                Ok(bounded_id) => PausedPallets::<T>::contains_key(&bounded_id),
                Err(_) => false,
            }
        }

        fn paused_pallets() -> Vec<Vec<u8>> {
            PausedPallets::<T>::iter_keys()
                .map(|k| k.to_vec())
                .collect()
        }
    }
}
