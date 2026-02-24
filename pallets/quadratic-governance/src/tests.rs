use crate::pallet::*;
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64},
};
use sp_runtime::BuildStorage;

// =========================================================
// Mock runtime
// =========================================================

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        AgentDid: pallet_agent_did,
        QuadraticGovernance: crate,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u128>;
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
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

parameter_types! {
    pub const MinProposalDeposit: u128 = 100;
    pub const VotingPeriod: u64 = 100;  // 100 blocks
    pub const MinQuorumPct: u32 = 10;   // require >= 10 total vote-weight
}

impl crate::pallet::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MinProposalDeposit = MinProposalDeposit;
    type VotingPeriod = VotingPeriod;
    type MinQuorumPct = MinQuorumPct;
    type WeightInfo = ();
}

// =========================================================
// Helpers
// =========================================================

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);

        // Fund accounts 1, 2, 3, 4
        pallet_balances::Pallet::<Test>::force_set_balance(RuntimeOrigin::root(), 1, 10_000)
            .unwrap();
        pallet_balances::Pallet::<Test>::force_set_balance(RuntimeOrigin::root(), 2, 10_000)
            .unwrap();
        pallet_balances::Pallet::<Test>::force_set_balance(RuntimeOrigin::root(), 3, 10_000)
            .unwrap();
        pallet_balances::Pallet::<Test>::force_set_balance(RuntimeOrigin::root(), 4, 10_000)
            .unwrap();

        // Register DIDs for 1, 2, 3 (NOT 4 — used for "no DID" tests)
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(1),
            alloc::vec![]
        ));
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(2),
            alloc::vec![]
        ));
        assert_ok!(AgentDid::register_did(
            RuntimeOrigin::signed(3),
            alloc::vec![]
        ));
    });
    ext
}

fn desc_hash() -> [u8; 32] {
    [42u8; 32]
}

extern crate alloc;

// =========================================================
// Tests
// =========================================================

// 1. submit_proposal works
#[test]
fn submit_proposal_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash(),
        ));

        let proposal = QuadraticGovernance::proposals(0).expect("proposal should exist");
        assert_eq!(proposal.proposer, 1);
        assert_eq!(proposal.description_hash, desc_hash());
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.start_block, 1);
        assert_eq!(proposal.end_block, 101); // 1 + 100
        assert_eq!(proposal.yes_votes, 0);
        assert_eq!(proposal.no_votes, 0);
        assert_eq!(proposal.deposit, 100);

        // Deposit was reserved
        assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&1), 100);

        // Next ID incremented
        assert_eq!(QuadraticGovernance::next_proposal_id(), 1);
        assert_eq!(QuadraticGovernance::proposal_count(), 1);

        // Event emitted
        System::assert_last_event(RuntimeEvent::QuadraticGovernance(
            Event::ProposalSubmitted {
                proposal_id: 0,
                proposer: 1,
                description_hash: desc_hash(),
            },
        ));
    });
}

// 2. submit_proposal fails without DID
#[test]
fn submit_proposal_fails_without_did() {
    new_test_ext().execute_with(|| {
        // Account 4 has no DID
        assert_noop!(
            QuadraticGovernance::submit_proposal(RuntimeOrigin::signed(4), desc_hash()),
            Error::<Test>::NotRegistered
        );
    });
}

// 3. vote works and weight = sqrt(staked)
#[test]
fn vote_works_weight_is_sqrt() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash(),
        ));

        // Vote Yes with 100 staked → weight = sqrt(100) = 10
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(2),
            0,
            Vote::Yes,
            100,
        ));

        let record = QuadraticGovernance::votes(0, 2).expect("vote should exist");
        assert_eq!(record.vote, Vote::Yes);
        assert_eq!(record.weight, 10);
        assert_eq!(record.block, 1);

        let proposal = QuadraticGovernance::proposals(0).unwrap();
        assert_eq!(proposal.yes_votes, 10);
        assert_eq!(proposal.no_votes, 0);

        System::assert_last_event(RuntimeEvent::QuadraticGovernance(Event::Voted {
            proposal_id: 0,
            voter: 2,
            vote: Vote::Yes,
            weight: 10,
        }));
    });
}

// 4. cannot vote twice
#[test]
fn cannot_vote_twice() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(2),
            0,
            Vote::Yes,
            100
        ));

        assert_noop!(
            QuadraticGovernance::vote(RuntimeOrigin::signed(2), 0, Vote::No, 200),
            Error::<Test>::AlreadyVoted
        );
    });
}

// 5. cannot vote after period ends
#[test]
fn cannot_vote_after_period_ends() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // Advance past voting period (end_block = 101)
        System::set_block_number(102);

        assert_noop!(
            QuadraticGovernance::vote(RuntimeOrigin::signed(2), 0, Vote::Yes, 100),
            Error::<Test>::VotingEnded
        );
    });
}

