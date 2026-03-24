//! Mock runtime for pallet-audit-attestation tests.

#![cfg(test)]

use crate::{self as pallet_audit_attestation, AgentRegistryInterface};
use frame_support::{derive_impl, traits::ConstU32};
use sp_runtime::BuildStorage;

// =========================================================
// Mock Agent Registry
// =========================================================

/// A mock that treats accounts 1–100 as registered active agents.
pub struct MockAgentRegistry;

impl AgentRegistryInterface<u64> for MockAgentRegistry {
    fn is_registered_agent(account: &u64) -> bool {
        *account >= 1 && *account <= 100
    }
}

// =========================================================
// Mock Runtime
// =========================================================

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        AuditAttestation: pallet_audit_attestation,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = frame_system::mocking::MockBlockU32<Test>;
    type AccountData = ();
}

impl pallet_audit_attestation::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAttestationsPerAuditor = ConstU32<500>;
    type MaxDidLen = ConstU32<128>;
    type AgentRegistry = MockAgentRegistry;
}

// =========================================================
// Test helpers
// =========================================================

/// Build default test externalities.
pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("Failed to build test storage")
        .into()
}

/// Advance the mock chain by `n` blocks.
pub fn run_to_block(n: u32) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}
