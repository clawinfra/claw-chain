//! Unit tests for the Reputation pallet.

use crate::{self as pallet_reputation, pallet::*, *};
use frame_support::{assert_noop, assert_ok, parameter_types};
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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 10000), (2, 10000), (3, 10000), (10, 10000)],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// ========== Initial State Tests ==========

#[test]
fn initial_reputation_is_correct() {
    new_test_ext().execute_with(|| {
        let rep = Reputation::reputations(1);
        assert_eq!(rep.score, 5000);
        assert_eq!(rep.total_tasks_completed, 0);
        assert_eq!(rep.total_tasks_posted, 0);
        assert_eq!(rep.successful_completions, 0);
        assert_eq!(rep.disputes_won, 0);
        assert_eq!(rep.disputes_lost, 0);
        assert_eq!(rep.total_earned, 0);
        assert_eq!(rep.total_spent, 0);
    });
}

// ========== Submit Review Tests ==========

#[test]
fn submit_review_works() {
    new_test_ext().execute_with(|| {
        let reviewer = 1;
        let reviewee = 2;
        let rating = 5;
        let comment = b"Excellent work!".to_vec();
        let task_id = 1;

        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(reviewer),
            reviewee,
            rating,
            comment.clone(),
            task_id
        ));

        // Check review was stored
        let review = Reputation::reviews(reviewer, reviewee).unwrap();
        assert_eq!(review.rating, rating);
        assert_eq!(review.task_id, task_id);
        assert_eq!(review.created_at, 1);

        // Check reputation increased (5 stars = +500, clamped by MaxReputationDelta=500)
        let rep = Reputation::reputations(reviewee);
        assert_eq!(rep.score, 5500);
    });
}

#[test]
fn submit_review_emits_events() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            4,
            b"Good work".to_vec(),
            1
        ));

        System::assert_has_event(
            Event::<Test>::ReviewSubmitted {
                reviewer: 1,
                reviewee: 2,
                rating: 4,
                task_id: 1,
            }
            .into(),
        );

        // Should also emit ReputationChanged
        System::assert_has_event(
            Event::<Test>::ReputationChanged {
                account: 2,
                old_score: 5000,
                new_score: 5400, // 4 stars = +400
            }
            .into(),
        );
    });
}

#[test]
fn cannot_review_self() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::signed(1), 1, 5, b"Self review".to_vec(), 1),
            Error::<Test>::SelfReview
        );
    });
}

#[test]
fn invalid_rating_zero_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::signed(1), 2, 0, b"Comment".to_vec(), 1),
            Error::<Test>::InvalidRating
        );
    });
}

#[test]
fn invalid_rating_six_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::signed(1), 2, 6, b"Comment".to_vec(), 1),
            Error::<Test>::InvalidRating
        );
    });
}

#[test]
fn invalid_rating_max_u8_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::signed(1), 2, 255, b"Comment".to_vec(), 1),
            Error::<Test>::InvalidRating
        );
    });
}

#[test]
fn comment_too_long_fails() {
    new_test_ext().execute_with(|| {
        let long_comment = vec![b'x'; 257]; // Exceeds MaxCommentLength of 256
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::signed(1), 2, 5, long_comment, 1),
            Error::<Test>::CommentTooLong
        );
    });
}

#[test]
fn submit_review_max_length_comment() {
    new_test_ext().execute_with(|| {
        let comment = vec![b'x'; 256]; // Exactly MaxCommentLength
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            5,
            comment,
            1
        ));
    });
}

#[test]
fn submit_review_empty_comment() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            3,
            b"".to_vec(),
            1
        ));

        let review = Reputation::reviews(1, 2).unwrap();
        assert_eq!(review.comment.len(), 0);
    });
}

