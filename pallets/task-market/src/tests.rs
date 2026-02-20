//! Unit tests for the Task Market pallet.

use crate::{self as pallet_task_market, pallet::*, *};
use frame_support::{assert_noop, assert_ok, parameter_types, PalletId};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Reputation: pallet_reputation,
        TaskMarket: pallet_task_market,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MaxCommentLength: u32 = 256;
    pub const InitialReputation: u32 = 5000;
    pub const MaxReputationDelta: u32 = 500;
    pub const MaxHistoryLength: u32 = 100;
}

impl pallet_reputation::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type MaxCommentLength = MaxCommentLength;
    type InitialReputation = InitialReputation;
    type MaxReputationDelta = MaxReputationDelta;
    type MaxHistoryLength = MaxHistoryLength;
}

parameter_types! {
    pub const TaskMarketPalletId: PalletId = PalletId(*b"taskmark");
    pub const MaxTitleLength: u32 = 128;
    pub const MaxDescriptionLength: u32 = 1024;
    pub const MaxProposalLength: u32 = 512;
    pub const MaxBidsPerTask: u32 = 20;
    pub const MinTaskReward: u64 = 100;
    pub const MaxActiveTasksPerAccount: u32 = 50;
}

impl pallet_task_market::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type ReputationManager = Reputation;
    type PalletId = TaskMarketPalletId;
    type MaxTitleLength = MaxTitleLength;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MaxProposalLength = MaxProposalLength;
    type MaxBidsPerTask = MaxBidsPerTask;
    type MinTaskReward = MinTaskReward;
    type MaxActiveTasksPerAccount = MaxActiveTasksPerAccount;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 10000), (2, 10000), (3, 10000), (4, 10000), (5, 50)],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Helper: post a standard task
fn post_default_task(poster: u64) {
    assert_ok!(TaskMarket::post_task(
        RuntimeOrigin::signed(poster),
        b"Test Task".to_vec(),
        b"Description".to_vec(),
        1000,
        1000
    ));
}

// Helper: post task, bid, assign
fn setup_assigned_task(poster: u64, worker: u64) -> TaskId {
    post_default_task(poster);
    let task_id = TaskMarket::task_count() - 1;
    assert_ok!(TaskMarket::bid_on_task(
        RuntimeOrigin::signed(worker),
        task_id,
        800,
        b"Proposal".to_vec()
    ));
    assert_ok!(TaskMarket::assign_task(
        RuntimeOrigin::signed(poster),
        task_id,
        worker
    ));
    task_id
}

// Helper: full workflow up to completion
fn setup_completed_task(poster: u64, worker: u64) -> TaskId {
    let task_id = setup_assigned_task(poster, worker);
    assert_ok!(TaskMarket::submit_work(
        RuntimeOrigin::signed(worker),
        task_id,
        b"proof".to_vec()
    ));
    task_id
}

// ========== Post Task Tests ==========

#[test]
fn post_task_works() {
    new_test_ext().execute_with(|| {
        let poster = 1;
        let title = b"Build a website".to_vec();
        let description = b"Need a React website".to_vec();
        let reward = 1000u64;
        let deadline = 1000u64;

        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(poster),
            title.clone(),
            description,
            reward,
            deadline
        ));

        let task = TaskMarket::tasks(0).unwrap();
        assert_eq!(task.poster, poster);
        assert_eq!(task.reward, reward);
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.deadline, deadline);
        assert_eq!(task.assigned_to, None);
        assert_eq!(task.created_at, 1);

        // Check escrow was reserved
        assert_eq!(Balances::reserved_balance(poster), reward);

        // Check reputation stats updated
        let rep = Reputation::reputations(poster);
        assert_eq!(rep.total_tasks_posted, 1);
        assert_eq!(rep.total_spent, reward);

        // Check task count
        assert_eq!(TaskMarket::task_count(), 1);
    });
}

#[test]
fn post_task_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(1),
            b"Task".to_vec(),
            b"Desc".to_vec(),
            1000,
            1000
        ));

        System::assert_has_event(
            Event::<Test>::TaskPosted {
                task_id: 0,
                poster: 1,
                reward: 1000,
            }
            .into(),
        );
    });
}

