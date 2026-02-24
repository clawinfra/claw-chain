//! Unit tests for the RPC Registry pallet.

use crate as pallet_rpc_registry;
use crate::pallet::{ActiveNodes, NodeCount, NodeStatus, NodeType, OwnerNodes, RpcNodes};
use frame_support::{assert_noop, assert_ok, derive_impl, traits::ConstU32};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime for testing.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        RpcRegistryPallet: pallet_rpc_registry,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
}

impl pallet_rpc_registry::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxUrlLength = ConstU32<256>;
    type MaxRegionLength = ConstU32<32>;
    type MaxNodesPerOwner = ConstU32<10>;
    type MaxActiveNodes = ConstU32<1000>;
    type MaxHeartbeatInterval = ConstU32<300>;
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
fn register_node_works() {
    new_test_ext().execute_with(|| {
        let url = b"wss://rpc1.clawchain.win".to_vec();
        let region = b"eu-west".to_vec();

        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            url.clone(),
            region.clone(),
            NodeType::FullNode,
            true,
            false
        ));

        // Check node was stored
        let node = RpcNodes::<Test>::get(0).expect("Node should exist");
        assert_eq!(node.owner, 1u64);
        assert_eq!(node.url.to_vec(), url);
        assert_eq!(node.region.to_vec(), region);
        assert_eq!(node.node_type, NodeType::FullNode);
        assert_eq!(node.supports_ws, true);
        assert_eq!(node.supports_http, false);
        assert_eq!(node.status, NodeStatus::Active);
        assert_eq!(node.registered_at, 1);
        assert_eq!(node.last_heartbeat, 1);

        // Check node count incremented
        assert_eq!(NodeCount::<Test>::get(), 1);

        // Check owner mapping
        let owner_nodes = OwnerNodes::<Test>::get(1u64);
        assert_eq!(owner_nodes.len(), 1);
        assert_eq!(owner_nodes[0], 0);

        // Check active nodes list
        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], 0);
    });
}

#[test]
fn register_multiple_nodes_works() {
    new_test_ext().execute_with(|| {
        for i in 0..5u64 {
            let url = format!("wss://rpc{}.clawchain.win", i).into_bytes();
            let region = b"us-east".to_vec();
            assert_ok!(RpcRegistryPallet::register_node(
                account(1),
                url,
                region,
                NodeType::FullNode,
                true,
                true
            ));
        }

        assert_eq!(NodeCount::<Test>::get(), 5);
        let owner_nodes = OwnerNodes::<Test>::get(1u64);
        assert_eq!(owner_nodes.len(), 5);
        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 5);
    });
}

#[test]
fn register_node_fails_with_too_long_url() {
    new_test_ext().execute_with(|| {
        let url = vec![0u8; 257]; // Exceeds MaxUrlLength of 256
        let region = b"eu-west".to_vec();

        assert_noop!(
            RpcRegistryPallet::register_node(
                account(1),
                url,
                region,
                NodeType::FullNode,
                true,
                false
            ),
            crate::Error::<Test>::UrlTooLong
        );
    });
}

#[test]
fn register_node_fails_with_too_long_region() {
    new_test_ext().execute_with(|| {
        let url = b"wss://test.com".to_vec();
        let region = vec![0u8; 33]; // Exceeds MaxRegionLength of 32

        assert_noop!(
            RpcRegistryPallet::register_node(
                account(1),
                url,
                region,
                NodeType::FullNode,
                true,
                false
            ),
            crate::Error::<Test>::RegionTooLong
        );
    });
}

#[test]
fn register_node_fails_with_too_many_nodes() {
    new_test_ext().execute_with(|| {
        // Register 10 nodes (the max per owner)
        for i in 0..10u64 {
            let url = format!("wss://rpc{}.test", i).into_bytes();
            assert_ok!(RpcRegistryPallet::register_node(
                account(1),
                url,
                b"region".to_vec(),
                NodeType::FullNode,
                true,
                false
            ));
        }

        // The 11th should fail
        assert_noop!(
            RpcRegistryPallet::register_node(
                account(1),
                b"wss://overflow.test".to_vec(),
                b"region".to_vec(),
                NodeType::FullNode,
                true,
                false
            ),
            crate::Error::<Test>::TooManyNodes
        );
    });
}

#[test]
fn update_node_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://old.test".to_vec(),
            b"eu-west".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        let new_url = b"wss://new.test".to_vec();
        let new_region = b"us-east".to_vec();
        assert_ok!(RpcRegistryPallet::update_node(
            account(1),
            0,
            new_url.clone(),
            new_region.clone()
        ));

        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.url.to_vec(), new_url);
        assert_eq!(node.region.to_vec(), new_region);
    });
}

#[test]
fn update_node_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        assert_noop!(
            RpcRegistryPallet::update_node(
                account(2),
                0,
                b"wss://hacked.com".to_vec(),
                b"region".to_vec()
            ),
            crate::Error::<Test>::NotNodeOwner
        );
    });
}

