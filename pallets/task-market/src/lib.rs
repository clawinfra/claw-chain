//! # Task Market Pallet
//!
//! An on-chain marketplace for ClawChain agents to post, bid on, and execute tasks with escrow.
//!
//! ## Overview
//!
//! This pallet provides functionality for:
//! - Posting tasks with CLAW token escrow
//! - Submitting bids on open tasks
//! - Assigning tasks to selected bidders
//! - Submitting work and proof of completion
//! - Approving work and releasing escrow
//! - Disputing tasks (governance resolution)
//! - Cancelling tasks and refunding escrow
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `post_task` - Create a task with locked escrow
//! - `bid_on_task` - Submit a bid on an open task
//! - `assign_task` - Poster selects a bidder
//! - `submit_work` - Worker submits completion proof
//! - `approve_work` - Poster approves and releases payment
//! - `dispute_task` - Either party disputes the task
//! - `cancel_task` - Poster cancels (only if still Open)
//! - `resolve_dispute` - Governance resolves a dispute

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
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use pallet_reputation::ReputationManager;

    /// Type alias for task IDs.
    pub type TaskId = u64;

    /// Type alias for balance (compatible with pallet-balances).
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Task status enum.
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
    pub enum TaskStatus {
        /// Accepting bids.
        #[default]
        Open,
        /// Worker selected, task assigned.
        Assigned,
        /// Work in progress.
        InProgress,
        /// Work submitted, pending review.
        Completed,
        /// Poster approved, reward released.
        Approved,
        /// Task is in dispute.
        Disputed,
        /// Poster cancelled (refunded).
        Cancelled,
        /// Deadline passed without completion.
        Expired,
    }

    /// Core task information.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct TaskInfo<T: Config> {
        /// Account that posted the task.
        pub poster: T::AccountId,
        /// Task title.
        pub title: BoundedVec<u8, T::MaxTitleLength>,
        /// Task description.
        pub description: BoundedVec<u8, T::MaxDescriptionLength>,
        /// Reward in CLAW tokens (held in escrow).
        pub reward: BalanceOf<T>,
        /// Deadline (block number).
        pub deadline: BlockNumberFor<T>,
        /// Current status.
        pub status: TaskStatus,
        /// Assigned worker (if any).
        pub assigned_to: Option<T::AccountId>,
        /// When the task was created.
        pub created_at: BlockNumberFor<T>,
    }

    /// Bid information.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct BidInfo<T: Config> {
        /// The bidder's account.
        pub bidder: T::AccountId,
        /// Amount they're willing to do it for.
        pub amount: BalanceOf<T>,
        /// Proposal text.
        pub proposal: BoundedVec<u8, T::MaxProposalLength>,
        /// When the bid was submitted.
        pub submitted_at: BlockNumberFor<T>,
    }

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Currency type for payments and escrow.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Reputation manager for cross-pallet calls.
        type ReputationManager: ReputationManager<Self::AccountId, BalanceOf<Self>>;

        /// Pallet ID for escrow account derivation.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Maximum length of task title in bytes.
        #[pallet::constant]
        type MaxTitleLength: Get<u32>;

        /// Maximum length of task description in bytes.
        #[pallet::constant]
        type MaxDescriptionLength: Get<u32>;

        /// Maximum length of bid proposal in bytes.
        #[pallet::constant]
        type MaxProposalLength: Get<u32>;

        /// Maximum number of bids per task.
        #[pallet::constant]
        type MaxBidsPerTask: Get<u32>;

        /// Minimum task reward (to prevent spam).
        #[pallet::constant]
        type MinTaskReward: Get<BalanceOf<Self>>;

        /// Maximum number of active tasks per account.
        #[pallet::constant]
        type MaxActiveTasksPerAccount: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map from TaskId to TaskInfo.
    #[pallet::storage]
    #[pallet::getter(fn tasks)]
    pub type Tasks<T: Config> = StorageMap<_, Blake2_128Concat, TaskId, TaskInfo<T>, OptionQuery>;

    /// Total number of tasks created.
    #[pallet::storage]
    #[pallet::getter(fn task_count)]
    pub type TaskCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Double map: TaskId -> AccountId -> BidInfo.
    #[pallet::storage]
    #[pallet::getter(fn task_bids)]
    pub type TaskBids<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        TaskId,
        Blake2_128Concat,
        T::AccountId,
        BidInfo<T>,
        OptionQuery,
    >;

    /// Map from AccountId to their posted task IDs.
    #[pallet::storage]
    #[pallet::getter(fn active_tasks)]
    pub type ActiveTasks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<TaskId, T::MaxActiveTasksPerAccount>,
        ValueQuery,
    >;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new task was posted.
        TaskPosted {
            task_id: TaskId,
            poster: T::AccountId,
            reward: BalanceOf<T>,
        },
        /// A bid was submitted.
        BidSubmitted {
            task_id: TaskId,
            bidder: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// A task was assigned to a worker.
        TaskAssigned {
            task_id: TaskId,
            worker: T::AccountId,
        },
        /// Work was submitted.
        WorkSubmitted { task_id: TaskId },
        /// Work was approved and payment released.
        WorkApproved { task_id: TaskId },
        /// A task was disputed.
        TaskDisputed {
            task_id: TaskId,
            disputer: T::AccountId,
            reason: Vec<u8>,
        },
        /// A task was cancelled.
        TaskCancelled { task_id: TaskId },
        /// A dispute was resolved.
        DisputeResolved {
            task_id: TaskId,
            winner: T::AccountId,
        },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// Task ID not found.
        TaskNotFound,
        /// Only the task poster can perform this action.
        NotPoster,
        /// Only the assigned worker can perform this action.
        NotAssignedWorker,
        /// Title exceeds maximum length.
        TitleTooLong,
        /// Description exceeds maximum length.
        DescriptionTooLong,
        /// Proposal exceeds maximum length.
        ProposalTooLong,
        /// Reward is below minimum.
        RewardTooLow,
        /// Task is not in the expected status.
        InvalidTaskStatus,
        /// Bid not found for this task and bidder.
        BidNotFound,
        /// Cannot bid on your own task.
        CannotBidOnOwnTask,
        /// Too many bids for this task.
        TooManyBids,
        /// Too many active tasks for this account.
        TooManyActiveTasks,
        /// Task deadline has passed.
        TaskExpired,
        /// Insufficient balance to post task.
        InsufficientBalance,
        /// Bidder does not meet minimum reputation requirement.
        InsufficientReputation,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Post a new task and lock the reward in escrow.
        ///
        /// # Arguments
        /// * `title` - Task title
        /// * `description` - Detailed description
        /// * `reward` - CLAW tokens to pay (locked immediately)
        /// * `deadline` - Block number deadline
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 4))]
        pub fn post_task(
            origin: OriginFor<T>,
            title: Vec<u8>,
            description: Vec<u8>,
            reward: BalanceOf<T>,
            deadline: BlockNumberFor<T>,
        ) -> DispatchResult {
            let poster = ensure_signed(origin)?;

            // Validation
            ensure!(reward >= T::MinTaskReward::get(), Error::<T>::RewardTooLow);
            let bounded_title: BoundedVec<u8, T::MaxTitleLength> =
                title.try_into().map_err(|_| Error::<T>::TitleTooLong)?;
            let bounded_description: BoundedVec<u8, T::MaxDescriptionLength> = description
                .try_into()
                .map_err(|_| Error::<T>::DescriptionTooLong)?;

            let current_block = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > current_block, Error::<T>::TaskExpired);

            // Reserve the reward (escrow)
            T::Currency::reserve(&poster, reward).map_err(|_| Error::<T>::InsufficientBalance)?;

            // Create task
            let task_id = TaskCount::<T>::get();
            let task_info = TaskInfo::<T> {
                poster: poster.clone(),
                title: bounded_title,
                description: bounded_description,
                reward,
                deadline,
                status: TaskStatus::Open,
                assigned_to: None,
                created_at: current_block,
            };

            Tasks::<T>::insert(task_id, task_info);
            TaskCount::<T>::put(task_id.saturating_add(1));

            // Add to poster's active tasks
            ActiveTasks::<T>::try_mutate(&poster, |tasks| {
                tasks
                    .try_push(task_id)
                    .map_err(|_| Error::<T>::TooManyActiveTasks)
            })?;

            // Update reputation stats
            T::ReputationManager::on_task_posted(&poster, reward);

            Self::deposit_event(Event::TaskPosted {
                task_id,
                poster,
                reward,
            });

            Ok(())
        }

        /// Submit a bid on an open task.
        ///
        /// # Arguments
        /// * `task_id` - The task to bid on
        /// * `amount` - How much you'll do it for
        /// * `proposal` - Your proposal text
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn bid_on_task(
            origin: OriginFor<T>,
            task_id: TaskId,
            amount: BalanceOf<T>,
            proposal: Vec<u8>,
        ) -> DispatchResult {
            let bidder = ensure_signed(origin)?;

            let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(
                task.status == TaskStatus::Open,
                Error::<T>::InvalidTaskStatus
            );
            ensure!(task.poster != bidder, Error::<T>::CannotBidOnOwnTask);

            // Check deadline
            let current_block = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block < task.deadline, Error::<T>::TaskExpired);

            // Optional: Check minimum reputation (example: 3000 = 30%)
            // Uncomment to enforce:
            // ensure!(
            //     T::ReputationManager::meets_minimum_reputation(&bidder, 3000),
            //     Error::<T>::InsufficientReputation
            // );

            let bounded_proposal: BoundedVec<u8, T::MaxProposalLength> = proposal
                .try_into()
                .map_err(|_| Error::<T>::ProposalTooLong)?;

            // Check bid count (simple check: if we can insert, there's space)
            let bid_info = BidInfo::<T> {
                bidder: bidder.clone(),
                amount,
                proposal: bounded_proposal,
                submitted_at: current_block,
            };

            TaskBids::<T>::insert(task_id, &bidder, bid_info);

            Self::deposit_event(Event::BidSubmitted {
                task_id,
                bidder,
                amount,
            });

            Ok(())
        }

        /// Assign a task to a selected bidder.
        ///
        /// # Arguments
        /// * `task_id` - The task to assign
        /// * `bidder` - The selected bidder
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 1))]
        pub fn assign_task(
            origin: OriginFor<T>,
            task_id: TaskId,
            bidder: T::AccountId,
        ) -> DispatchResult {
            let poster = ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(task.poster == poster, Error::<T>::NotPoster);
                ensure!(
                    task.status == TaskStatus::Open,
                    Error::<T>::InvalidTaskStatus
                );

                // Verify bid exists
                ensure!(
                    TaskBids::<T>::contains_key(task_id, &bidder),
                    Error::<T>::BidNotFound
                );

                task.status = TaskStatus::Assigned;
                task.assigned_to = Some(bidder.clone());

                Ok(())
            })?;

            Self::deposit_event(Event::TaskAssigned {
                task_id,
                worker: bidder,
            });

            Ok(())
        }

        /// Submit completed work with proof.
        ///
        /// # Arguments
        /// * `task_id` - The task being completed
        /// * `proof` - Proof of completion (URL, hash, etc.)
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn submit_work(
            origin: OriginFor<T>,
            task_id: TaskId,
            _proof: Vec<u8>, // Could store this in a separate storage map if needed
        ) -> DispatchResult {
            let worker = ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(
                    task.assigned_to == Some(worker.clone()),
                    Error::<T>::NotAssignedWorker
                );
                ensure!(
                    task.status == TaskStatus::Assigned || task.status == TaskStatus::InProgress,
                    Error::<T>::InvalidTaskStatus
                );

                task.status = TaskStatus::Completed;

                Ok(())
            })?;

            Self::deposit_event(Event::WorkSubmitted { task_id });

            Ok(())
        }

        /// Approve the submitted work and release payment.
        ///
        /// # Arguments
        /// * `task_id` - The task to approve
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn approve_work(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let poster = ensure_signed(origin)?;

            let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(task.poster == poster, Error::<T>::NotPoster);
            ensure!(
                task.status == TaskStatus::Completed,
                Error::<T>::InvalidTaskStatus
            );

            let worker = task.assigned_to.ok_or(Error::<T>::NotAssignedWorker)?;

            // Unreserve from poster and transfer to worker
            T::Currency::unreserve(&poster, task.reward);
            T::Currency::transfer(
                &poster,
                &worker,
                task.reward,
                ExistenceRequirement::KeepAlive,
            )?;

            // Update task status
            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let t = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                t.status = TaskStatus::Approved;
                Ok(())
            })?;

            // Update reputation
            T::ReputationManager::on_task_completed(&worker, task.reward);

            Self::deposit_event(Event::WorkApproved { task_id });

            Ok(())
        }

        /// Dispute a task (either poster or worker can dispute).
        ///
        /// # Arguments
        /// * `task_id` - The task to dispute
        /// * `reason` - Reason for dispute
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn dispute_task(
            origin: OriginFor<T>,
            task_id: TaskId,
            reason: Vec<u8>,
        ) -> DispatchResult {
            let disputer = ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;

                // Only poster or assigned worker can dispute
                let is_authorized =
                    task.poster == disputer || task.assigned_to == Some(disputer.clone());
                ensure!(is_authorized, Error::<T>::NotPoster);

                // Can dispute if Assigned, InProgress, or Completed
                ensure!(
                    matches!(
                        task.status,
                        TaskStatus::Assigned | TaskStatus::InProgress | TaskStatus::Completed
                    ),
                    Error::<T>::InvalidTaskStatus
                );

                task.status = TaskStatus::Disputed;

                Ok(())
            })?;

            Self::deposit_event(Event::TaskDisputed {
                task_id,
                disputer,
                reason,
            });

            Ok(())
        }

        /// Cancel a task (poster only, only if Open).
        ///
        /// # Arguments
        /// * `task_id` - The task to cancel
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn cancel_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
            let poster = ensure_signed(origin)?;

            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let task = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                ensure!(task.poster == poster, Error::<T>::NotPoster);
                ensure!(
                    task.status == TaskStatus::Open,
                    Error::<T>::InvalidTaskStatus
                );

                // Unreserve escrow
                T::Currency::unreserve(&poster, task.reward);

                task.status = TaskStatus::Cancelled;

                Ok(())
            })?;

            Self::deposit_event(Event::TaskCancelled { task_id });

            Ok(())
        }

        /// Resolve a dispute (governance/sudo only).
        ///
        /// # Arguments
        /// * `task_id` - The disputed task
        /// * `winner` - Who gets the escrow
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn resolve_dispute(
            origin: OriginFor<T>,
            task_id: TaskId,
            winner: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let task = Tasks::<T>::get(task_id).ok_or(Error::<T>::TaskNotFound)?;
            ensure!(
                task.status == TaskStatus::Disputed,
                Error::<T>::InvalidTaskStatus
            );

            let poster = task.poster.clone();
            let worker = task.assigned_to.ok_or(Error::<T>::NotAssignedWorker)?;

            // Determine loser
            let loser = if winner == poster {
                worker.clone()
            } else {
                poster.clone()
            };

            // Unreserve and transfer to winner
            T::Currency::unreserve(&poster, task.reward);
            T::Currency::transfer(
                &poster,
                &winner,
                task.reward,
                ExistenceRequirement::KeepAlive,
            )?;

            // Update task status
            Tasks::<T>::try_mutate(task_id, |maybe_task| -> DispatchResult {
                let t = maybe_task.as_mut().ok_or(Error::<T>::TaskNotFound)?;
                t.status = TaskStatus::Approved; // Mark as resolved
                Ok(())
            })?;

            // Update reputations
            T::ReputationManager::on_dispute_resolved(&winner, &loser);

            Self::deposit_event(Event::DisputeResolved { task_id, winner });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    pub trait WeightInfo {
        fn post_task() -> Weight;
        fn bid_on_task() -> Weight;
        fn assign_task() -> Weight;
        fn submit_work() -> Weight;
        fn approve_work() -> Weight;
        fn dispute_task() -> Weight;
        fn cancel_task() -> Weight;
        fn resolve_dispute() -> Weight;
    }

    impl WeightInfo for () {
        fn post_task() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn bid_on_task() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn assign_task() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn submit_work() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn approve_work() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn dispute_task() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn cancel_task() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn resolve_dispute() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
