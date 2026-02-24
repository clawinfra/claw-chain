//! Unit tests for the Agent DID pallet.

use crate as pallet_agent_did;
use crate::pallet::{DIDCount, DIDDocuments, ServiceEndpoints};
use frame_support::{assert_noop, assert_ok, derive_impl, traits::ConstU32};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        AgentDID: pallet_agent_did,
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
    type WeightInfo = ();
    type MaxContextLength = ConstU32<512>;
    type MaxServiceIdLength = ConstU32<128>;
    type MaxServiceTypeLength = ConstU32<128>;
    type MaxEndpointLength = ConstU32<512>;
    type MaxServiceEndpoints = ConstU32<10>;
    type MaxKeyIdLength = ConstU32<128>;
    type MaxKeyTypeLength = ConstU32<128>;
    type MaxKeyLength = ConstU32<256>;
    type MaxVerificationMethods = ConstU32<5>;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn signed(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

// ========================= register_did =========================

#[test]
fn register_did_works() {
    new_test_ext().execute_with(|| {
        let ctx = b"https://www.w3.org/ns/did/v1".to_vec();
        assert_ok!(AgentDID::register_did(signed(1), ctx.clone()));

        let doc = DIDDocuments::<Test>::get(1u64).expect("DID doc should exist");
        assert_eq!(doc.controller, 1u64);
        assert_eq!(doc.context.to_vec(), ctx);
        assert_eq!(doc.created, 1);
        assert_eq!(doc.updated, 1);
        assert!(!doc.deactivated);
        assert_eq!(doc.service_endpoint_count, 0);
        assert_eq!(doc.verification_method_count, 0);
        assert_eq!(DIDCount::<Test>::get(), 1);
    });
}

#[test]
fn register_did_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(2), b"".to_vec()));
        System::assert_last_event(crate::pallet::Event::DIDRegistered { controller: 2u64 }.into());
    });
}

#[test]
fn register_did_fails_if_already_registered() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_noop!(
            AgentDID::register_did(signed(1), b"".to_vec()),
            crate::pallet::Error::<Test>::DIDAlreadyExists
        );
    });
}

#[test]
fn register_did_fails_context_too_long() {
    new_test_ext().execute_with(|| {
        let too_long = vec![0u8; 513]; // MaxContextLength = 512
        assert_noop!(
            AgentDID::register_did(signed(1), too_long),
            crate::pallet::Error::<Test>::ContextTooLong
        );
    });
}

// ========================= update_did =========================

#[test]
fn update_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"v1".to_vec()));
        System::set_block_number(5);
        assert_ok!(AgentDID::update_did(signed(1), b"v2".to_vec()));

        let doc = DIDDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.context.to_vec(), b"v2".to_vec());
        assert_eq!(doc.created, 1);
        assert_eq!(doc.updated, 5);
    });
}

#[test]
fn update_did_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDID::update_did(signed(99), b"".to_vec()),
            crate::pallet::Error::<Test>::DIDNotFound
        );
    });
}

#[test]
fn update_did_fails_if_deactivated() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::deactivate_did(signed(1)));
        assert_noop!(
            AgentDID::update_did(signed(1), b"new".to_vec()),
            crate::pallet::Error::<Test>::DIDDeactivated
        );
    });
}

// ========================= deactivate_did =========================

#[test]
fn deactivate_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_eq!(DIDCount::<Test>::get(), 1);

        assert_ok!(AgentDID::deactivate_did(signed(1)));
        let doc = DIDDocuments::<Test>::get(1u64).unwrap();
        assert!(doc.deactivated);
        assert_eq!(DIDCount::<Test>::get(), 0);

        System::assert_last_event(crate::pallet::Event::DIDDeactivated { controller: 1u64 }.into());
    });
}

#[test]
fn deactivate_did_fails_if_already_deactivated() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::deactivate_did(signed(1)));
        assert_noop!(
            AgentDID::deactivate_did(signed(1)),
            crate::pallet::Error::<Test>::DIDDeactivated
        );
    });
}

#[test]
fn deactivate_did_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDID::deactivate_did(signed(99)),
            crate::pallet::Error::<Test>::DIDNotFound
        );
    });
}

// ========================= add_service_endpoint =========================

#[test]
fn add_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#rpc".to_vec(),
            b"JsonRpcService".to_vec(),
            b"https://node.claw.network/rpc".to_vec(),
        ));

        let doc = DIDDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.service_endpoint_count, 1);

        let bounded_id: frame_support::BoundedVec<u8, ConstU32<128>> =
            b"#rpc".to_vec().try_into().unwrap();
        let se = ServiceEndpoints::<Test>::get(1u64, &bounded_id).expect("endpoint exists");
        assert_eq!(se.id.to_vec(), b"#rpc".to_vec());
        assert_eq!(se.service_type.to_vec(), b"JsonRpcService".to_vec());
        assert_eq!(
            se.endpoint.to_vec(),
            b"https://node.claw.network/rpc".to_vec()
        );
    });
}

