//! Mock runtime for pallet-emergency-pause unit tests.

use crate as pallet_emergency_pause;
use frame_support::{derive_impl, parameter_types};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        EmergencyPause: pallet_emergency_pause,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
}

parameter_types! {
    /// Block number type is u64 in mock — match ProposalExpiry / EmergencyPauseDuration.
    pub const PauseThreshold: u32 = 3;
    pub const UnpauseThreshold: u32 = 3;
    pub const MaxCouncilSize: u32 = 9;
    pub const MaxPalletIdLen: u32 = 64;
    pub const MaxPausedPallets: u32 = 32;
    pub const MaxActiveProposals: u32 = 16;
    pub const ProposalExpiry: u64 = 14_400;
    pub const EmergencyPauseDuration: u64 = 1_200;
}

impl pallet_emergency_pause::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type PauseThreshold = PauseThreshold;
    type UnpauseThreshold = UnpauseThreshold;
    type MaxCouncilSize = MaxCouncilSize;
    type MaxPalletIdLen = MaxPalletIdLen;
    type MaxPausedPallets = MaxPausedPallets;
    type MaxActiveProposals = MaxActiveProposals;
    type ProposalExpiry = ProposalExpiry;
    type EmergencyPauseDuration = EmergencyPauseDuration;
}

/// Build test externalities from genesis with optional council members.
pub fn new_test_ext() -> sp_io::TestExternalities {
    new_test_ext_with_members(vec![])
}

/// Build test externalities with the given initial council members.
pub fn new_test_ext_with_members(members: Vec<u64>) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_emergency_pause::GenesisConfig::<Test> {
        council_members: members,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Signed origin helper.
pub fn origin(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

/// Root origin helper.
pub fn root() -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Root.into()
}

/// Advance the block number by `n` and run `on_initialize`.
pub fn roll_to(n: u64) {
    use frame_support::traits::Hooks;
    let current = System::block_number();
    for i in (current + 1)..=n {
        System::set_block_number(i);
        EmergencyPause::on_initialize(i);
    }
}
