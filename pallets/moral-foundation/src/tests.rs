//! Unit tests for `pallet-moral-foundation`.

use crate::{self as pallet_moral_foundation, pallet::*, AgentRegistryInterface};
use frame_support::{
    assert_noop, assert_ok, parameter_types,
    traits::{ConstU32, ConstU64},
    BoundedVec,
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

// =========================================================
// Mock runtime
// =========================================================

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        MoralFoundation: pallet_moral_foundation,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
}

// =========================================================
// Mock AgentRegistry
//
// Rules:
//   - AccountId 1  => DID b"did:claw:agent1"  registered, controller = 1
//   - AccountId 2  => DID b"did:claw:agent2"  registered, controller = 2
//   - AccountId 99 => no DID
// =========================================================

fn did1() -> BoundedVec<u8, ConstU32<128>> {
    BoundedVec::try_from(b"did:claw:agent1".to_vec()).expect("fits")
}

fn did2() -> BoundedVec<u8, ConstU32<128>> {
    BoundedVec::try_from(b"did:claw:agent2".to_vec()).expect("fits")
}

fn unknown_did() -> BoundedVec<u8, ConstU32<128>> {
    BoundedVec::try_from(b"did:claw:ghost".to_vec()).expect("fits")
}

pub struct MockRegistry;

impl AgentRegistryInterface<u64, ConstU32<128>> for MockRegistry {
    fn is_registered(did: &BoundedVec<u8, ConstU32<128>>) -> bool {
        did.as_slice() == b"did:claw:agent1" || did.as_slice() == b"did:claw:agent2"
    }

    fn is_controller(did: &BoundedVec<u8, ConstU32<128>>, controller: &u64) -> bool {
        (did.as_slice() == b"did:claw:agent1" && *controller == 1)
            || (did.as_slice() == b"did:claw:agent2" && *controller == 2)
    }
}

// =========================================================
// Pallet config
// =========================================================

parameter_types! {
    pub const VotingPeriod: u64 = 50_400;
}

impl pallet_moral_foundation::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxDidLength = ConstU32<128>;
    type VotingPeriod = ConstU64<50_400>;
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type AgentRegistry = MockRegistry;
}

// =========================================================
// Test helpers
// =========================================================

fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("genesis build succeeds");

    // Seed the moral framework hash.
    pallet_moral_foundation::GenesisConfig::<Test>::default()
        .assimilate_storage(&mut storage)
        .expect("moral-foundation genesis build succeeds");

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// =========================================================
// attest_to_framework tests
// =========================================================

#[test]
fn attest_to_framework_works() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        let record = AgentAttestation::<Test>::get(&did).expect("record stored");
        assert!(record.attested);
        assert_eq!(record.attested_at, 1);
        assert_eq!(record.framework_hash, MoralFramework::<Test>::get());

        // Empathy score initialised to 500.
        assert_eq!(EmpathyScore::<Test>::get(&did), 500);
    });
}

#[test]
fn attest_sets_empathy_only_on_first_call() {
    new_test_ext().execute_with(|| {
        let did = did1();

        // First attestation sets score to 500.
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));
        assert_eq!(EmpathyScore::<Test>::get(&did), 500);

        // Governance changes score to 800.
        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did.clone(),
            800
        ));
        assert_eq!(EmpathyScore::<Test>::get(&did), 800);

        // Attesting again after a framework update should NOT reset the score.
        // Simulate a new framework hash being set.
        let new_hash = H256::from([0x42u8; 32]);
        MoralFramework::<Test>::put(new_hash);

        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));
        // Score should remain 800.
        assert_eq!(EmpathyScore::<Test>::get(&did), 800);
    });
}

#[test]
fn attest_fails_for_unregistered_did() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            MoralFoundation::attest_to_framework(RuntimeOrigin::signed(99), unknown_did()),
            Error::<Test>::NotRegisteredAgent
        );
    });
}

#[test]
fn attest_fails_for_wrong_controller() {
    new_test_ext().execute_with(|| {
        // AccountId 2 tries to attest using did1 which belongs to AccountId 1.
        assert_noop!(
            MoralFoundation::attest_to_framework(RuntimeOrigin::signed(2), did1()),
            Error::<Test>::NotAgentController
        );
    });
}

