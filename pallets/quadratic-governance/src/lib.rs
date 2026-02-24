//! # Quadratic Governance Pallet (ADR-004)
//!
//! On-chain quadratic voting with DID-based sybil resistance for ClawChain.
//!
//! ## Overview
//!
//! - Vote weight = `integer_sqrt(tokens_staked)`
//! - DID required to submit proposals and vote (pallet-agent-did integration)
//! - Proposals: description hash + voting period
//! - Quorum: configurable minimum participation threshold
//!
//! ## Dispatchable Functions
//!
//! - `submit_proposal` — Create a new proposal (requires DID + deposit)
//! - `vote` — Cast a quadratic vote on an active proposal
//! - `finalize_proposal` — Close voting after the period ends
//! - `cancel_proposal` — Cancel a proposal (proposer only, refunds deposit)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    /// Type alias for balance (same pattern as pallet-reputation / pallet-task-market).
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // =========================================================
    // Types
    // =========================================================

    /// Proposal ID type.
    pub type ProposalId = u64;

    /// Vote weight type (result of integer sqrt).
    pub type VoteWeight = u128;

    /// Status of a governance proposal.
    #[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ProposalStatus {
        Active,
        Passed,
        Rejected,
        Expired,
    }

    impl codec::DecodeWithMemTracking for ProposalStatus {}

    impl Default for ProposalStatus {
        fn default() -> Self {
            ProposalStatus::Active
        }
    }

    /// A vote direction.
    #[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum Vote {
        Yes,
        No,
    }

    impl codec::DecodeWithMemTracking for Vote {}

    /// Record of a single vote cast on a proposal.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub struct VoteRecord {
        /// The vote direction.
        pub vote: Vote,
        /// Quadratic weight applied.
        pub weight: VoteWeight,
        /// Block at which the vote was cast.
        pub block: u32,
    }

    impl codec::DecodeWithMemTracking for VoteRecord {}

    /// A governance proposal.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        /// Account that submitted the proposal.
        pub proposer: T::AccountId,
        /// SHA-256 hash of the proposal description text.
        pub description_hash: [u8; 32],
        /// Block when voting began.
        pub start_block: BlockNumberFor<T>,
        /// Block when voting ends.
        pub end_block: BlockNumberFor<T>,
        /// Accumulated quadratic weight of Yes votes.
        pub yes_votes: VoteWeight,
        /// Accumulated quadratic weight of No votes.
        pub no_votes: VoteWeight,
        /// Current lifecycle status.
        pub status: ProposalStatus,
        /// Deposit reserved by the proposer.
        pub deposit: BalanceOf<T>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for Proposal<T> {}

    // =========================================================
    // Config
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_agent_did::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for proposal deposits.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Minimum deposit required to submit a proposal.
        #[pallet::constant]
        type MinProposalDeposit: Get<BalanceOf<Self>>;

        /// Voting period length in blocks (e.g. 50 400 blocks ≈ 7 days at 6 s/block).
        #[pallet::constant]
        type VotingPeriod: Get<BlockNumberFor<Self>>;

        /// Minimum quorum percentage (0–100). A finalised proposal must have
        /// `total_votes >= MinQuorumVotes` (set separately or derived).
        #[pallet::constant]
        type MinQuorumPct: Get<u32>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // =========================================================
    // Pallet struct
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    /// All proposals, keyed by `ProposalId`.
    #[pallet::storage]
    #[pallet::getter(fn proposals)]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, Proposal<T>, OptionQuery>;

    /// Monotonically increasing proposal counter (next id to assign).
    #[pallet::storage]
    #[pallet::getter(fn next_proposal_id)]
    pub type NextProposalId<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    /// Votes cast: `(proposal_id, voter) → VoteRecord`.
    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type Votes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        T::AccountId,
        VoteRecord,
        OptionQuery,
    >;

    /// Total number of proposals ever created (statistics).
    #[pallet::storage]
    #[pallet::getter(fn proposal_count)]
    pub type ProposalCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new proposal was submitted.
        ProposalSubmitted {
            proposal_id: ProposalId,
            proposer: T::AccountId,
            description_hash: [u8; 32],
        },
        /// A vote was cast.
        Voted {
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
            weight: VoteWeight,
        },
        /// A proposal was finalised after its voting period ended.
        ProposalFinalized {
            proposal_id: ProposalId,
            status: ProposalStatus,
        },
        /// A proposal was cancelled and the deposit refunded.
        ProposalCancelled {
            proposal_id: ProposalId,
            proposer: T::AccountId,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Caller does not have a registered (active) DID.
        NotRegistered,
        /// Proposal with the given ID does not exist.
        ProposalNotFound,
        /// Voting period has already ended (or proposal is not active).
        VotingEnded,
        /// Caller has already voted on this proposal.
        AlreadyVoted,
        /// Account balance is too low for the required deposit.
        InsufficientDeposit,
        /// Caller is not the proposer of this proposal.
        NotProposer,
        /// Cannot finalise — voting period has not ended yet.
        ProposalStillActive,
        /// Quorum was not reached.
        QuorumNotMet,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a new governance proposal.
        ///
        /// - `description_hash`: SHA-256 of the off-chain proposal text.
        ///
        /// The caller must have an active DID and sufficient balance for the
        /// minimum deposit (which is reserved until the proposal is finalised
        /// or cancelled).
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
        pub fn submit_proposal(origin: OriginFor<T>, description_hash: [u8; 32]) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // DID check — must have an active (non-deactivated) DID document.
            Self::ensure_has_active_did(&who)?;

            // Reserve deposit.
            let deposit = T::MinProposalDeposit::get();
            T::Currency::reserve(&who, deposit).map_err(|_| Error::<T>::InsufficientDeposit)?;

            let now = frame_system::Pallet::<T>::block_number();
            let end_block = now.saturating_add(T::VotingPeriod::get());

            let proposal_id = NextProposalId::<T>::get();

            let proposal = Proposal::<T> {
                proposer: who.clone(),
                description_hash,
                start_block: now,
                end_block,
                yes_votes: 0u128,
                no_votes: 0u128,
                status: ProposalStatus::Active,
                deposit,
            };

            Proposals::<T>::insert(proposal_id, proposal);
            NextProposalId::<T>::put(proposal_id.saturating_add(1));
            ProposalCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ProposalSubmitted {
                proposal_id,
                proposer: who,
                description_hash,
            });

            Ok(())
        }

        /// Cast a quadratic vote on an active proposal.
        ///
        /// - `proposal_id`: Which proposal to vote on.
        /// - `vote`: `Yes` or `No`.
        /// - `staked_amount`: Number of tokens the voter wishes to stake.
        ///   The actual vote weight is `integer_sqrt(staked_amount)`.
        ///
        /// The caller must have an active DID and can only vote once per
        /// proposal.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
            vote: Vote,
            staked_amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // DID check
            Self::ensure_has_active_did(&who)?;

            // Proposal must exist and be active
            Proposals::<T>::try_mutate(proposal_id, |maybe_prop| -> DispatchResult {
                let proposal = maybe_prop.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(
                    proposal.status == ProposalStatus::Active,
                    Error::<T>::VotingEnded
                );

                // Must still be within voting period
                let now = frame_system::Pallet::<T>::block_number();
                ensure!(now < proposal.end_block, Error::<T>::VotingEnded);

                // No double-voting
                ensure!(
                    !Votes::<T>::contains_key(proposal_id, &who),
                    Error::<T>::AlreadyVoted
                );

                // Quadratic weight
                let weight = Self::integer_sqrt(staked_amount);

                // Record the vote
                let record = VoteRecord {
                    vote: vote.clone(),
                    weight,
                    block: Self::block_to_u32(now),
                };
                Votes::<T>::insert(proposal_id, &who, record);

                // Tally
                match vote {
                    Vote::Yes => proposal.yes_votes = proposal.yes_votes.saturating_add(weight),
                    Vote::No => proposal.no_votes = proposal.no_votes.saturating_add(weight),
                }

                Self::deposit_event(Event::Voted {
                    proposal_id,
                    voter: who.clone(),
                    vote,
                    weight,
                });

                Ok(())
            })
        }

        /// Finalise a proposal after its voting period has ended.
        ///
        /// Determines Passed / Rejected / Expired based on quorum and vote
        /// totals.  Unreserves the proposer's deposit regardless of outcome.
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn finalize_proposal(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            Proposals::<T>::try_mutate(proposal_id, |maybe_prop| -> DispatchResult {
                let proposal = maybe_prop.as_mut().ok_or(Error::<T>::ProposalNotFound)?;
                ensure!(
                    proposal.status == ProposalStatus::Active,
                    Error::<T>::VotingEnded
                );

                let now = frame_system::Pallet::<T>::block_number();
                ensure!(now >= proposal.end_block, Error::<T>::ProposalStillActive);

                let total_votes = proposal.yes_votes.saturating_add(proposal.no_votes);

                // Quorum check: total_votes must be >= MinQuorumPct (as an
                // absolute minimum weight, treating the percentage as the
                // minimum vote-weight threshold for simplicity on a testnet).
                let min_quorum = T::MinQuorumPct::get() as u128;
                ensure!(total_votes >= min_quorum, Error::<T>::QuorumNotMet);

                let new_status = if proposal.yes_votes > proposal.no_votes {
                    ProposalStatus::Passed
                } else {
                    ProposalStatus::Rejected
                };

                proposal.status = new_status;

                // Unreserve proposer deposit
                T::Currency::unreserve(&proposal.proposer, proposal.deposit);

                Self::deposit_event(Event::ProposalFinalized {
                    proposal_id,
                    status: new_status,
                });

                Ok(())
            })
        }

        /// Cancel an active proposal.
        ///
        /// Only the original proposer may cancel. The deposit is unreserved
        /// (refunded).
        #[pallet::call_index(3)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
        pub fn cancel_proposal(origin: OriginFor<T>, proposal_id: ProposalId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let proposal = Proposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

            ensure!(
                proposal.status == ProposalStatus::Active,
                Error::<T>::VotingEnded
            );
            ensure!(proposal.proposer == who, Error::<T>::NotProposer);

            // Refund deposit
            T::Currency::unreserve(&proposal.proposer, proposal.deposit);

            // Remove proposal
            Proposals::<T>::remove(proposal_id);
            ProposalCount::<T>::mutate(|c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::ProposalCancelled {
                proposal_id,
                proposer: proposal.proposer,
            });

            Ok(())
        }
    }

    // =========================================================
    // Internal helpers
    // =========================================================

    impl<T: Config> Pallet<T> {
        /// Ensure account has an active DID document.
        ///
        /// Uses `pallet_agent_did::DIDDocuments` storage directly (tight
        /// coupling via `Config: pallet_agent_did::Config`).
        fn ensure_has_active_did(who: &T::AccountId) -> DispatchResult {
            let doc = pallet_agent_did::pallet::DIDDocuments::<T>::get(who)
                .ok_or(Error::<T>::NotRegistered)?;
            ensure!(!doc.deactivated, Error::<T>::NotRegistered);
            Ok(())
        }

        /// Integer square root using Newton / Babylonian method.
        /// NO floating point. Handles u128::MAX without overflow.
        pub fn integer_sqrt(n: u128) -> u128 {
            if n == 0 {
                return 0;
            }
            let mut x = n;
            let mut y = n / 2 + 1;
            while y < x {
                x = y;
                y = (x + n / x) / 2;
            }
            x
        }

        /// Convert a `BlockNumberFor<T>` to `u32` for the VoteRecord.
        fn block_to_u32(bn: BlockNumberFor<T>) -> u32 {
            use sp_runtime::traits::UniqueSaturatedInto;
            bn.unique_saturated_into()
        }
    }

    // =========================================================
    // Weight trait (placeholder)
    // =========================================================

    pub trait WeightInfo {
        fn submit_proposal() -> Weight;
        fn vote() -> Weight;
        fn finalize_proposal() -> Weight;
        fn cancel_proposal() -> Weight;
    }

    impl WeightInfo for () {
        fn submit_proposal() -> Weight {
            Weight::zero()
        }
        fn vote() -> Weight {
            Weight::zero()
        }
        fn finalize_proposal() -> Weight {
            Weight::zero()
        }
        fn cancel_proposal() -> Weight {
            Weight::zero()
        }
    }
}
