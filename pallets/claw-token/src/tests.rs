//! Unit tests for the CLAW Token pallet.

use crate as pallet_claw_token;
use crate::pallet::{
    AirdropClaimed, AirdropDistributed, ContributorScores, Event, TotalContributionScore,
    TreasuryBalance,
};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime for testing.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        ClawTokenPallet: pallet_claw_token,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type AccountData = pallet_balances::AccountData<u128>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

// Airdrop pool: 400,000 tokens (simplified for testing)
parameter_types! {
    pub const TestAirdropPool: u128 = 400_000;
}

impl pallet_claw_token::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type AirdropPool = TestAirdropPool;
    type MaxContributionScore = ConstU64<{ u64::MAX }>;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 1_000_000), (2, 1_000_000), (3, 1_000_000)],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn root() -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Root.into()
}

fn account(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

// ========== Record Contribution Tests ==========

#[test]
fn record_contribution_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));

        assert_eq!(ContributorScores::<Test>::get(1), 100);
        assert_eq!(TotalContributionScore::<Test>::get(), 100);
    });
}

#[test]
fn record_contribution_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));

        System::assert_has_event(
            Event::<Test>::ContributionRecorded {
                contributor: 1,
                score: 100,
                total_score: 100,
            }
            .into(),
        );
    });
}

#[test]
fn record_contribution_accumulates() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 50));

        assert_eq!(ContributorScores::<Test>::get(1), 150);
        assert_eq!(TotalContributionScore::<Test>::get(), 150);

        // Check accumulated event
        System::assert_has_event(
            Event::<Test>::ContributionRecorded {
                contributor: 1,
                score: 50,
                total_score: 150,
            }
            .into(),
        );
    });
}

#[test]
fn record_contribution_multiple_contributors() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 200));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 3, 300));

        assert_eq!(ContributorScores::<Test>::get(1), 100);
        assert_eq!(ContributorScores::<Test>::get(2), 200);
        assert_eq!(ContributorScores::<Test>::get(3), 300);
        assert_eq!(TotalContributionScore::<Test>::get(), 600);
    });
}

#[test]
fn record_contribution_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ClawTokenPallet::record_contribution(account(1), 2, 100),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn record_contribution_zero_score() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 0));
        assert_eq!(ContributorScores::<Test>::get(1), 0);
        assert_eq!(TotalContributionScore::<Test>::get(), 0);
    });
}

#[test]
fn record_contribution_large_score() {
    new_test_ext().execute_with(|| {
        let large_score = 1_000_000_000u64;
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, large_score));
        assert_eq!(ContributorScores::<Test>::get(1), large_score);
        assert_eq!(TotalContributionScore::<Test>::get(), large_score as u128);
    });
}

#[test]
fn record_contribution_fails_unsigned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ClawTokenPallet::record_contribution(frame_system::RawOrigin::None.into(), 1, 100),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ========== Claim Airdrop Tests ==========

#[test]
fn claim_airdrop_works() {
    new_test_ext().execute_with(|| {
        // Record contributions for two users
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 300));

        // User 1 claims: should get 100/400 * 400_000 = 100_000
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));
        assert!(AirdropClaimed::<Test>::get(1));
        assert_eq!(AirdropDistributed::<Test>::get(), 100_000);

        // User 2 claims: should get 300/400 * 400_000 = 300_000
        assert_ok!(ClawTokenPallet::claim_airdrop(account(2)));
        assert!(AirdropClaimed::<Test>::get(2));
        assert_eq!(AirdropDistributed::<Test>::get(), 400_000);
    });
}

#[test]
fn claim_airdrop_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 100));

        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));

        // claim = 100/200 * 400_000 = 200_000
        System::assert_has_event(
            Event::<Test>::AirdropClaimed {
                who: 1,
                amount: 200_000,
            }
            .into(),
        );
    });
}

#[test]
fn claim_airdrop_fails_if_already_claimed() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));

        assert_noop!(
            ClawTokenPallet::claim_airdrop(account(1)),
            crate::Error::<Test>::AlreadyClaimed
        );
    });
}

#[test]
fn claim_airdrop_fails_without_score() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ClawTokenPallet::claim_airdrop(account(1)),
            crate::Error::<Test>::NoContributionScore
        );
    });
}

#[test]
fn claim_airdrop_fails_with_zero_score() {
    new_test_ext().execute_with(|| {
        // Record zero score
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 0));

        assert_noop!(
            ClawTokenPallet::claim_airdrop(account(1)),
            crate::Error::<Test>::NoContributionScore
        );
    });
}