// 6. finalize passes when yes > no + quorum met
#[test]
fn finalize_passes_when_yes_wins_and_quorum_met() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // 2 votes Yes with 100 (weight 10), 3 votes No with 25 (weight 5)
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(2),
            0,
            Vote::Yes,
            100
        ));
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(3),
            0,
            Vote::No,
            25
        ));

        // total_votes = 15, quorum = 10, 15 >= 10 ✓, yes(10) > no(5) ✓

        System::set_block_number(102);

        assert_ok!(QuadraticGovernance::finalize_proposal(
            RuntimeOrigin::signed(1),
            0
        ));

        let proposal = QuadraticGovernance::proposals(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Passed);

        // Deposit unreserved
        assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&1), 0);

        System::assert_last_event(RuntimeEvent::QuadraticGovernance(
            Event::ProposalFinalized {
                proposal_id: 0,
                status: ProposalStatus::Passed,
            },
        ));
    });
}

// 7. finalize rejects when no > yes
#[test]
fn finalize_rejects_when_no_wins() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // 2 votes Yes with 25 (weight 5), 3 votes No with 100 (weight 10)
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(2),
            0,
            Vote::Yes,
            25
        ));
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(3),
            0,
            Vote::No,
            100
        ));

        System::set_block_number(102);

        assert_ok!(QuadraticGovernance::finalize_proposal(
            RuntimeOrigin::signed(1),
            0
        ));

        let proposal = QuadraticGovernance::proposals(0).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    });
}

// 8. finalize fails when quorum not met
#[test]
fn finalize_fails_when_quorum_not_met() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // 1 vote with weight 3 (sqrt(9)=3). Quorum is 10. 3 < 10 → fail.
        assert_ok!(QuadraticGovernance::vote(
            RuntimeOrigin::signed(2),
            0,
            Vote::Yes,
            9
        ));

        System::set_block_number(102);

        assert_noop!(
            QuadraticGovernance::finalize_proposal(RuntimeOrigin::signed(1), 0),
            Error::<Test>::QuorumNotMet
        );
    });
}

// 9. cancel_proposal by proposer refunds deposit
#[test]
fn cancel_by_proposer_refunds_deposit() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));
        assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&1), 100);

        assert_ok!(QuadraticGovernance::cancel_proposal(
            RuntimeOrigin::signed(1),
            0
        ));

        // Deposit refunded
        assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&1), 0);

        // Proposal removed
        assert!(QuadraticGovernance::proposals(0).is_none());
        assert_eq!(QuadraticGovernance::proposal_count(), 0);

        System::assert_last_event(RuntimeEvent::QuadraticGovernance(
            Event::ProposalCancelled {
                proposal_id: 0,
                proposer: 1,
            },
        ));
    });
}

// 10. cancel by non-proposer fails
#[test]
fn cancel_by_non_proposer_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        assert_noop!(
            QuadraticGovernance::cancel_proposal(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotProposer
        );
    });
}

// 11. integer_sqrt correctness
#[test]
fn integer_sqrt_is_correct() {
    new_test_ext().execute_with(|| {
        assert_eq!(QuadraticGovernance::integer_sqrt(0), 0);
        assert_eq!(QuadraticGovernance::integer_sqrt(1), 1);
        assert_eq!(QuadraticGovernance::integer_sqrt(4), 2);
        assert_eq!(QuadraticGovernance::integer_sqrt(9), 3);
        assert_eq!(QuadraticGovernance::integer_sqrt(10), 3); // floor
        assert_eq!(QuadraticGovernance::integer_sqrt(100), 10);
        assert_eq!(QuadraticGovernance::integer_sqrt(10_000), 100);
        assert_eq!(QuadraticGovernance::integer_sqrt(1_000_000), 1_000);
        assert_eq!(
            QuadraticGovernance::integer_sqrt(u128::MAX),
            18_446_744_073_709_551_615
        );
    });
}

// 12. vote fails without DID
#[test]
fn vote_fails_without_did() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // Account 4 has no DID
        assert_noop!(
            QuadraticGovernance::vote(RuntimeOrigin::signed(4), 0, Vote::Yes, 100),
            Error::<Test>::NotRegistered
        );
    });
}

// 13. finalize fails while still active
#[test]
fn finalize_fails_while_still_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(QuadraticGovernance::submit_proposal(
            RuntimeOrigin::signed(1),
            desc_hash()
        ));

        // Don't advance time
        assert_noop!(
            QuadraticGovernance::finalize_proposal(RuntimeOrigin::signed(1), 0),
            Error::<Test>::ProposalStillActive
        );
    });
}
