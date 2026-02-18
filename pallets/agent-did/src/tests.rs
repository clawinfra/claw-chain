//! Unit tests for the Agent DID pallet.

use crate as pallet_agent_did;
use crate::pallet::{DidDocuments, DidStatus, ServiceEndpointCount, ServiceEndpoints};
use frame_support::{
    assert_noop, assert_ok, derive_impl,
    traits::ConstU32,
};
use sp_runtime::BuildStorage;

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
    type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
}

impl pallet_agent_did::Config for Test {
    type WeightInfo = ();
    type MaxServiceEndpoints = ConstU32<5>;
    type MaxServiceTypeLength = ConstU32<64>;
    type MaxServiceUrlLength = ConstU32<256>;
    type MaxMetadataLength = ConstU32<1024>;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn origin(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

#[test]
fn register_did_works() {
    new_test_ext().execute_with(|| {
        let metadata = b"{\"name\":\"Alice\"}".to_vec();
        assert_ok!(AgentDid::register_did(origin(1), metadata.clone()));

        let doc = DidDocuments::<Test>::get(1u64).expect("DID should exist");
        assert_eq!(doc.controller, 1u64);
        assert_eq!(doc.metadata.to_vec(), metadata);
        assert_eq!(doc.status, DidStatus::Active);
        assert_eq!(doc.registered_at, 1);
        assert_eq!(doc.next_service_id, 0);
    });
}

#[test]
fn register_did_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        System::assert_last_event(
            crate::pallet::Event::DidRegistered { who: 1u64 }.into(),
        );
    });
}

#[test]
fn register_did_fails_if_already_registered() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_noop!(
            AgentDid::register_did(origin(1), b"{}".to_vec()),
            crate::Error::<Test>::DidAlreadyRegistered
        );
    });
}

#[test]
fn register_did_fails_with_too_long_metadata() {
    new_test_ext().execute_with(|| {
        let metadata = vec![b'x'; 1025];
        assert_noop!(
            AgentDid::register_did(origin(1), metadata),
            crate::Error::<Test>::MetadataTooLong
        );
    });
}

#[test]
fn update_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        System::set_block_number(5);
        let new_meta = b"{\"updated\":true}".to_vec();
        assert_ok!(AgentDid::update_did(origin(1), new_meta.clone()));
        let doc = DidDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.metadata.to_vec(), new_meta);
        assert_eq!(doc.updated_at, 5);
    });
}

#[test]
fn update_did_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AgentDid::update_did(origin(99), b"{}".to_vec()),
            crate::Error::<Test>::DidNotFound
        );
    });
}

#[test]
fn update_did_fails_after_deactivation() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::deactivate_did(origin(1)));
        assert_noop!(
            AgentDid::update_did(origin(1), b"{\"new\":1}".to_vec()),
            crate::Error::<Test>::DidDeactivated
        );
    });
}

#[test]
fn deactivate_did_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::deactivate_did(origin(1)));
        let doc = DidDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.status, DidStatus::Deactivated);
    });
}

#[test]
fn deactivate_did_fails_if_already_deactivated() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::deactivate_did(origin(1)));
        assert_noop!(
            AgentDid::deactivate_did(origin(1)),
            crate::Error::<Test>::DidDeactivated
        );
    });
}

#[test]
fn add_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::add_service_endpoint(
            origin(1),
            b"AgentMessaging".to_vec(),
            b"https://agent.example.com/msg".to_vec(),
        ));
        let ep = ServiceEndpoints::<Test>::get(1u64, 0u32).expect("endpoint should exist");
        assert_eq!(ep.service_type.to_vec(), b"AgentMessaging".to_vec());
        assert_eq!(ep.service_url.to_vec(), b"https://agent.example.com/msg".to_vec());
        assert_eq!(ServiceEndpointCount::<Test>::get(1u64), 1);
        let doc = DidDocuments::<Test>::get(1u64).unwrap();
        assert_eq!(doc.next_service_id, 1);
    });
}

#[test]
fn add_service_endpoint_fails_on_too_many() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        for i in 0..5u32 {
            let url = format!("https://ep{}.example.com", i).into_bytes();
            assert_ok!(AgentDid::add_service_endpoint(origin(1), b"RpcNode".to_vec(), url));
        }
        assert_noop!(
            AgentDid::add_service_endpoint(
                origin(1),
                b"RpcNode".to_vec(),
                b"https://overflow.example.com".to_vec(),
            ),
            crate::Error::<Test>::TooManyServiceEndpoints
        );
    });
}

#[test]
fn remove_service_endpoint_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::add_service_endpoint(
            origin(1), b"AgentMessaging".to_vec(), b"https://agent.example.com/msg".to_vec(),
        ));
        assert_eq!(ServiceEndpointCount::<Test>::get(1u64), 1);
        assert_ok!(AgentDid::remove_service_endpoint(origin(1), 0u32));
        assert!(ServiceEndpoints::<Test>::get(1u64, 0u32).is_none());
        assert_eq!(ServiceEndpointCount::<Test>::get(1u64), 0);
    });
}

#[test]
fn remove_service_endpoint_fails_if_not_found() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_noop!(
            AgentDid::remove_service_endpoint(origin(1), 42u32),
            crate::Error::<Test>::ServiceEndpointNotFound
        );
    });
}

#[test]
fn add_service_endpoint_fails_on_deactivated_did() {
    new_test_ext().execute_with(|| {
        assert_ok!(AgentDid::register_did(origin(1), b"{}".to_vec()));
        assert_ok!(AgentDid::deactivate_did(origin(1)));
        assert_noop!(
            AgentDid::add_service_endpoint(
                origin(1), b"RpcNode".to_vec(), b"https://example.com".to_vec(),
            ),
            crate::Error::<Test>::DidDeactivated
        );
    });
}

#[test]
fn multiple_accounts_can_register_dids() {
    new_test_ext().execute_with(|| {
        for i in 1u64..=4 {
            let meta = format!("{{\"id\":{}}}", i).into_bytes();
            assert_ok!(AgentDid::register_did(origin(i), meta));
        }
        for i in 1u64..=4 {
            assert!(DidDocuments::<Test>::get(i).is_some());
        }
    });
}