#[test]
fn attest_fails_if_already_attested_same_hash() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));
        // Second call with same framework hash should fail.
        assert_noop!(
            MoralFoundation::attest_to_framework(RuntimeOrigin::signed(1), did),
            Error::<Test>::AlreadyAttested
        );
    });
}

#[test]
fn attest_allowed_after_framework_update() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        // Update framework hash — should allow re-attestation.
        let new_hash = H256::from([0xABu8; 32]);
        MoralFramework::<Test>::put(new_hash);

        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        let record = AgentAttestation::<Test>::get(&did).expect("record stored");
        assert_eq!(record.framework_hash, new_hash);
    });
}

#[test]
fn attest_emits_framework_attested_event() {
    new_test_ext().execute_with(|| {
        let did = did1();
        let hash = MoralFramework::<Test>::get();

        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        System::assert_last_event(RuntimeEvent::MoralFoundation(Event::FrameworkAttested {
            agent_did: did,
            framework_hash: hash,
            at_block: 1,
        }));
    });
}

// =========================================================
// update_empathy_score tests
// =========================================================

#[test]
fn update_empathy_score_works() {
    new_test_ext().execute_with(|| {
        let did = did1();

        // Attest first.
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did.clone(),
            750
        ));
        assert_eq!(EmpathyScore::<Test>::get(&did), 750);
    });
}

#[test]
fn update_empathy_score_to_zero_and_max() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did.clone(),
            0
        ));
        assert_eq!(EmpathyScore::<Test>::get(&did), 0);

        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did.clone(),
            1000
        ));
        assert_eq!(EmpathyScore::<Test>::get(&did), 1000);
    });
}

#[test]
fn update_empathy_score_fails_out_of_range() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        assert_noop!(
            MoralFoundation::update_empathy_score(RuntimeOrigin::root(), did, 1001),
            Error::<Test>::ScoreOutOfRange
        );
    });
}

#[test]
fn update_empathy_score_fails_if_not_attested() {
    new_test_ext().execute_with(|| {
        // did1 never attested.
        assert_noop!(
            MoralFoundation::update_empathy_score(RuntimeOrigin::root(), did1(), 500),
            Error::<Test>::NotAttested
        );
    });
}

#[test]
fn update_empathy_score_requires_governance_origin() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        // Non-root signed origin should be rejected.
        assert_noop!(
            MoralFoundation::update_empathy_score(RuntimeOrigin::signed(1), did, 600),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn update_empathy_score_emits_event() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));
        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did.clone(),
            900
        ));
        System::assert_last_event(RuntimeEvent::MoralFoundation(Event::EmpathyScoreUpdated {
            agent_did: did,
            new_score: 900,
        }));
    });
}

// =========================================================
// propose_framework_amendment tests
// =========================================================

#[test]
fn propose_framework_amendment_works() {
    new_test_ext().execute_with(|| {
        let did = did1();

        // Attest first.
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        let new_hash = H256::from([0x11u8; 32]);
        let desc: BoundedVec<u8, ConstU32<1024>> =
            BoundedVec::try_from(b"Add clause 6: agents pay taxes".to_vec()).expect("fits");

        assert_ok!(MoralFoundation::propose_framework_amendment(
            RuntimeOrigin::signed(1),
            did.clone(),
            new_hash,
            desc
        ));

        // At least one amendment should be stored.
        let mut found = false;
        PendingAmendments::<Test>::iter().for_each(|(_k, v)| {
            if v.new_framework_hash == new_hash {
                found = true;
                assert_eq!(v.vote_closes_at, 1 + 50_400);
            }
        });
        assert!(found, "amendment not found in storage");
    });
}

#[test]
fn propose_amendment_fails_for_unregistered_did() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            MoralFoundation::propose_framework_amendment(
                RuntimeOrigin::signed(99),
                unknown_did(),
                H256::from([0x22u8; 32]),
                BoundedVec::try_from(b"desc".to_vec()).expect("fits"),
            ),
            Error::<Test>::NotRegisteredAgent
        );
    });
}