#[test]
fn submit_review_overwrites_previous() {
    new_test_ext().execute_with(|| {
        // First review: 3 stars
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            3,
            b"OK".to_vec(),
            1
        ));

        let review1 = Reputation::reviews(1, 2).unwrap();
        assert_eq!(review1.rating, 3);

        // Second review: 5 stars (overwrites)
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            5,
            b"Great after revision".to_vec(),
            2
        ));

        let review2 = Reputation::reviews(1, 2).unwrap();
        assert_eq!(review2.rating, 5);
        assert_eq!(review2.task_id, 2);
    });
}

#[test]
fn submit_review_unsigned_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::submit_review(RuntimeOrigin::none(), 2, 5, b"Comment".to_vec(), 1),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn reputation_clamped_at_max() {
    new_test_ext().execute_with(|| {
        let account = 1;

        // Submit multiple 5-star reviews to push over 10000
        // MaxReputationDelta is 500, so each 5-star gives +500
        // Starting at 5000, need 10+ reviews to reach 10000
        for i in 0..25 {
            assert_ok!(Reputation::submit_review(
                RuntimeOrigin::signed(2),
                account,
                5,
                b"Great!".to_vec(),
                i
            ));
        }

        // Should be clamped at 10000
        let rep = Reputation::reputations(account);
        assert_eq!(rep.score, 10000);
    });
}

#[test]
fn review_updates_history() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            4,
            b"Good".to_vec(),
            1
        ));

        let history = Reputation::reputation_history(2);
        assert_eq!(history.len(), 1);
    });
}

// ========== Rating Scale Tests ==========

#[test]
fn rating_scales_reputation_boost() {
    new_test_ext().execute_with(|| {
        let reviewee1 = 1;
        let reviewee2 = 2;
        let reviewee3 = 3;

        // 1-star review: +100
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(10),
            reviewee1,
            1,
            b"Poor".to_vec(),
            1
        ));
        assert_eq!(Reputation::reputations(reviewee1).score, 5100);

        // 3-star review: +300
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(10),
            reviewee2,
            3,
            b"Average".to_vec(),
            2
        ));
        assert_eq!(Reputation::reputations(reviewee2).score, 5300);

        // 5-star review: +500
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(10),
            reviewee3,
            5,
            b"Excellent".to_vec(),
            3
        ));
        assert_eq!(Reputation::reputations(reviewee3).score, 5500);
    });
}

#[test]
fn all_valid_ratings_work() {
    new_test_ext().execute_with(|| {
        // Ratings 1-5 are all valid
        for rating in 1..=5u8 {
            let reviewee = rating as u64 + 10; // accounts 11-15
            assert_ok!(Reputation::submit_review(
                RuntimeOrigin::signed(1),
                reviewee,
                rating,
                b"Test".to_vec(),
                rating as u64
            ));

            let expected = 5000 + (rating as u32) * 100;
            assert_eq!(
                Reputation::reputations(reviewee).score,
                expected,
                "Rating {} should give score {}",
                rating,
                expected
            );
        }
    });
}

// ========== Slash Reputation Tests ==========

#[test]
fn slash_reputation_works() {
    new_test_ext().execute_with(|| {
        let account = 1;
        let slash_amount = 1000;
        let reason = b"Misbehavior detected".to_vec();

        assert_eq!(Reputation::reputations(account).score, 5000);

        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            account,
            slash_amount,
            reason
        ));

        let rep = Reputation::reputations(account);
        assert_eq!(rep.score, 4000);
    });
}

#[test]
fn slash_reputation_emits_events() {
    new_test_ext().execute_with(|| {
        let reason = b"Bad behavior".to_vec();
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            1000,
            reason.clone()
        ));

        System::assert_has_event(
            Event::<Test>::ReputationSlashed {
                account: 1,
                amount: 1000,
                reason,
            }
            .into(),
        );

        System::assert_has_event(
            Event::<Test>::ReputationChanged {
                account: 1,
                old_score: 5000,
                new_score: 4000,
            }
            .into(),
        );
    });
}

#[test]
fn slash_reputation_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Reputation::slash_reputation(RuntimeOrigin::signed(1), 2, 1000, b"Reason".to_vec()),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn slash_reputation_clamps_at_zero() {
    new_test_ext().execute_with(|| {
        // Slash more than current score
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            20000,
            b"Heavy slash".to_vec()
        ));

        assert_eq!(Reputation::reputations(1).score, 0);
    });
}