#[test]
fn update_node_fails_for_nonexistent_node() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            RpcRegistryPallet::update_node(
                account(1),
                999,
                b"wss://test.com".to_vec(),
                b"region".to_vec()
            ),
            crate::Error::<Test>::NodeNotFound
        );
    });
}

#[test]
fn heartbeat_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        // Advance blocks
        System::set_block_number(100);

        assert_ok!(RpcRegistryPallet::heartbeat(account(1), 0));

        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.last_heartbeat, 100);
        assert_eq!(node.status, NodeStatus::Active);
    });
}

#[test]
fn heartbeat_reactivates_inactive_node() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        // Mark as inactive
        System::set_block_number(500);
        assert_ok!(RpcRegistryPallet::report_inactive(account(2), 0));
        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.status, NodeStatus::Inactive);

        // Send heartbeat to reactivate
        System::set_block_number(600);
        assert_ok!(RpcRegistryPallet::heartbeat(account(1), 0));

        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.status, NodeStatus::Active);
        assert_eq!(node.last_heartbeat, 600);
    });
}

#[test]
fn heartbeat_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        assert_noop!(
            RpcRegistryPallet::heartbeat(account(2), 0),
            crate::Error::<Test>::NotNodeOwner
        );
    });
}

#[test]
fn deregister_node_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        assert_ok!(RpcRegistryPallet::deregister_node(account(1), 0));

        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.status, NodeStatus::Deregistered);

        // Should be removed from active list
        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 0);
    });
}

#[test]
fn deregister_node_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        assert_noop!(
            RpcRegistryPallet::deregister_node(account(2), 0),
            crate::Error::<Test>::NotNodeOwner
        );
    });
}

#[test]
fn cannot_update_deregistered_node() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));
        assert_ok!(RpcRegistryPallet::deregister_node(account(1), 0));

        // Cannot update node
        assert_noop!(
            RpcRegistryPallet::update_node(
                account(1),
                0,
                b"wss://new.com".to_vec(),
                b"region".to_vec()
            ),
            crate::Error::<Test>::NodeAlreadyDeregistered
        );

        // Cannot send heartbeat
        assert_noop!(
            RpcRegistryPallet::heartbeat(account(1), 0),
            crate::Error::<Test>::NodeAlreadyDeregistered
        );

        // Cannot deregister again
        assert_noop!(
            RpcRegistryPallet::deregister_node(account(1), 0),
            crate::Error::<Test>::NodeAlreadyDeregistered
        );
    });
}

#[test]
fn report_inactive_works() {
    new_test_ext().execute_with(|| {
        // Register at block 1
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        // Advance past the heartbeat interval (300 blocks)
        System::set_block_number(400);

        // Anyone can report inactive
        assert_ok!(RpcRegistryPallet::report_inactive(account(2), 0));

        let node = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node.status, NodeStatus::Inactive);

        // Should be removed from active list
        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 0);
    });
}

#[test]
fn report_inactive_fails_if_node_still_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://test.com".to_vec(),
            b"region".to_vec(),
            NodeType::FullNode,
            true,
            false
        ));

        // Try to report inactive immediately (only 1 block has passed)
        System::set_block_number(2);

        assert_noop!(
            RpcRegistryPallet::report_inactive(account(2), 0),
            crate::Error::<Test>::NodeStillActive
        );
    });
}

#[test]
fn report_inactive_fails_for_nonexistent_node() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            RpcRegistryPallet::report_inactive(account(1), 999),
            crate::Error::<Test>::NodeNotFound
        );
    });
}

#[test]
fn different_node_types_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(RpcRegistryPallet::register_node(
            account(1),
            b"wss://validator.test".to_vec(),
            b"region".to_vec(),
            NodeType::Validator,
            true,
            false
        ));

        assert_ok!(RpcRegistryPallet::register_node(
            account(2),
            b"wss://archive.test".to_vec(),
            b"region".to_vec(),
            NodeType::ArchiveNode,
            true,
            true
        ));

        let node1 = RpcNodes::<Test>::get(0).unwrap();
        assert_eq!(node1.node_type, NodeType::Validator);

        let node2 = RpcNodes::<Test>::get(1).unwrap();
        assert_eq!(node2.node_type, NodeType::ArchiveNode);
    });
}

#[test]
fn active_nodes_list_management() {
    new_test_ext().execute_with(|| {
        // Register 3 nodes
        for i in 0..3u64 {
            assert_ok!(RpcRegistryPallet::register_node(
                account(i),
                format!("wss://rpc{}.test", i).into_bytes(),
                b"region".to_vec(),
                NodeType::FullNode,
                true,
                false
            ));
        }

        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 3);

        // Deregister middle node
        assert_ok!(RpcRegistryPallet::deregister_node(account(1), 1));

        let active = ActiveNodes::<Test>::get();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&0));
        assert!(!active.contains(&1));
        assert!(active.contains(&2));
    });
}
