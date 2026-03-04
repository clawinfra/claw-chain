//! Unit tests for the Agent Registry pallet.

use crate as pallet_agent_registry;
use crate::pallet::{AgentCount, AgentRegistry, AgentStatus, Event, OwnerAgents};
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

// ========== Registration Tests ==========

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
        assert_eq!(agent.last_active, 1);

        // Check agent count incremented
        assert_eq!(AgentCount::<Test>::get(), 1);

        // Check owner mapping
        let owner_agents = OwnerAgents::<Test>::get(1u64);
        assert_eq!(owner_agents.len(), 1);
        assert_eq!(owner_agents[0], 0);
    });
}

#[test]
fn register_agent_emits_event() {
    new_test_ext().execute_with(|| {
        let did = b"did:claw:agent001".to_vec();
        let metadata = b"{}".to_vec();

        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            did.clone(),
            metadata,
        ));

        System::assert_has_event(
            Event::<Test>::AgentRegistered {
                agent_id: 0,
                owner: 1,
                did,
            }
            .into(),
        );
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
        // Agent IDs should be sequential
        for i in 0..5u64 {
            assert_eq!(owner_agents[i as usize], i);
        }
    });
}

#[test]
fn register_agents_different_owners() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:a".to_vec(),
            b"{}".to_vec()
        ));
        assert_ok!(AgentRegistryPallet::register_agent(
            account(2),
            b"did:claw:b".to_vec(),
            b"{}".to_vec()
        ));

        assert_eq!(AgentCount::<Test>::get(), 2);
        assert_eq!(OwnerAgents::<Test>::get(1u64).len(), 1);
        assert_eq!(OwnerAgents::<Test>::get(2u64).len(), 1);

        // Agent 0 owned by 1, Agent 1 owned by 2
        assert_eq!(AgentRegistry::<Test>::get(0).unwrap().owner, 1u64);
        assert_eq!(AgentRegistry::<Test>::get(1).unwrap().owner, 2u64);
    });
}

#[test]
fn register_agent_with_empty_did() {
    new_test_ext().execute_with(|| {
        // Empty DID should be allowed (no minimum length check)
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"".to_vec(),
            b"{}".to_vec()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.did.len(), 0);
    });
}

#[test]
fn register_agent_with_empty_metadata() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"".to_vec()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.metadata.len(), 0);
    });
}

#[test]
fn register_agent_with_max_length_did() {
    new_test_ext().execute_with(|| {
        let did = vec![b'x'; 256]; // Exactly MaxDidLength
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            did.clone(),
            b"{}".to_vec()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.did.len(), 256);
    });
}

#[test]
fn register_agent_with_max_length_metadata() {
    new_test_ext().execute_with(|| {
        let metadata = vec![b'y'; 4096]; // Exactly MaxMetadataLength
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            metadata.clone()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.metadata.len(), 4096);
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

        // But a different owner should still be able to register
        assert_ok!(AgentRegistryPallet::register_agent(
            account(2),
            b"did:claw:other".to_vec(),
            b"{}".to_vec()
        ));
    });
}

#[test]
fn register_agent_fails_with_unsigned_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistryPallet::register_agent(
                frame_system::RawOrigin::None.into(),
                b"did:claw:test".to_vec(),
                b"{}".to_vec()
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ========== Update Metadata Tests ==========

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
fn update_metadata_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        let new_metadata = b"{\"updated\": true}".to_vec();
        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            new_metadata.clone()
        ));

        System::assert_has_event(
            Event::<Test>::AgentUpdated {
                agent_id: 0,
                metadata: new_metadata,
            }
            .into(),
        );
    });
}

#[test]
fn update_metadata_updates_last_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Advance block
        System::set_block_number(42);

        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            b"{\"v\": 2}".to_vec()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.last_active, 42);
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
fn update_metadata_fails_with_too_long_metadata() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        let long_metadata = vec![b'x'; 4097];
        assert_noop!(
            AgentRegistryPallet::update_metadata(account(1), 0, long_metadata),
            crate::Error::<Test>::MetadataTooLong
        );
    });
}

#[test]
fn update_metadata_preserves_did_and_reputation() {
    new_test_ext().execute_with(|| {
        let did = b"did:claw:test".to_vec();
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            did.clone(),
            b"{}".to_vec()
        ));

        // Change reputation first
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 1000));

        // Update metadata
        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            b"{\"new\": true}".to_vec()
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.did.to_vec(), did); // DID unchanged
        assert_eq!(agent.reputation, 6000); // Reputation unchanged
    });
}

// ========== Update Reputation Tests ==========

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
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, -2000));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 4000);
    });
}

#[test]
fn update_reputation_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 500));

        System::assert_has_event(
            Event::<Test>::ReputationChanged {
                agent_id: 0,
                old_score: 5000,
                new_score: 5500,
            }
            .into(),
        );
    });
}

#[test]
fn update_reputation_clamps_at_max() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Try to exceed max (10000)
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 9999));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 10000); // Clamped at max
    });
}

#[test]
fn update_reputation_clamps_at_min() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Try to go below 0
        assert_ok!(AgentRegistryPallet::update_reputation(
            account(1),
            0,
            -20000
        ));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 0); // Clamped at zero
    });
}

