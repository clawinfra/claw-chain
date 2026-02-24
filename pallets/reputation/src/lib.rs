//! # Reputation System Pallet
//!
//! On-chain trust scoring for ClawChain agents.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Tracking agent reputation scores (0-10000 basis points, 0-100.00%)
//! - Recording task completion history
//! - Storing peer reviews from completed tasks
//! - Managing dispute outcomes (wins/losses)
//! - Integration with task-market pallet for automatic reputation updates
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `submit_review` - Leave a review for another agent after task completion
//! - `slash_reputation` - Governance/sudo can slash reputation for misbehavior
//!
//! ### Public Functions (for cross-pallet calls)
//!
//! - `on_task_completed` - Called by task-market when work is approved
//! - `on_task_posted` - Called by task-market when task is created
//! - `on_dispute_resolved` - Called by task-market when dispute is resolved
//! - `get_reputation` - Get current reputation score for an account
//! - `meets_minimum_reputation` - Check if account meets minimum reputation threshold

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;

/// Trait for cross-pallet reputation management.
pub trait ReputationManager<AccountId, Balance> {
    fn on_task_completed(worker: &AccountId, earned: Balance);
    fn on_task_posted(poster: &AccountId, spent: Balance);
    fn on_dispute_resolved(winner: &AccountId, loser: &AccountId);
    fn get_reputation(account: &AccountId) -> u32;
    fn meets_minimum_reputation(account: &AccountId, minimum: u32) -> bool;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, sp_runtime::traits::Saturating};
    use frame_system::pallet_prelude::*;

    /// Type alias for balance (compatible with pallet-balances).
    pub type BalanceOf<T> = <<T as Config>::Currency as frame_support::traits::Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    /// Core reputation information for an agent.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ReputationInfo<T: Config> {
        /// Reputation score in basis points (0-10000 = 0-100.00%).
        pub score: u32,
        /// Total number of tasks completed as a worker.
        pub total_tasks_completed: u32,
        /// Total number of tasks posted.
        pub total_tasks_posted: u32,
        /// Number of successful task completions (approved by poster).
        pub successful_completions: u32,
        /// Number of disputes won.
        pub disputes_won: u32,
        /// Number of disputes lost.
        pub disputes_lost: u32,
        /// Total amount earned from completed tasks.
        pub total_earned: BalanceOf<T>,
        /// Total amount spent on posted tasks.
        pub total_spent: BalanceOf<T>,
        /// Block number of last activity.
        pub last_active: BlockNumberFor<T>,
    }

    impl<T: Config> Default for ReputationInfo<T> {
        fn default() -> Self {
            ReputationInfo {
                score: T::InitialReputation::get(),
                total_tasks_completed: 0,
                total_tasks_posted: 0,
                successful_completions: 0,
                disputes_won: 0,
                disputes_lost: 0,
                total_earned: Zero::zero(),
                total_spent: Zero::zero(),
                last_active: Zero::zero(),
            }
        }
    }

    /// A review left by one agent for another.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Review<T: Config> {
        /// Rating from 1-5 stars.
        pub rating: u8,
        /// Text comment.
        pub comment: BoundedVec<u8, T::MaxCommentLength>,
        /// Which task this review is for.
        pub task_id: u64,
        /// When the review was submitted.
        pub created_at: BlockNumberFor<T>,
    }

    /// Reputation event types for history tracking.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub enum ReputationEvent<T: Config> {
        TaskCompleted {
            task_id: u64,
            earned: BalanceOf<T>,
        },
        TaskPosted {
            task_id: u64,
            spent: BalanceOf<T>,
        },
        ReviewReceived {
            from: T::AccountId,
            rating: u8,
        },
        DisputeWon {
            task_id: u64,
        },
        DisputeLost {
            task_id: u64,
        },
        Slashed {
            amount: u32,
            reason: BoundedVec<u8, T::MaxCommentLength>,
        },
    }

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Currency type for tracking earnings/spending.
        type Currency: frame_support::traits::Currency<Self::AccountId>;

        /// Maximum length of review comments in bytes.
        #[pallet::constant]
        type MaxCommentLength: Get<u32>;

        /// Initial reputation score for new agents (basis points).
        #[pallet::constant]
        type InitialReputation: Get<u32>;

        /// Maximum reputation change per single event (basis points).
        #[pallet::constant]
        type MaxReputationDelta: Get<u32>;

        /// Maximum number of reputation events to store per account.
        #[pallet::constant]
        type MaxHistoryLength: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map from AccountId to their reputation info.
    #[pallet::storage]
    #[pallet::getter(fn reputations)]
    pub type Reputations<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, ReputationInfo<T>, ValueQuery>;

    /// Map from (reviewer, reviewee) to the review they left.
    /// Double map allows querying reviews by reviewer or reviewee.
    #[pallet::storage]
    #[pallet::getter(fn reviews)]
    pub type Reviews<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId, // reviewer
        Blake2_128Concat,
        T::AccountId, // reviewee
        Review<T>,
        OptionQuery,
    >;

    /// Reputation event history for each account (bounded vector).
    #[pallet::storage]
    #[pallet::getter(fn reputation_history)]
    pub type ReputationHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<ReputationEvent<T>, T::MaxHistoryLength>,
        ValueQuery,
    >;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A review was submitted.
        ReviewSubmitted {
            reviewer: T::AccountId,
            reviewee: T::AccountId,
            rating: u8,
            task_id: u64,
        },
        /// Reputation score changed.
        ReputationChanged {
            account: T::AccountId,
            old_score: u32,
            new_score: u32,
        },
        /// Reputation was slashed by governance.
        ReputationSlashed {
            account: T::AccountId,
            amount: u32,
            reason: Vec<u8>,
        },
        /// Task completion recorded.
        TaskCompletionRecorded {
            worker: T::AccountId,
            task_id: u64,
            earned: BalanceOf<T>,
        },
        /// Task posting recorded.
        TaskPostingRecorded {
            poster: T::AccountId,
            task_id: u64,
            spent: BalanceOf<T>,
        },
        /// Dispute outcome recorded.
        DisputeResolved {
            winner: T::AccountId,
            loser: T::AccountId,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// The rating must be between 1 and 5.
        InvalidRating,
        /// Comment exceeds maximum length.
        CommentTooLong,
        /// Cannot review yourself.
        SelfReview,
        /// Reputation history is full.
        HistoryOverflow,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a review for another agent after a task.
        ///
        /// # Arguments
        /// * `reviewee` - The account being reviewed
        /// * `rating` - Star rating (1-5)
        /// * `comment` - Text comment
        /// * `task_id` - Which task this review is for
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 3))]
        pub fn submit_review(
            origin: OriginFor<T>,
            reviewee: T::AccountId,
            rating: u8,
            comment: Vec<u8>,
            task_id: u64,
        ) -> DispatchResult {
            let reviewer = ensure_signed(origin)?;

            // Validation
            ensure!(rating >= 1 && rating <= 5, Error::<T>::InvalidRating);
            ensure!(reviewer != reviewee, Error::<T>::SelfReview);
            let bounded_comment: BoundedVec<u8, T::MaxCommentLength> =
                comment.try_into().map_err(|_| Error::<T>::CommentTooLong)?;

            let current_block = <frame_system::Pallet<T>>::block_number();

            // Store the review
            let review = Review::<T> {
                rating,
                comment: bounded_comment,
                task_id,
                created_at: current_block,
            };
            Reviews::<T>::insert(&reviewer, &reviewee, review);

            // Update reviewee's reputation based on rating
            // 1 star = +100, 2 stars = +200, ... 5 stars = +500
            let delta = (rating as u32) * 100;
            Self::apply_reputation_change(&reviewee, delta as i32, true);

            // Record event in history
            let event = ReputationEvent::<T>::ReviewReceived {
                from: reviewer.clone(),
                rating,
            };
            Self::add_to_history(&reviewee, event);

            Self::deposit_event(Event::ReviewSubmitted {
                reviewer,
                reviewee,
                rating,
                task_id,
            });

            Ok(())
        }

        /// Slash an agent's reputation (governance/sudo only).
        ///
        /// # Arguments
        /// * `account` - The account to slash
        /// * `amount` - Amount to subtract from reputation (basis points)
        /// * `reason` - Reason for the slash
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 2))]
        pub fn slash_reputation(
            origin: OriginFor<T>,
            account: T::AccountId,
            amount: u32,
            reason: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let bounded_reason: BoundedVec<u8, T::MaxCommentLength> = reason
                .clone()
                .try_into()
                .map_err(|_| Error::<T>::CommentTooLong)?;

            // Apply negative delta
            Self::apply_reputation_change(&account, -(amount as i32), false);

            // Record event
            let event = ReputationEvent::<T>::Slashed {
                amount,
                reason: bounded_reason,
            };
            Self::add_to_history(&account, event);

            Self::deposit_event(Event::ReputationSlashed {
                account,
                amount,
                reason,
            });

            Ok(())
        }
    }

    // ========== Internal Functions ==========

    impl<T: Config> Pallet<T> {
        /// Apply a reputation change (clamped to 0-10000).
        fn apply_reputation_change(account: &T::AccountId, delta: i32, limit_delta: bool) {
            Reputations::<T>::mutate(account, |rep| {
                let old_score = rep.score;

                // Clamp delta if requested
                let clamped_delta = if limit_delta {
                    let max = T::MaxReputationDelta::get() as i32;
                    delta.clamp(-max, max)
                } else {
                    delta
                };

                let new_score = if clamped_delta >= 0 {
                    old_score.saturating_add(clamped_delta as u32).min(10000)
                } else {
                    old_score
                        .saturating_sub(clamped_delta.unsigned_abs())
                        .max(0)
                };

                rep.score = new_score;
                rep.last_active = <frame_system::Pallet<T>>::block_number();

                Self::deposit_event(Event::ReputationChanged {
                    account: account.clone(),
                    old_score,
                    new_score,
                });
            });
        }

        /// Add an event to reputation history (removes oldest if full).
        fn add_to_history(account: &T::AccountId, event: ReputationEvent<T>) {
            ReputationHistory::<T>::mutate(account, |history| {
                // If full, remove the oldest event (FIFO)
                if history.len() >= T::MaxHistoryLength::get() as usize {
                    history.remove(0);
                }
                // Add the new event (ignore error if somehow still full)
                let _ = history.try_push(event);
            });
        }
    }

    // ========== ReputationManager Trait Implementation ==========

    impl<T: Config> ReputationManager<T::AccountId, BalanceOf<T>> for Pallet<T> {
        fn on_task_completed(worker: &T::AccountId, earned: BalanceOf<T>) {
            Reputations::<T>::mutate(worker, |rep| {
                rep.total_tasks_completed = rep.total_tasks_completed.saturating_add(1);
                rep.successful_completions = rep.successful_completions.saturating_add(1);
                rep.total_earned = rep.total_earned.saturating_add(earned);
                rep.last_active = <frame_system::Pallet<T>>::block_number();
            });

            // Note: Actual reputation boost happens when review is submitted
            // This just records the completion
        }

        fn on_task_posted(poster: &T::AccountId, spent: BalanceOf<T>) {
            Reputations::<T>::mutate(poster, |rep| {
                rep.total_tasks_posted = rep.total_tasks_posted.saturating_add(1);
                rep.total_spent = rep.total_spent.saturating_add(spent);
                rep.last_active = <frame_system::Pallet<T>>::block_number();
            });
        }

        fn on_dispute_resolved(winner: &T::AccountId, loser: &T::AccountId) {
            // Winner gains +200 reputation
            Self::apply_reputation_change(winner, 200, false);
            Reputations::<T>::mutate(winner, |rep| {
                rep.disputes_won = rep.disputes_won.saturating_add(1);
            });

            // Loser loses -500 reputation
            Self::apply_reputation_change(loser, -500, false);
            Reputations::<T>::mutate(loser, |rep| {
                rep.disputes_lost = rep.disputes_lost.saturating_add(1);
            });

            Self::deposit_event(Event::DisputeResolved {
                winner: winner.clone(),
                loser: loser.clone(),
            });
        }

        fn get_reputation(account: &T::AccountId) -> u32 {
            Reputations::<T>::get(account).score
        }

        fn meets_minimum_reputation(account: &T::AccountId, minimum: u32) -> bool {
            Self::get_reputation(account) >= minimum
        }
    }

    // ========== Weight Info Trait ==========

    pub trait WeightInfo {
        fn submit_review() -> Weight;
        fn slash_reputation() -> Weight;
    }

    impl WeightInfo for () {
        fn submit_review() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn slash_reputation() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