#[test]
fn slash_reputation_zero_amount() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            0,
            b"No-op slash".to_vec()
        ));

        assert_eq!(Reputation::reputations(1).score, 5000);
    });
}

#[test]
fn slash_reputation_reason_too_long() {
    new_test_ext().execute_with(|| {
        let long_reason = vec![b'r'; 257];
        assert_noop!(
            Reputation::slash_reputation(RuntimeOrigin::root(), 1, 500, long_reason),
            Error::<Test>::CommentTooLong
        );
    });
}

#[test]
fn slash_reputation_updates_history() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            500,
            b"Violation".to_vec()
        ));

        let history = Reputation::reputation_history(1);
        assert_eq!(history.len(), 1);
    });
}

#[test]
fn multiple_slashes_accumulate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            1000,
            b"First".to_vec()
        ));
        assert_eq!(Reputation::reputations(1).score, 4000);

        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            1000,
            b"Second".to_vec()
        ));
        assert_eq!(Reputation::reputations(1).score, 3000);

        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            1000,
            b"Third".to_vec()
        ));
        assert_eq!(Reputation::reputations(1).score, 2000);
    });
}

// ========== ReputationManager Trait Tests ==========

#[test]
fn reputation_manager_on_task_completed() {
    new_test_ext().execute_with(|| {
        let worker = 1;
        let earned = 1000u64;

        Reputation::on_task_completed(&worker, earned);
        let rep = Reputation::reputations(worker);
        assert_eq!(rep.total_tasks_completed, 1);
        assert_eq!(rep.successful_completions, 1);
        assert_eq!(rep.total_earned, earned);
    });
}

#[test]
fn reputation_manager_on_task_completed_accumulates() {
    new_test_ext().execute_with(|| {
        let worker = 1;

        Reputation::on_task_completed(&worker, 500);
        Reputation::on_task_completed(&worker, 700);

        let rep = Reputation::reputations(worker);
        assert_eq!(rep.total_tasks_completed, 2);
        assert_eq!(rep.successful_completions, 2);
        assert_eq!(rep.total_earned, 1200);
    });
}

#[test]
fn reputation_manager_on_task_posted() {
    new_test_ext().execute_with(|| {
        let poster = 2;
        let spent = 1000u64;

        Reputation::on_task_posted(&poster, spent);
        let rep = Reputation::reputations(poster);
        assert_eq!(rep.total_tasks_posted, 1);
        assert_eq!(rep.total_spent, spent);
    });
}

#[test]
fn reputation_manager_on_task_posted_accumulates() {
    new_test_ext().execute_with(|| {
        let poster = 2;

        Reputation::on_task_posted(&poster, 300);
        Reputation::on_task_posted(&poster, 700);

        let rep = Reputation::reputations(poster);
        assert_eq!(rep.total_tasks_posted, 2);
        assert_eq!(rep.total_spent, 1000);
    });
}

#[test]
fn reputation_manager_get_reputation() {
    new_test_ext().execute_with(|| {
        assert_eq!(Reputation::get_reputation(&1), 5000);
    });
}

#[test]
fn reputation_manager_meets_minimum() {
    new_test_ext().execute_with(|| {
        assert!(Reputation::meets_minimum_reputation(&1, 4000));
        assert!(Reputation::meets_minimum_reputation(&1, 5000));
        assert!(!Reputation::meets_minimum_reputation(&1, 5001));
        assert!(!Reputation::meets_minimum_reputation(&1, 10000));
    });
}