#[test]
fn post_task_fails_if_reward_too_low() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                b"Task".to_vec(),
                b"Description".to_vec(),
                50, // Below MinTaskReward (100)
                1000
            ),
            Error::<Test>::RewardTooLow
        );
    });
}

#[test]
fn post_task_fails_if_reward_zero() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                b"Task".to_vec(),
                b"Description".to_vec(),
                0,
                1000
            ),
            Error::<Test>::RewardTooLow
        );
    });
}

#[test]
fn post_task_at_minimum_reward() {
    new_test_ext().execute_with(|| {
        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(1),
            b"Task".to_vec(),
            b"Desc".to_vec(),
            100, // Exactly MinTaskReward
            1000
        ));
    });
}

#[test]
fn post_task_fails_if_title_too_long() {
    new_test_ext().execute_with(|| {
        let long_title = vec![b'x'; 129]; // Exceeds MaxTitleLength of 128
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                long_title,
                b"Description".to_vec(),
                1000,
                1000
            ),
            Error::<Test>::TitleTooLong
        );
    });
}

#[test]
fn post_task_max_title_length() {
    new_test_ext().execute_with(|| {
        let title = vec![b'x'; 128]; // Exactly MaxTitleLength
        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(1),
            title,
            b"Description".to_vec(),
            1000,
            1000
        ));
    });
}

#[test]
fn post_task_fails_if_description_too_long() {
    new_test_ext().execute_with(|| {
        let long_desc = vec![b'x'; 1025]; // Exceeds MaxDescriptionLength of 1024
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                b"Task".to_vec(),
                long_desc,
                1000,
                1000
            ),
            Error::<Test>::DescriptionTooLong
        );
    });
}

#[test]
fn post_task_fails_if_deadline_in_past() {
    new_test_ext().execute_with(|| {
        // Current block is 1, deadline of 0 is in the past
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                b"Task".to_vec(),
                b"Desc".to_vec(),
                1000,
                0
            ),
            Error::<Test>::TaskExpired
        );
    });
}

#[test]
fn post_task_fails_if_deadline_is_current_block() {
    new_test_ext().execute_with(|| {
        // Deadline must be strictly greater than current block
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(1),
                b"Task".to_vec(),
                b"Desc".to_vec(),
                1000,
                1 // Same as current block
            ),
            Error::<Test>::TaskExpired
        );
    });
}

#[test]
fn post_task_fails_if_insufficient_balance() {
    new_test_ext().execute_with(|| {
        // Account 5 has only 50 tokens
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::signed(5),
                b"Task".to_vec(),
                b"Desc".to_vec(),
                1000,
                1000
            ),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn post_task_unsigned_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::post_task(
                RuntimeOrigin::none(),
                b"Task".to_vec(),
                b"Desc".to_vec(),
                1000,
                1000
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn post_task_active_tasks_tracking() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        post_default_task(1);

        let active = TaskMarket::active_tasks(1);
        assert_eq!(active.len(), 2);
        assert_eq!(active[0], 0);
        assert_eq!(active[1], 1);
    });
}

#[test]
fn task_count_increments() {
    new_test_ext().execute_with(|| {
        assert_eq!(TaskMarket::task_count(), 0);

        post_default_task(1);
        assert_eq!(TaskMarket::task_count(), 1);

        post_default_task(1);
        assert_eq!(TaskMarket::task_count(), 2);
    });
}

// ========== Bid on Task Tests ==========

#[test]
fn bid_on_task_works() {
    new_test_ext().execute_with(|| {
        let poster = 1;
        let bidder = 2;

        post_default_task(poster);

        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(bidder),
            0,
            800,
            b"I can do this".to_vec()
        ));

        let bid = TaskMarket::task_bids(0, bidder).unwrap();
        assert_eq!(bid.bidder, bidder);
        assert_eq!(bid.amount, 800);
        assert_eq!(bid.submitted_at, 1);
    });
}

#[test]
fn bid_on_task_emits_event() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Proposal".to_vec()
        ));

        System::assert_has_event(
            Event::<Test>::BidSubmitted {
                task_id: 0,
                bidder: 2,
                amount: 800,
            }
            .into(),
        );
    });
}

#[test]
fn cannot_bid_on_own_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(1),
                0,
                800,
                b"Proposal".to_vec()
            ),
            Error::<Test>::CannotBidOnOwnTask
        );
    });
}