#[test]
fn claim_airdrop_fails_unsigned() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));

        assert_noop!(
            ClawTokenPallet::claim_airdrop(frame_system::RawOrigin::None.into()),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn claim_airdrop_proportional_calculation() {
    new_test_ext().execute_with(|| {
        // Set up: 4 contributors with known ratios
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100)); // 10%
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 200)); // 20%
        assert_ok!(ClawTokenPallet::record_contribution(root(), 3, 300)); // 30%

        // Total = 600
        // Note: there's an account 4 with 400 missing - but we test with what we have

        // Account 1: 100/600 * 400_000 = 66_666 (integer division)
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));
        // 100 * 400_000 / 600 = 66666
        assert_eq!(AirdropDistributed::<Test>::get(), 66_666);

        // Account 2: 200/600 * 400_000 = 133_333
        assert_ok!(ClawTokenPallet::claim_airdrop(account(2)));

        // Account 3: 300/600 * 400_000 = 200_000
        assert_ok!(ClawTokenPallet::claim_airdrop(account(3)));
    });
}

#[test]
fn claim_airdrop_single_contributor_gets_full_pool() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 500));

        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));

        // Only contributor gets entire pool
        // 500/500 * 400_000 = 400_000
        System::assert_has_event(
            Event::<Test>::AirdropClaimed {
                who: 1,
                amount: 400_000,
            }
            .into(),
        );
        assert_eq!(AirdropDistributed::<Test>::get(), 400_000);
    });
}

#[test]
fn multiple_contributors_proportional_claims() {
    new_test_ext().execute_with(|| {
        // 4 contributors with different scores
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100)); // 25%
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 100)); // 25%
        assert_ok!(ClawTokenPallet::record_contribution(root(), 3, 200)); // 50%

        // Total score = 400
        assert_eq!(TotalContributionScore::<Test>::get(), 400);

        // Each claims their proportional share
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));
        assert_ok!(ClawTokenPallet::claim_airdrop(account(2)));
        assert_ok!(ClawTokenPallet::claim_airdrop(account(3)));

        // Total distributed should equal full pool
        assert_eq!(AirdropDistributed::<Test>::get(), 400_000);
    });
}

// ========== Treasury Spend Tests ==========

#[test]
fn treasury_spend_works() {
    new_test_ext().execute_with(|| {
        // Fund the treasury first
        assert_ok!(ClawTokenPallet::fund_treasury(root(), 100_000));
        assert_eq!(TreasuryBalance::<Test>::get(), 100_000);

        assert_ok!(ClawTokenPallet::treasury_spend(root(), 1, 50_000));
        assert_eq!(TreasuryBalance::<Test>::get(), 50_000);
    });
}

#[test]
fn treasury_spend_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::fund_treasury(root(), 100_000));
        assert_ok!(ClawTokenPallet::treasury_spend(root(), 1, 50_000));

        System::assert_has_event(
            Event::<Test>::TreasurySpend {
                to: 1,
                amount: 50_000,
            }
            .into(),
        );
    });
}

#[test]
fn treasury_spend_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ClawTokenPallet::treasury_spend(account(1), 2, 50_000),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn treasury_spend_zero_amount() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::treasury_spend(root(), 1, 0));

        System::assert_has_event(Event::<Test>::TreasurySpend { to: 1, amount: 0 }.into());
    });
}

#[test]
fn treasury_spend_fails_insufficient_balance() {
    new_test_ext().execute_with(|| {
        // Treasury has 0 balance
        assert_noop!(
            ClawTokenPallet::treasury_spend(root(), 1, 50_000),
            crate::Error::<Test>::InsufficientTreasuryBalance
        );

        // Fund partially, try to overspend
        assert_ok!(ClawTokenPallet::fund_treasury(root(), 10_000));
        assert_noop!(
            ClawTokenPallet::treasury_spend(root(), 1, 50_000),
            crate::Error::<Test>::InsufficientTreasuryBalance
        );
    });
}

#[test]
fn treasury_spend_exact_balance() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::fund_treasury(root(), 50_000));
        assert_ok!(ClawTokenPallet::treasury_spend(root(), 1, 50_000));
        assert_eq!(TreasuryBalance::<Test>::get(), 0);
    });
}

#[test]
fn fund_treasury_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ClawTokenPallet::fund_treasury(account(1), 50_000),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ========== Storage State Tests ==========

#[test]
fn initial_storage_state() {
    new_test_ext().execute_with(|| {
        assert_eq!(ContributorScores::<Test>::get(1), 0);
        assert!(!AirdropClaimed::<Test>::get(1));
        assert_eq!(TotalContributionScore::<Test>::get(), 0);
        assert_eq!(AirdropDistributed::<Test>::get(), 0);
    });
}

#[test]
fn airdrop_claimed_flag_persists() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));

        // Claimed flag should be true
        assert!(AirdropClaimed::<Test>::get(1));

        // Unclaimed account should be false
        assert!(!AirdropClaimed::<Test>::get(2));
    });
}

#[test]
fn contribution_score_not_reset_after_claim() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));

        // Score should persist after claim
        assert_eq!(ContributorScores::<Test>::get(1), 100);
    });
}

#[test]
fn record_contribution_to_nonexistent_account() {
    new_test_ext().execute_with(|| {
        // Account 99 has no genesis balance, but contribution score is just storage
        assert_ok!(ClawTokenPallet::record_contribution(root(), 99, 500));
        assert_eq!(ContributorScores::<Test>::get(99), 500);
    });
}