#[test]
fn add_service_endpoint_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#storage".to_vec(),
            b"IPFS".to_vec(),
            b"ipfs://Qm...".to_vec(),
        ));
        System::assert_last_event(
            crate::pallet::Event::ServiceEndpointAdded {
                controller: 1u64,
                endpoint_id: b"#storage".to_vec(),
            }
            .into(),
        );
    });
}

#[test]
fn add_service_endpoint_fails_duplicate_id() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#rpc".to_vec(),
            b"T".to_vec(),
            b"https://a".to_vec(),
        ));
        assert_noop!(
            AgentDID::add_service_endpoint(
                signed(1),
                b"#rpc".to_vec(),
                b"T".to_vec(),
                b"https://b".to_vec(),
            ),
            crate::pallet::Error::<Test>::ServiceEndpointAlreadyExists
        );
    });
}

#[test]
fn add_service_endpoint_fails_on_deactivated_did() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::deactivate_did(signed(1)));
        assert_noop!(
            AgentDID::add_service_endpoint(
                signed(1),
                b"#rpc".to_vec(),
                b"T".to_vec(),
                b"https://a".to_vec(),
            ),
            crate::pallet::Error::<Test>::DIDDeactivated
        );
    });
}

#[test]
fn add_service_endpoint_fails_too_many() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        for i in 0u8..10 {
            let id = alloc::format!("#{}", i).into_bytes();
            assert_ok!(AgentDID::add_service_endpoint(
                signed(1),
                id,
                b"T".to_vec(),
                b"http://x".to_vec(),
            ));
        }
        assert_noop!(
            AgentDID::add_service_endpoint(
                signed(1),
                b"#overflow".to_vec(),
                b"T".to_vec(),
                b"http://x".to_vec(),
            ),
            crate::pallet::Error::<Test>::TooManyServiceEndpoints
        );
    });
}

// ========================= remove_service_endpoint =========================

#[test]
fn remove_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#rpc".to_vec(),
            b"T".to_vec(),
            b"https://node".to_vec(),
        ));
        assert_ok!(AgentDID::remove_service_endpoint(
            signed(1),
            b"#rpc".to_vec()
        ));

        let doc = DIDDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.service_endpoint_count, 0);

        System::assert_last_event(
            crate::pallet::Event::ServiceEndpointRemoved {
                controller: 1u64,
                endpoint_id: b"#rpc".to_vec(),
            }
            .into(),
        );
    });
}

#[test]
fn remove_service_endpoint_fails_not_found() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_noop!(
            AgentDID::remove_service_endpoint(signed(1), b"#missing".to_vec()),
            crate::pallet::Error::<Test>::ServiceEndpointNotFound
        );
    });
}

#[test]
fn remove_service_endpoint_fails_on_deactivated_did() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#rpc".to_vec(),
            b"T".to_vec(),
            b"http://x".to_vec(),
        ));
        assert_ok!(AgentDID::deactivate_did(signed(1)));
        assert_noop!(
            AgentDID::remove_service_endpoint(signed(1), b"#rpc".to_vec()),
            crate::pallet::Error::<Test>::DIDDeactivated
        );
    });
}

// ========================= multi-account isolation =========================

#[test]
fn multiple_accounts_have_independent_dids() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"ctx-1".to_vec()));
        assert_ok!(AgentDID::register_did(signed(2), b"ctx-2".to_vec()));

        let doc1 = DIDDocuments::<Test>::get(1u64).unwrap();
        let doc2 = DIDDocuments::<Test>::get(2u64).unwrap();
        assert_eq!(doc1.context.to_vec(), b"ctx-1".to_vec());
        assert_eq!(doc2.context.to_vec(), b"ctx-2".to_vec());
        assert_eq!(DIDCount::<Test>::get(), 2);

        assert_ok!(AgentDID::deactivate_did(signed(1)));
        assert_eq!(DIDCount::<Test>::get(), 1);
        assert!(DIDDocuments::<Test>::get(1u64).unwrap().deactivated);
        assert!(!DIDDocuments::<Test>::get(2u64).unwrap().deactivated);
    });
}

#[test]
fn service_endpoints_are_account_scoped() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDID::register_did(signed(1), b"".to_vec()));
        assert_ok!(AgentDID::register_did(signed(2), b"".to_vec()));

        assert_ok!(AgentDID::add_service_endpoint(
            signed(1),
            b"#rpc".to_vec(),
            b"T".to_vec(),
            b"https://node1".to_vec(),
        ));
        assert_ok!(AgentDID::add_service_endpoint(
            signed(2),
            b"#rpc".to_vec(),
            b"T".to_vec(),
            b"https://node2".to_vec(),
        ));

        assert_eq!(
            DIDDocuments::<Test>::get(1u64)
                .unwrap()
                .service_endpoint_count,
            1
        );
        assert_eq!(
            DIDDocuments::<Test>::get(2u64)
                .unwrap()
                .service_endpoint_count,
            1
        );
    });
}