#[test]
fn propose_amendment_fails_for_wrong_controller() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            MoralFoundation::propose_framework_amendment(
                RuntimeOrigin::signed(2),
                did1(),
                H256::from([0x33u8; 32]),
                BoundedVec::try_from(b"desc".to_vec()).expect("fits"),
            ),
            Error::<Test>::NotAgentController
        );
    });
}

#[test]
fn propose_amendment_fails_if_not_attested() {
    new_test_ext().execute_with(|| {
        // Registered but never attested.
        assert_noop!(
            MoralFoundation::propose_framework_amendment(
                RuntimeOrigin::signed(1),
                did1(),
                H256::from([0x44u8; 32]),
                BoundedVec::try_from(b"desc".to_vec()).expect("fits"),
            ),
            Error::<Test>::NotAttested
        );
    });
}

#[test]
fn propose_amendment_emits_event() {
    new_test_ext().execute_with(|| {
        let did = did1();
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did.clone()
        ));

        let new_hash = H256::from([0x55u8; 32]);
        let desc: BoundedVec<u8, ConstU32<1024>> =
            BoundedVec::try_from(b"Update wording".to_vec()).expect("fits");

        assert_ok!(MoralFoundation::propose_framework_amendment(
            RuntimeOrigin::signed(1),
            did,
            new_hash,
            desc,
        ));

        // The last event should be AmendmentProposed.
        let events = System::events();
        let last = &events.last().expect("at least one event").event;
        match last {
            RuntimeEvent::MoralFoundation(Event::AmendmentProposed {
                new_framework_hash,
                vote_closes_at,
                ..
            }) => {
                assert_eq!(*new_framework_hash, new_hash);
                assert_eq!(*vote_closes_at, 1 + 50_400);
            }
            other => panic!("unexpected event: {:?}", other),
        }
    });
}

// =========================================================
// Genesis tests
// =========================================================

#[test]
fn genesis_sets_default_framework_hash() {
    new_test_ext().execute_with(|| {
        let stored = MoralFramework::<Test>::get();
        // The RFC-specified hash.
        // Padded to 64 hex chars (RFC value was 63 chars, leading zero added).
        let expected = H256::from(crate::hex_literal(
            b"08d4f9a2c1b3e7f60a5d8c2e94b1f3a7d6c2e8f40b5d9a1c3e7f2b60a4d8c1e9",
        ));
        assert_eq!(stored, expected);
    });
}

#[test]
fn genesis_accepts_custom_framework_hash() {
    let custom = H256::from([0xDEu8; 32]);

    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("genesis");

    pallet_moral_foundation::GenesisConfig::<Test> {
        initial_framework_hash: Some(custom),
        ..Default::default()
    }
    .assimilate_storage(&mut storage)
    .expect("custom genesis");

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| {
        assert_eq!(MoralFramework::<Test>::get(), custom);
    });
}

// =========================================================
// Multi-agent tests
// =========================================================

#[test]
fn multiple_agents_can_attest_independently() {
    new_test_ext().execute_with(|| {
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did1()
        ));
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(2),
            did2()
        ));

        assert!(AgentAttestation::<Test>::get(&did1()).is_some());
        assert!(AgentAttestation::<Test>::get(&did2()).is_some());

        assert_eq!(EmpathyScore::<Test>::get(&did1()), 500);
        assert_eq!(EmpathyScore::<Test>::get(&did2()), 500);
    });
}

#[test]
fn empathy_score_independent_per_agent() {
    new_test_ext().execute_with(|| {
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(1),
            did1()
        ));
        assert_ok!(MoralFoundation::attest_to_framework(
            RuntimeOrigin::signed(2),
            did2()
        ));

        assert_ok!(MoralFoundation::update_empathy_score(
            RuntimeOrigin::root(),
            did1(),
            900
        ));

        // did2 score unchanged.
        assert_eq!(EmpathyScore::<Test>::get(&did1()), 900);
        assert_eq!(EmpathyScore::<Test>::get(&did2()), 500);
    });
}