#[test]
fn dispute_resolution_updates_reputation() {
    new_test_ext().execute_with(|| {
        let winner = 1;
        let loser = 2;

        assert_eq!(Reputation::reputations(winner).score, 5000);
        assert_eq!(Reputation::reputations(loser).score, 5000);

        Reputation::on_dispute_resolved(&winner, &loser);

        // Winner gains +200, loser loses -500
        assert_eq!(Reputation::reputations(winner).score, 5200);
        assert_eq!(Reputation::reputations(loser).score, 4500);

        assert_eq!(Reputation::reputations(winner).disputes_won, 1);
        assert_eq!(Reputation::reputations(loser).disputes_lost, 1);
    });
}

#[test]
fn dispute_resolution_emits_event() {
    new_test_ext().execute_with(|| {
        Reputation::on_dispute_resolved(&1, &2);

        System::assert_has_event(
            Event::<Test>::DisputeResolved {
                winner: 1,
                loser: 2,
            }
            .into(),
        );
    });
}

#[test]
fn multiple_dispute_resolutions() {
    new_test_ext().execute_with(|| {
        Reputation::on_dispute_resolved(&1, &2);
        Reputation::on_dispute_resolved(&1, &2);

        assert_eq!(Reputation::reputations(1).score, 5400); // +200 * 2
        assert_eq!(Reputation::reputations(2).score, 4000); // -500 * 2
        assert_eq!(Reputation::reputations(1).disputes_won, 2);
        assert_eq!(Reputation::reputations(2).disputes_lost, 2);
    });
}

// ========== History Tests ==========

#[test]
fn history_records_reviews() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            5,
            b"Excellent".to_vec(),
            1
        ));

        let history = Reputation::reputation_history(2);
        assert_eq!(history.len(), 1);
    });
}

#[test]
fn history_records_slashes() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            100,
            b"Bad".to_vec()
        ));

        let history = Reputation::reputation_history(1);
        assert_eq!(history.len(), 1);
    });
}

#[test]
fn history_combined_events() {
    new_test_ext().execute_with(|| {
        // Review adds to history
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            4,
            b"Good".to_vec(),
            1
        ));

        // Slash adds to history
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            2,
            100,
            b"Minor".to_vec()
        ));

        let history = Reputation::reputation_history(2);
        assert_eq!(history.len(), 2);
    });
}

// ========== Edge Cases ==========

#[test]
fn review_different_reviewers_same_reviewee() {
    new_test_ext().execute_with(|| {
        // Multiple reviewers can review the same person
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            3,
            5,
            b"Great".to_vec(),
            1
        ));
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(2),
            3,
            4,
            b"Good".to_vec(),
            2
        ));

        // Both reviews stored
        assert!(Reputation::reviews(1, 3).is_some());
        assert!(Reputation::reviews(2, 3).is_some());

        // Reputation updated by both: 5000 + 500 + 400 = 5900
        assert_eq!(Reputation::reputations(3).score, 5900);
    });
}

#[test]
fn review_same_reviewer_different_reviewees() {
    new_test_ext().execute_with(|| {
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            2,
            5,
            b"Great".to_vec(),
            1
        ));
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(1),
            3,
            3,
            b"OK".to_vec(),
            2
        ));

        assert!(Reputation::reviews(1, 2).is_some());
        assert!(Reputation::reviews(1, 3).is_some());
    });
}

#[test]
fn slash_and_review_combined() {
    new_test_ext().execute_with(|| {
        // Slash first
        assert_ok!(Reputation::slash_reputation(
            RuntimeOrigin::root(),
            1,
            2000,
            b"Bad".to_vec()
        ));
        assert_eq!(Reputation::reputations(1).score, 3000);

        // Then get a good review
        assert_ok!(Reputation::submit_review(
            RuntimeOrigin::signed(2),
            1,
            5,
            b"Recovered".to_vec(),
            1
        ));
        assert_eq!(Reputation::reputations(1).score, 3500);
    });
}

#[test]
fn last_active_updated_by_operations() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        Reputation::on_task_completed(&1, 100);
        assert_eq!(Reputation::reputations(1).last_active, 10);

        System::set_block_number(20);
        Reputation::on_task_posted(&1, 200);
        assert_eq!(Reputation::reputations(1).last_active, 20);
    });
}
