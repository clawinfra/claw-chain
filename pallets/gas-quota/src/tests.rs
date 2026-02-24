//! Tests for pallet-gas-quota

use crate::{self as pallet_gas_quota, AgentQuotas};
use frame_support::{
    assert_noop, assert_ok,
    parameter_types,
    traits::ConstU32,
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, Perbill,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        GasQuota: pallet_gas_quota,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const ExistentialDeposit: u64 = 1;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
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
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
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
    pub const BlocksPerDay: u64 = 14_400; // 6s blocks
    pub const MinFreeQuota: u32 = 10;
    pub const StakePerFreeTx: u64 = 1_000_000; // 1 $CLAW (planck) = 1 free tx
    pub const UnlimitedStakeThreshold: u64 = 10_000_000_000; // 10,000 $CLAW
    pub const BaseFeePerTx: u64 = 1_000; // 0.001 $CLAW
    pub const FeeDiscountPerKStake: Perbill = Perbill::from_percent(90); // 10% discount per kStake
}

impl pallet_gas_quota::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlocksPerDay = BlocksPerDay;
    type MinFreeQuota = MinFreeQuota;
    type StakePerFreeTx = StakePerFreeTx;
    type UnlimitedStakeThreshold = UnlimitedStakeThreshold;
    type BaseFeePerTx = BaseFeePerTx;
    type FeeDiscountPerKStake = FeeDiscountPerKStake;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 10_000_000_000), // 10,000 $CLAW — unlimited tier
            (2, 1_000_000_000),  // 1,000 $CLAW
            (3, 100_000_000),    // 100 $CLAW
            (4, 5_000),          // dust — minimum tier (needs > ExistentialDeposit + BaseFeePerTx)
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

#[test]
fn min_quota_with_no_stake() {
    new_test_ext().execute_with(|| {
        // Agent 4 has dust balance — should get minimum 10 free TX
        let quota = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(0, 0);
        assert_eq!(quota, 10);
    });
}

#[test]
fn quota_scales_with_stake() {
    new_test_ext().execute_with(|| {
        // 100 $CLAW (100_000_000 planck) → 100 free tx
        let quota = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(100_000_000, 0);
        assert_eq!(quota, 100);

        // 1,000 $CLAW → 1,000 free tx
        let quota = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(1_000_000_000, 0);
        assert_eq!(quota, 1000);
    });
}

#[test]
fn reputation_multiplier_applied() {
    new_test_ext().execute_with(|| {
        let base = 100u32;
        let stake = 100_000_000u64; // 100 $CLAW → 100 base quota

        let normal = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(stake, 0);
        let high_rep = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(stake, 1);
        let verified = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(stake, 2);

        assert_eq!(normal, base);
        assert_eq!(high_rep, 150); // 1.5×
        assert_eq!(verified, 200); // 2×
    });
}

#[test]
fn unlimited_at_threshold() {
    new_test_ext().execute_with(|| {
        let quota = pallet_gas_quota::Pallet::<Test>::calculate_free_quota(
            10_000_000_000, // exactly at threshold
            0,
        );
        assert_eq!(quota, u32::MAX);
    });
}

#[test]
fn consume_quota_initializes_on_first_use() {
    new_test_ext().execute_with(|| {
        assert!(!AgentQuotas::<Test>::contains_key(4));
        assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));
        assert!(AgentQuotas::<Test>::contains_key(4));
    });
}

#[test]
fn consume_quota_within_free_limit() {
    new_test_ext().execute_with(|| {
        // Agent 4 has 1,000 planck → minimum 10 free tx
        for _ in 0..10 {
            assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));
        }
        // No fee charged — balance unchanged (modulo existential deposit)
        let quota = AgentQuotas::<Test>::get(4).unwrap();
        assert_eq!(quota.daily_used, 10);
    });
}

#[test]
fn consume_quota_charges_fee_over_limit() {
    new_test_ext().execute_with(|| {
        // Exhaust the 10 free TX for agent 4
        for _ in 0..10 {
            assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));
        }
        let balance_before = pallet_balances::Pallet::<Test>::free_balance(4);
        // 11th TX should charge fee
        assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));
        let balance_after = pallet_balances::Pallet::<Test>::free_balance(4);
        assert!(balance_after < balance_before, "Fee should have been charged");
    });
}

#[test]
fn quota_resets_after_day() {
    new_test_ext().execute_with(|| {
        // Use all 10 free TX
        for _ in 0..10 {
            assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));
        }
        let quota_before = AgentQuotas::<Test>::get(4).unwrap();
        assert_eq!(quota_before.daily_used, 10);

        // Advance by one day
        frame_system::Pallet::<Test>::set_block_number(14_401);
        assert_ok!(pallet_gas_quota::Pallet::<Test>::consume_quota(&4));

        let quota_after = AgentQuotas::<Test>::get(4).unwrap();
        assert_eq!(quota_after.daily_used, 1, "Counter should reset and count just this TX");
    });
}