#[test]
fn update_reputation_zero_delta() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 0));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 5000); // Unchanged
    });
}

#[test]
fn update_reputation_by_non_owner_allowed() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Anyone can update reputation (design choice per the code comment)
        assert_ok!(AgentRegistryPallet::update_reputation(account(2), 0, 100));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 5100);
    });
}

#[test]
fn update_reputation_fails_for_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistryPallet::update_reputation(account(1), 999, 100),
            crate::Error::<Test>::AgentNotFound
        );
    });
}

#[test]
fn update_reputation_updates_last_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        System::set_block_number(99);

        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 100));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.last_active, 99);
    });
}

// ========== Deregister Tests ==========

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
fn deregister_agent_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        System::assert_has_event(Event::<Test>::AgentDeregistered { agent_id: 0 }.into());
    });
}

#[test]
fn deregister_agent_updates_last_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        System::set_block_number(50);
        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.last_active, 50);
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
fn deregister_agent_fails_for_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistryPallet::deregister_agent(account(1), 999),
            crate::Error::<Test>::AgentNotFound
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

        // Cannot set status
        assert_noop!(
            AgentRegistryPallet::set_agent_status(account(1), 0, AgentStatus::Active),
            crate::Error::<Test>::AgentAlreadyDeregistered
        );
    });
}

#[test]
fn deregistered_agent_data_persists() {
    new_test_ext().execute_with(|| {
        let did = b"did:claw:test".to_vec();
        let metadata = b"{\"name\": \"agent\"}".to_vec();
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            did.clone(),
            metadata.clone()
        ));
        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        // Agent data should still be readable
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.owner, 1u64);
        assert_eq!(agent.did.to_vec(), did);
        assert_eq!(agent.metadata.to_vec(), metadata);
        assert_eq!(agent.reputation, 5000);
        // Count is not decremented
        assert_eq!(AgentCount::<Test>::get(), 1);
    });
}

// ========== Set Agent Status Tests ==========

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
fn set_agent_status_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Suspended
        ));

        System::assert_has_event(
            Event::<Test>::AgentStatusChanged {
                agent_id: 0,
                status: AgentStatus::Suspended,
            }
            .into(),
        );
    });
}

#[test]
fn set_agent_status_updates_last_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        System::set_block_number(77);
        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Suspended
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.last_active, 77);
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

#[test]
fn set_agent_status_fails_for_nonexistent_agent() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentRegistryPallet::set_agent_status(account(1), 999, AgentStatus::Active),
            crate::Error::<Test>::AgentNotFound
        );
    });
}

#[test]
fn set_agent_status_to_deregistered_via_set_status() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        // Can set status to Deregistered via set_agent_status
        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Deregistered
        ));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Deregistered);

        // And then it's locked — can't change again
        assert_noop!(
            AgentRegistryPallet::set_agent_status(account(1), 0, AgentStatus::Active),
            crate::Error::<Test>::AgentAlreadyDeregistered
        );
    });
}

#[test]
fn suspended_agent_can_be_updated() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:test".to_vec(),
            b"{}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Suspended
        ));

        // Suspended agents can still have metadata updated
        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            b"{\"suspended\": true}".to_vec()
        ));

        // And reputation updated
        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, -500));
        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.reputation, 4500);
    });
}

// ========== Edge Cases ==========

#[test]
fn agent_id_zero_is_first() {
    new_test_ext().execute_with(|| {
        // First agent should get ID 0
        assert_eq!(AgentCount::<Test>::get(), 0);
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:first".to_vec(),
            b"{}".to_vec()
        ));

        assert!(AgentRegistry::<Test>::get(0).is_some());
        assert!(AgentRegistry::<Test>::get(1).is_none());
    });
}

#[test]
fn nonexistent_agent_returns_none() {
    new_test_ext().execute_with(|| {
        assert!(AgentRegistry::<Test>::get(0).is_none());
        assert!(AgentRegistry::<Test>::get(u64::MAX).is_none());
    });
}

#[test]
fn owner_agents_empty_initially() {
    new_test_ext().execute_with(|| {
        let agents = OwnerAgents::<Test>::get(1u64);
        assert_eq!(agents.len(), 0);
    });
}

#[test]
fn multiple_operations_sequence() {
    new_test_ext().execute_with(|| {
        // Register → Update Metadata → Change Reputation → Suspend → Reactivate → Deregister
        assert_ok!(AgentRegistryPallet::register_agent(
            account(1),
            b"did:claw:lifecycle".to_vec(),
            b"{\"v\": 1}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::update_metadata(
            account(1),
            0,
            b"{\"v\": 2}".to_vec()
        ));

        assert_ok!(AgentRegistryPallet::update_reputation(account(1), 0, 2000));

        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Suspended
        ));

        assert_ok!(AgentRegistryPallet::set_agent_status(
            account(1),
            0,
            AgentStatus::Active
        ));

        assert_ok!(AgentRegistryPallet::deregister_agent(account(1), 0));

        let agent = AgentRegistry::<Test>::get(0).unwrap();
        assert_eq!(agent.status, AgentStatus::Deregistered);
        assert_eq!(agent.reputation, 7000);
        assert_eq!(agent.metadata.to_vec(), b"{\"v\": 2}");
    });
}