#[test]
fn bid_on_nonexistent_task_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(2),
                999,
                800,
                b"Proposal".to_vec()
            ),
            Error::<Test>::TaskNotFound
        );
    });
}

#[test]
fn bid_fails_if_task_not_open() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Task is now Assigned, can't bid
        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(3),
                task_id,
                800,
                b"Proposal".to_vec()
            ),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn bid_fails_if_task_expired() {
    new_test_ext().execute_with(|| {
        // Post task with deadline 10
        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(1),
            b"Task".to_vec(),
            b"Desc".to_vec(),
            1000,
            10
        ));

        // Advance past deadline
        System::set_block_number(11);

        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(2),
                0,
                800,
                b"Proposal".to_vec()
            ),
            Error::<Test>::TaskExpired
        );
    });
}

#[test]
fn bid_proposal_too_long_fails() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        let long_proposal = vec![b'x'; 513]; // Exceeds MaxProposalLength of 512
        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(2),
                0,
                800,
                long_proposal
            ),
            Error::<Test>::ProposalTooLong
        );
    });
}

#[test]
fn multiple_bids_on_same_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Bid 1".to_vec()
        ));
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(3),
            0,
            750,
            b"Bid 2".to_vec()
        ));

        assert!(TaskMarket::task_bids(0, 2).is_some());
        assert!(TaskMarket::task_bids(0, 3).is_some());
    });
}

#[test]
fn bid_zero_amount() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        // Zero bid amount is allowed (pallet doesn't enforce minimum)
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            0,
            b"Free work".to_vec()
        ));
    });
}

// ========== Assign Task Tests ==========

#[test]
fn assign_task_works() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Proposal".to_vec()
        ));

        assert_ok!(TaskMarket::assign_task(RuntimeOrigin::signed(1), 0, 2));

        let task = TaskMarket::tasks(0).unwrap();
        assert_eq!(task.status, TaskStatus::Assigned);
        assert_eq!(task.assigned_to, Some(2));
    });
}

#[test]
fn assign_task_emits_event() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Proposal".to_vec()
        ));

        assert_ok!(TaskMarket::assign_task(RuntimeOrigin::signed(1), 0, 2));

        System::assert_has_event(
            Event::<Test>::TaskAssigned {
                task_id: 0,
                worker: 2,
            }
            .into(),
        );
    });
}

#[test]
fn only_poster_can_assign() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Proposal".to_vec()
        ));

        // Account 3 is not the poster
        assert_noop!(
            TaskMarket::assign_task(RuntimeOrigin::signed(3), 0, 2),
            Error::<Test>::NotPoster
        );
    });
}

#[test]
fn assign_fails_without_bid() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        // Try to assign account 2, but they haven't bid
        assert_noop!(
            TaskMarket::assign_task(RuntimeOrigin::signed(1), 0, 2),
            Error::<Test>::BidNotFound
        );
    });
}

#[test]
fn assign_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::assign_task(RuntimeOrigin::signed(1), 999, 2),
            Error::<Test>::TaskNotFound
        );
    });
}

#[test]
fn assign_fails_if_already_assigned() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            0,
            800,
            b"Proposal".to_vec()
        ));
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(3),
            0,
            700,
            b"Proposal 2".to_vec()
        ));

        assert_ok!(TaskMarket::assign_task(RuntimeOrigin::signed(1), 0, 2));

        // Can't assign again
        assert_noop!(
            TaskMarket::assign_task(RuntimeOrigin::signed(1), 0, 3),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

// ========== Submit Work Tests ==========

#[test]
fn submit_work_works() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        assert_ok!(TaskMarket::submit_work(
            RuntimeOrigin::signed(2),
            task_id,
            b"https://proof.com".to_vec()
        ));

        let task = TaskMarket::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
    });
}

#[test]
fn submit_work_emits_event() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        assert_ok!(TaskMarket::submit_work(
            RuntimeOrigin::signed(2),
            task_id,
            b"proof".to_vec()
        ));

        System::assert_has_event(Event::<Test>::WorkSubmitted { task_id }.into());
    });
}

#[test]
fn submit_work_fails_for_non_worker() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Account 3 is not the assigned worker
        assert_noop!(
            TaskMarket::submit_work(
                RuntimeOrigin::signed(3),
                task_id,
                b"proof".to_vec()
            ),
            Error::<Test>::NotAssignedWorker
        );
    });
}

