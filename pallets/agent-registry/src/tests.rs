//! Unit tests for the Agent Registry pallet.

use crate as pallet_agent_registry;
use crate::pallet::{AgentCount, AgentRegistry, AgentStatus, OwnerAgents};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
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
        AgentRegistryPallet: pallet_agent_registry,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
}

impl pallet_agent_registry::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxDidLength = ConstU32<256>;
    type MaxMetadataLength = ConstU32<4096>;
    type MaxAgentsPerOwner = ConstU32<10>;
}

// Build test externalities from genesis storage.
fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

// ========== Tests ==========

#[test]
fn register_agent_works() {
    new_test_ext().execute_with(|| {
        let did = b"did:claw:agent001".to_vec();
        let metadata = b"{\"name\": \"EvoClaw\", \"type\": \"assistant\"}".to_vec();

        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            did.clone(),
            metadata.clone()
        ));

        // Check agent was stored
        let agent = AgentRegistry::<Test>::get(0).expect("Agent should exist");
        assert_eq!(agent.owner, 1u64);
        assert_eq!(agent.did.to_vec(), did);
        assert_eq!(agent.metadata.to_vec(), metadata);
        assert_eq!(agent.reputation, 5000);
        assert_eq!(agent.status, AgentStatus::Active);
        assert_eq!(agent.registered_at, 1);

        // Check agent count incremented
        assert_eq!(AgentCount::<Test>::get(), 1);

        // Check owner mapping
        let owner_agents = OwnerAgents::<Test>::get(1u64);
        assert_eq!(owner_agents.len(), 1);
        assert_eq!(owner_agents[0], 0);
    });
}

#[test]
fn register_multiple_agents_works() {
    new_test_ext().execute_with(|| {
        for i in 0..5u64 {
            let did = format!("did:claw:agent{:03}", i).into_bytes();
            let metadata = format!("{{\"name\": \"Agent {}\"}}", i).into_bytes();
            assert_ok!(AgentRegistryPallet::register_agent(
                account(1),
                did,
                metadata
            ));
        }

        assert_eq!(AgentCount::<Test>::get(), 5);
        let owner_agents = OwnerAgents::<Test>::get(1u64);
        assert_eq!(owner_agents.len(), 5);
    });
}

#[test]
fn register_agent_fails_with_too_long_did() {
    new_test_ext().execute_with(|| {
        let did = vec![0u8; 257]; // Exceeds MaxDidLength of 256
        let metadata = b"{}".to_vec();

        assert_noop!(
            AgentRegistryPallet::register_agent(account(1), did, metadata),
            crate::Error::<Test>::DidTooLong
        );
    });
}

#[test]
fn register_agent_fails_with_too_long_metadata() {
    new_test_ext().execute_with(|| {
        let did = b"did:claw:test".to_vec();
        let metadata = vec![0u8; 4097]; // Exceeds MaxMetadataLength of 4096

        assert_noop!(
            AgentRegistryPallet::register_agent(account(1), did, metadata),
            crate::Error::<Test>::MetadataTooLong
        );
    });
}

#[test]
fn register_agent_fails_with_too_many_agents() {
    new_test_ext().execute_with(|| {
        // Register 10 agents (the max per owner)
        for i in 0..10u64 {
            let did = format!("did:claw:agent{:03}", i).into_bytes();
            assert_ok!(AgentRegistryPallet::register_agent(
                account(1),
                did,
                b"{}".to_vec()
            ));
        }

        // The 11th should fail
        assert_noop!(
            AgentRegistryPallet::register_agent(
                account(1),
                b"did:claw:overflow".to_vec(),
                b"{}".to_vec()
            ),
            crate::Error::<Test>::TooManyAgents
        );
    });
}

#[test]
fn update_metadata_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{\"v\": 1}".to_vec()
        ));

        let new_metadata = b"{\"v\": 2, \"upgraded\": true}".to_vec();
        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            new_metadata.clone()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.metadata.to_vec(), new_metadata);
    });
}

#[test]
fn update_metadata_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_noop!(
            AgentRegistryPallet::update_metadata(account(2), 0, b"{\"hacked\": true}".to_vec()),
            crate::Error::<Test>::NotAgentOwner
        );
    });
}

#[test]
fn update_metadata_fails_for_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistryPallet::update_metadata(account(1), 999, b"{}".to_vec()),
            crate::Error::<Test>::AgentNotFound
        );
    });
}

#[test]
fn update_reputation_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Increase reputation
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 1000));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 6000);

        // Decrease reputation
        assert_ok!(AgentRegistryPallet::update_reputation(
            account(1),
            0,
            -2000
        ));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 4000);
    });
}

#[test]
fn update_reputation_clamps_at_bounds() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Try to exceed max (10000)
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 9999));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 10000); // Clamped

        // Try to go below 0
        assert_ok!(AgentRegistryPallet::update_reputation(
            account(1),
            0,
            -20000
        ));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 0); // Clamped
    });
}

#[test]
fn deregister_agent_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Deregistered);
    });
}

#[test]
fn deregister_agent_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_noop!(
            AgentRegistryPallet::deregister_agent(account(2), 0),
            crate::Error::<Test>::NotAgentOwner
        );
    });
}

#[test]
fn cannot_update_deregistered_agent() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));
        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        // Cannot update metadata
        assert_noop!(
            AgentRegistryPallet::update_metadata(account(1), 0, b"{}".to_vec()),
            crate::Error::<Test>::AgentAlreadyDeregistered
        );

        // Cannot update reputation
        assert_noop!(
            AgentRegistryPallet::update_reputation(account(1), 0, 100),
            crate::Error::<Test>::AgentAlreadyDeregistered
        );

        // Cannot deregister again
        assert_noop!(
            AgentRegistryPallet::deregister_agent(account(1), 0),
            crate::Error::<Test>::AgentAlreadyDeregistered
        );
    });
}

#[test]
fn set_agent_status_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Suspend the agent
        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Suspended
        ));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Suspended);

        // Reactivate the agent
        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Active
        ));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Active);
    });
}

#[test]
fn set_agent_status_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_noop!(
            AgentRegistryPallet::set_agent_status(account(2), 0, AgentStatus::Suspended),
            crate::Error::<Test>::NotAgentOwner
        );
    });
}
