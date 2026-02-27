use crate as pallet_anon_messaging;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use frame_system as system;
use pallet_balances::AccountData;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        AnonMessaging: pallet_anon_messaging,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type Block = Block;
    type AccountData = AccountData<u64>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

/// Mock reputation manager — returns configurable scores.
pub struct MockReputation;

thread_local! {
    static MOCK_REPUTATION: std::cell::RefCell<std::collections::HashMap<u64, u32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Set a reputation score for an account in tests.
pub fn set_reputation(account: u64, score: u32) {
    MOCK_REPUTATION.with(|r| {
        r.borrow_mut().insert(account, score);
    });
}

impl pallet_reputation::ReputationManager<u64, u64> for MockReputation {
    fn on_task_completed(_worker: &u64, _earned: u64) {}
    fn on_task_posted(_poster: &u64, _spent: u64) {}
    fn on_dispute_resolved(_winner: &u64, _loser: &u64) {}

    fn get_reputation(account: &u64) -> u32 {
        MOCK_REPUTATION.with(|r| *r.borrow().get(account).unwrap_or(&5000))
    }

    fn meets_minimum_reputation(account: &u64, minimum: u32) -> bool {
        Self::get_reputation(account) >= minimum
    }
}

parameter_types! {
    pub const MaxKeyBytes: u32 = 64;
    pub const MaxInboxSize: u32 = 100;
    pub const MaxInlinePayloadBytes: u32 = 512;
    pub const MaxEphemeralPerBlock: u32 = 50;
    pub const MinReputationToSend: u32 = 0; // off by default in tests
    pub const MinTtlBlocks: u32 = 10;
    pub const MaxTtlBlocks: u32 = 1_000_000;
    pub const MaxEscrowAmount: u64 = 1_000_000_000;
}

impl pallet_anon_messaging::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type ReputationManager = MockReputation;
    type MaxKeyBytes = MaxKeyBytes;
    type MaxInboxSize = MaxInboxSize;
    type MaxInlinePayloadBytes = MaxInlinePayloadBytes;
    type MaxEphemeralPerBlock = MaxEphemeralPerBlock;
    type MinReputationToSend = MinReputationToSend;
    type MinTtlBlocks = MinTtlBlocks;
    type MaxTtlBlocks = MaxTtlBlocks;
    type MaxEscrowAmount = MaxEscrowAmount;
}

/// Build a test externalities environment.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 100_000), (2, 100_000), (3, 100_000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// X25519 test key for account 1.
pub const ALICE_KEY: [u8; 32] = [1u8; 32];
/// X25519 test key for account 2.
pub const BOB_KEY: [u8; 32] = [2u8; 32];

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