#[test]
fn submit_work_fails_for_poster() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Poster can't submit work
        assert_noop!(
            TaskMarket::submit_work(
                RuntimeOrigin::signed(1),
                task_id,
                b"proof".to_vec()
            ),
            Error::<Test>::NotAssignedWorker
        );
    });
}

#[test]
fn submit_work_fails_if_task_open() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        // Task is Open, no worker assigned
        assert_noop!(
            TaskMarket::submit_work(
                RuntimeOrigin::signed(2),
                0,
                b"proof".to_vec()
            ),
            Error::<Test>::NotAssignedWorker
        );
    });
}

#[test]
fn submit_work_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::submit_work(
                RuntimeOrigin::signed(1),
                999,
                b"proof".to_vec()
            ),
            Error::<Test>::TaskNotFound
        );
    });
}

// ========== Approve Work Tests ==========

#[test]
fn approve_work_works() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        let worker_balance_before = Balances::free_balance(2);

        assert_ok!(TaskMarket::approve_work(RuntimeOrigin::signed(1), task_id));

        let task = TaskMarket::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Approved);

        // Check payment transferred
        assert_eq!(Balances::free_balance(2), worker_balance_before + 1000);
        assert_eq!(Balances::free_balance(1), 9000);
        assert_eq!(Balances::reserved_balance(1), 0);

        // Check reputation updated
        let rep = Reputation::reputations(2);
        assert_eq!(rep.total_tasks_completed, 1);
        assert_eq!(rep.successful_completions, 1);
        assert_eq!(rep.total_earned, 1000);
    });
}

#[test]
fn approve_work_emits_event() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        assert_ok!(TaskMarket::approve_work(RuntimeOrigin::signed(1), task_id));

        System::assert_has_event(Event::<Test>::WorkApproved { task_id }.into());
    });
}

#[test]
fn approve_work_fails_for_non_poster() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        assert_noop!(
            TaskMarket::approve_work(RuntimeOrigin::signed(3), task_id),
            Error::<Test>::NotPoster
        );
    });
}

#[test]
fn approve_work_fails_if_not_completed() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Task is Assigned, not Completed
        assert_noop!(
            TaskMarket::approve_work(RuntimeOrigin::signed(1), task_id),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn approve_work_fails_for_open_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_noop!(
            TaskMarket::approve_work(RuntimeOrigin::signed(1), 0),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn approve_work_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::approve_work(RuntimeOrigin::signed(1), 999),
            Error::<Test>::TaskNotFound
        );
    });
}

// ========== Cancel Task Tests ==========

#[test]
fn cancel_task_works() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_eq!(Balances::reserved_balance(1), 1000);

        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));

        assert_eq!(Balances::reserved_balance(1), 0);
        assert_eq!(Balances::free_balance(1), 10000); // Full refund

        let task = TaskMarket::tasks(0).unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    });
}

#[test]
fn cancel_task_emits_event() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));

        System::assert_has_event(Event::<Test>::TaskCancelled { task_id: 0 }.into());
    });
}

#[test]
fn cancel_task_fails_for_non_poster() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_noop!(
            TaskMarket::cancel_task(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotPoster
        );
    });
}

#[test]
fn cannot_cancel_assigned_task() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        assert_noop!(
            TaskMarket::cancel_task(RuntimeOrigin::signed(1), task_id),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn cannot_cancel_completed_task() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        assert_noop!(
            TaskMarket::cancel_task(RuntimeOrigin::signed(1), task_id),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn cancel_task_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::cancel_task(RuntimeOrigin::signed(1), 999),
            Error::<Test>::TaskNotFound
        );
    });
}

// ========== Dispute Tests ==========

#[test]
fn dispute_task_by_poster() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"Work is incomplete".to_vec()
        ));

        let task = TaskMarket::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Disputed);
    });
}

#[test]
fn dispute_task_by_worker() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Worker disputes while assigned
        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(2),
            task_id,
            b"Unclear requirements".to_vec()
        ));

        let task = TaskMarket::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Disputed);
    });
}

