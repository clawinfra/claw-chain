//! Tests for pallet-agent-did

use crate::{self as pallet_agent_did, AgentDID};
use frame_support::{
    assert_noop, assert_ok, derive_impl,
    traits::ConstU32,
    BoundedVec,
};
use frame_support::sp_runtime::BuildStorage;
use sp_runtime::traits::IdentityLookup;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        AgentDid: pallet_agent_did,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
}

impl pallet_agent_did::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxEndpoints = ConstU32<10>;
    type MaxFieldLen = ConstU32<256>;
}

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

// ============================================================================
// Helpers
// ============================================================================

fn mk_bounded(s: &[u8]) -> BoundedVec<u8, ConstU32<256>> {
    BoundedVec::try_from(s.to_vec()).unwrap()
}

fn mk_ep(id: &[u8]) -> crate::pallet::ServiceEndpoint {
    crate::pallet::ServiceEndpoint {
        id: mk_bounded(id),
        endpoint_type: mk_bounded(b"LinkedDomains"),
        url: mk_bounded(b"https://example.com"),
    }
}

fn mk_eps(ids: &[&[u8]]) -> BoundedVec<crate::pallet::ServiceEndpoint, ConstU32<10>> {
    BoundedVec::try_from(ids.iter().map(|id| mk_ep(id)).collect::<Vec<_>>()).unwrap()
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn register_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        let doc = AgentDID::<Test>::get(1).expect("DID should exist");
        assert_eq!(doc.controller, 1u64);
        assert_eq!(doc.service_endpoints.len(), 0);
    });
}

#[test]
fn register_did_stores_endpoints() {
    new_test_ext().execute_with(|| {
        let eps = mk_eps(&[b"svc-1", b"svc-2"]);
        assert_ok!(AgentDid::register_did(RuntimeOrigin::signed(1), eps));
        let doc = AgentDID::<Test>::get(1).unwrap();
        assert_eq!(doc.service_endpoints.len(), 2);
    });
}

#[test]
fn register_did_fails_if_already_registered() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_noop!(
            AgentDid::register_did(RuntimeOrigin::signed(1), BoundedVec::default()),
            crate::Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn update_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        let eps = mk_eps(&[b"svc-new"]);
        assert_ok!(AgentDid::update_did(RuntimeOrigin::signed(1), eps));
        let doc = AgentDID::<Test>::get(1).unwrap();
        assert_eq!(doc.service_endpoints.len(), 1);
        assert_eq!(doc.service_endpoints[0].id.as_slice(), b"svc-new");
    });
}

#[test]
fn update_did_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDid::update_did(RuntimeOrigin::signed(2), BoundedVec::default()),
            crate::Error::<Test>::NotRegistered
        );
    });
}

#[test]
fn deactivate_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_ok!(AgentDid::deactivate_did(RuntimeOrigin::signed(1)));
        assert!(!AgentDID::<Test>::contains_key(1));
    });
}

#[test]
fn deactivate_did_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDid::deactivate_did(RuntimeOrigin::signed(3)),
            crate::Error::<Test>::NotRegistered
        );
    });
}

#[test]
fn add_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_ok!(AgentDid::add_service_endpoint(
            RuntimeOrigin::signed(1),
            mk_bounded(b"svc-1"),
            mk_bounded(b"LinkedDomains"),
            mk_bounded(b"https://example.com"),
        ));
        let doc = AgentDID::<Test>::get(1).unwrap();
        assert_eq!(doc.service_endpoints.len(), 1);
        assert_eq!(doc.service_endpoints[0].id.as_slice(), b"svc-1");
    });
}

#[test]
fn remove_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_ok!(AgentDid::add_service_endpoint(
            RuntimeOrigin::signed(1),
            mk_bounded(b"svc-1"),
            mk_bounded(b"LinkedDomains"),
            mk_bounded(b"https://example.com"),
        ));
        assert_ok!(AgentDid::remove_service_endpoint(
            RuntimeOrigin::signed(1),
            mk_bounded(b"svc-1"),
        ));
        let doc = AgentDID::<Test>::get(1).unwrap();
        assert_eq!(doc.service_endpoints.len(), 0);
    });
}

#[test]
fn remove_endpoint_fails_if_not_found() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_noop!(
            AgentDid::remove_service_endpoint(
                RuntimeOrigin::signed(1),
                mk_bounded(b"nonexistent"),
            ),
            crate::Error::<Test>::EndpointNotFound
        );
    });
}

#[test]
fn too_many_endpoints_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        // Fill up to MaxEndpoints (10)
        for i in 0u8..10 {
            assert_ok!(AgentDid::add_service_endpoint(
                RuntimeOrigin::signed(1),
                BoundedVec::try_from(vec![i]).unwrap(),
                mk_bounded(b"type"),
                mk_bounded(b"https://example.com"),
            ));
        }
        // 11th should fail
        assert_noop!(
            AgentDid::add_service_endpoint(
                RuntimeOrigin::signed(1),
                mk_bounded(b"overflow"),
                mk_bounded(b"type"),
                mk_bounded(b"https://example.com"),
            ),
            crate::Error::<Test>::TooManyEndpoints
        );
    });
}

#[test]
fn add_endpoint_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDid::add_service_endpoint(
                RuntimeOrigin::signed(99),
                mk_bounded(b"svc-1"),
                mk_bounded(b"LinkedDomains"),
                mk_bounded(b"https://example.com"),
            ),
            crate::Error::<Test>::NotRegistered
        );
    });
}

#[test]
fn did_can_be_re_registered_after_deactivation() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert_ok!(AgentDid::deactivate_did(RuntimeOrigin::signed(1)));
        // Should be able to re-register after deactivation
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            BoundedVec::default()
        ));
        assert!(AgentDID::<Test>::contains_key(1));
    });
}
