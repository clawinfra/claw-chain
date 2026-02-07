//! Unit tests for the CLAW Token pallet.

use crate as pallet_claw_token;
use crate::pallet::{AirdropClaimed, ContributorScores, TotalContributionScore};
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

// Airdrop pool: 400 tokens (simplified for testing)
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

// ========== Tests ==========

#[test]
fn record_contribution_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));

        assert_eq!(ContributorScores::<Test>::get(1), 100);
        assert_eq!(TotalContributionScore::<Test>::get(), 100);
    });
}

#[test]
fn record_contribution_accumulates() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 50));

        assert_eq!(ContributorScores::<Test>::get(1), 150);
        assert_eq!(TotalContributionScore::<Test>::get(), 150);
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
fn claim_airdrop_works() {
    new_test_ext().execute_with(|| {
        // Record contributions for two users
        assert_ok!(ClawTokenPallet::record_contribution(root(), 1, 100));
        assert_ok!(ClawTokenPallet::record_contribution(root(), 2, 300));

        // User 1 claims: should get 100/400 * 400_000 = 100_000
        assert_ok!(ClawTokenPallet::claim_airdrop(account(1)));
        assert!(AirdropClaimed::<Test>::get(1));

        // User 2 claims: should get 300/400 * 400_000 = 300_000
        assert_ok!(ClawTokenPallet::claim_airdrop(account(2)));
        assert!(AirdropClaimed::<Test>::get(2));
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
fn treasury_spend_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ClawTokenPallet::treasury_spend(root(), 1, 50_000));
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
    });
}