#[test]
fn dispute_task_emits_event() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        let reason = b"Incomplete work".to_vec();

        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            reason.clone()
        ));

        System::assert_has_event(
            Event::<Test>::TaskDisputed {
                task_id,
                disputer: 1,
                reason,
            }
            .into(),
        );
    });
}

#[test]
fn dispute_fails_for_unauthorized() {
    new_test_ext().execute_with(|| {
        let task_id = setup_assigned_task(1, 2);

        // Account 3 is neither poster nor worker
        assert_noop!(
            TaskMarket::dispute_task(
                RuntimeOrigin::signed(3),
                task_id,
                b"Reason".to_vec()
            ),
            Error::<Test>::NotPoster
        );
    });
}

#[test]
fn dispute_fails_for_open_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);

        assert_noop!(
            TaskMarket::dispute_task(
                RuntimeOrigin::signed(1),
                0,
                b"Reason".to_vec()
            ),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn dispute_fails_for_cancelled_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));

        assert_noop!(
            TaskMarket::dispute_task(
                RuntimeOrigin::signed(1),
                0,
                b"Reason".to_vec()
            ),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn dispute_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::dispute_task(
                RuntimeOrigin::signed(1),
                999,
                b"Reason".to_vec()
            ),
            Error::<Test>::TaskNotFound
        );
    });
}

// ========== Resolve Dispute Tests ==========

#[test]
fn resolve_dispute_in_favor_of_worker() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"Dispute".to_vec()
        ));

        let worker_balance_before = Balances::free_balance(2);
        let poster_rep_before = Reputation::reputations(1).score;
        let worker_rep_before = Reputation::reputations(2).score;

        assert_ok!(TaskMarket::resolve_dispute(
            RuntimeOrigin::root(),
            task_id,
            2
        ));

        // Worker gets the escrow
        assert_eq!(Balances::free_balance(2), worker_balance_before + 1000);
        assert_eq!(Balances::reserved_balance(1), 0);

        // Reputation changes
        assert_eq!(Reputation::reputations(2).score, worker_rep_before + 200);
        assert_eq!(Reputation::reputations(1).score, poster_rep_before - 500);
        assert_eq!(Reputation::reputations(2).disputes_won, 1);
        assert_eq!(Reputation::reputations(1).disputes_lost, 1);
    });
}

#[test]
fn resolve_dispute_in_favor_of_poster() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"Dispute".to_vec()
        ));

        let poster_balance_before = Balances::free_balance(1);

        assert_ok!(TaskMarket::resolve_dispute(
            RuntimeOrigin::root(),
            task_id,
            1
        ));

        // Poster gets the escrow back (unreserve + transfer to self)
        // Unreserve gives back 1000, then transfer from self to self = no-op on balance
        assert_eq!(Balances::free_balance(1), poster_balance_before + 1000);
        assert_eq!(Balances::reserved_balance(1), 0);

        // Poster wins dispute, worker loses
        assert_eq!(Reputation::reputations(1).disputes_won, 1);
        assert_eq!(Reputation::reputations(2).disputes_lost, 1);
    });
}

#[test]
fn resolve_dispute_emits_event() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"Dispute".to_vec()
        ));

        assert_ok!(TaskMarket::resolve_dispute(
            RuntimeOrigin::root(),
            task_id,
            2
        ));

        System::assert_has_event(
            Event::<Test>::DisputeResolved {
                task_id,
                winner: 2,
            }
            .into(),
        );
    });
}

#[test]
fn resolve_dispute_requires_root() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"Dispute".to_vec()
        ));

        assert_noop!(
            TaskMarket::resolve_dispute(RuntimeOrigin::signed(1), task_id, 2),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn resolve_dispute_fails_if_not_disputed() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        // Task is Completed, not Disputed
        assert_noop!(
            TaskMarket::resolve_dispute(RuntimeOrigin::root(), task_id, 2),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn resolve_dispute_fails_for_nonexistent_task() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            TaskMarket::resolve_dispute(RuntimeOrigin::root(), 999, 1),
            Error::<Test>::TaskNotFound
        );
    });
}

// ========== Full Workflow Tests ==========

