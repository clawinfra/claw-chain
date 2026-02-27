//! Mock runtime for IBC-lite pallet tests.

#![cfg(test)]

use super::*;
use frame_support::derive_impl;
use frame_support::traits::{ConstBool, ConstU32, ConstU64};
use frame_system::config::PalletId;
use sp_core::H256;

// =========================================================
// Mock Agent Registry
// =========================================================

pub struct MockAgentRegistry;

impl AgentRegistryInterface<u64> for MockAgentRegistry {
    fn agent_exists(agent_id: u64) -> bool {
        agent_id > 0 && agent_id <= 100
    }

    fn agent_owner(agent_id: u64) -> Option<u64> {
        if Self::agent_exists(agent_id) {
            Some(agent_id)
        } else {
            None
        }
    }

    fn is_agent_active(agent_id: u64) -> bool {
        Self::agent_exists(agent_id)
    }
}

// =========================================================
// Mock Runtime
// =========================================================

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        IbcLite: pallet_ibc_lite,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Runtime {
    type Block = frame_system::mocking::MockBlockU32<Runtime>;
    type AccountData = ();
}

impl Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type RelayerManagerOrigin = frame_system::EnsureRoot<u64>;
    type MaxRelayers = ConstU32<10>;
    type MaxChannelsPerChain = ConstU32<100>;
    type MaxChannelIdLen = ConstU32<128>;
    type MaxChainIdLen = ConstU32<128>;
    type MaxPayloadLen = ConstU32<4096>;
    type MaxPendingPackets = ConstU32<1000>;
    type PacketTimeoutBlocks = ConstU32<100>;
    type AgentRegistry = MockAgentRegistry;
}

// =========================================================
// Test Externalities
// =========================================================

pub fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .unwrap()
        .into()
}