#[test]
fn full_happy_path_workflow() {
    new_test_ext().execute_with(|| {
        let poster = 1;
        let worker = 2;

        // 1. Post task
        assert_ok!(TaskMarket::post_task(
            RuntimeOrigin::signed(poster),
            b"Build something".to_vec(),
            b"Detailed description".to_vec(),
            2000,
            1000
        ));

        // 2. Worker bids
        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(worker),
            0,
            1500,
            b"I can do this for less".to_vec()
        ));

        // 3. Poster assigns
        assert_ok!(TaskMarket::assign_task(
            RuntimeOrigin::signed(poster),
            0,
            worker
        ));

        // 4. Worker submits
        assert_ok!(TaskMarket::submit_work(
            RuntimeOrigin::signed(worker),
            0,
            b"https://github.com/proof".to_vec()
        ));

        // 5. Poster approves
        assert_ok!(TaskMarket::approve_work(RuntimeOrigin::signed(poster), 0));

        // Final state checks
        let task = TaskMarket::tasks(0).unwrap();
        assert_eq!(task.status, TaskStatus::Approved);
        assert_eq!(Balances::free_balance(poster), 8000); // 10000 - 2000
        assert_eq!(Balances::free_balance(worker), 12000); // 10000 + 2000
        assert_eq!(Balances::reserved_balance(poster), 0);
    });
}

#[test]
fn full_dispute_workflow() {
    new_test_ext().execute_with(|| {
        let poster = 1;
        let worker = 2;

        // Post → Bid → Assign → Submit → Dispute → Resolve
        let task_id = setup_completed_task(poster, worker);

        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(poster),
            task_id,
            b"Not what I asked for".to_vec()
        ));

        assert_ok!(TaskMarket::resolve_dispute(
            RuntimeOrigin::root(),
            task_id,
            worker
        ));

        let task = TaskMarket::tasks(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Approved);
    });
}

#[test]
fn multiple_tasks_independent() {
    new_test_ext().execute_with(|| {
        // Post two tasks
        post_default_task(1);
        post_default_task(1);

        // Cancel first, complete second
        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));

        assert_ok!(TaskMarket::bid_on_task(
            RuntimeOrigin::signed(2),
            1,
            800,
            b"Proposal".to_vec()
        ));
        assert_ok!(TaskMarket::assign_task(RuntimeOrigin::signed(1), 1, 2));

        // First task cancelled, second assigned
        assert_eq!(TaskMarket::tasks(0).unwrap().status, TaskStatus::Cancelled);
        assert_eq!(TaskMarket::tasks(1).unwrap().status, TaskStatus::Assigned);
    });
}

#[test]
fn escrow_properly_reserved_across_multiple_tasks() {
    new_test_ext().execute_with(|| {
        // Post 3 tasks at 1000 each
        post_default_task(1);
        post_default_task(1);
        post_default_task(1);

        // Should have 3000 reserved
        assert_eq!(Balances::reserved_balance(1), 3000);
        assert_eq!(Balances::free_balance(1), 7000);

        // Cancel one
        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));
        assert_eq!(Balances::reserved_balance(1), 2000);
        assert_eq!(Balances::free_balance(1), 8000);
    });
}

// ========== Edge Case: Double Operations ==========

#[test]
fn cannot_approve_already_approved_task() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);
        assert_ok!(TaskMarket::approve_work(RuntimeOrigin::signed(1), task_id));

        // Can't approve again
        assert_noop!(
            TaskMarket::approve_work(RuntimeOrigin::signed(1), task_id),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn cannot_dispute_already_disputed_task() {
    new_test_ext().execute_with(|| {
        let task_id = setup_completed_task(1, 2);

        assert_ok!(TaskMarket::dispute_task(
            RuntimeOrigin::signed(1),
            task_id,
            b"First dispute".to_vec()
        ));

        // Already disputed
        assert_noop!(
            TaskMarket::dispute_task(
                RuntimeOrigin::signed(2),
                task_id,
                b"Second dispute".to_vec()
            ),
            Error::<Test>::InvalidTaskStatus
        );
    });
}

#[test]
fn cannot_bid_on_cancelled_task() {
    new_test_ext().execute_with(|| {
        post_default_task(1);
        assert_ok!(TaskMarket::cancel_task(RuntimeOrigin::signed(1), 0));

        assert_noop!(
            TaskMarket::bid_on_task(
                RuntimeOrigin::signed(2),
                0,
                800,
                b"Proposal".to_vec()
            ),
            Error::<Test>::InvalidTaskStatus
        );
    });
}
