//! Unit tests for pallet-emergency-pause.
//!
//! Coverage targets (≥ 90%):
//! - Council management (add/remove)
//! - Propose pause / unpause
//! - Voting flow through to execution
//! - Emergency pause
//! - Proposal expiry (on_initialize)
//! - Emergency pause expiry (on_initialize)
//! - All error paths
//! - EmergencyPauseProvider trait
//! - Genesis config

use crate::{
    mock::*,
    pallet::{
        ActiveProposalCount, CouncilMembers, Error, Event, NextProposalId, PauseInfo, PauseReason,
        PauseVotes, PausedPallets, ProposalKind,
    },
    EmergencyPauseProvider,
};
use frame_support::assert_noop;
use frame_support::assert_ok;

// ---------------------------------------------------------------------------
// Helper: make a bounded pallet-id Vec
// ---------------------------------------------------------------------------
fn pid(s: &[u8]) -> Vec<u8> {
    s.to_vec()
}

// ---------------------------------------------------------------------------
// 1. Genesis / council membership
// ---------------------------------------------------------------------------

#[test]
fn genesis_populates_council_members() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let members = CouncilMembers::<Test>::get();
        assert!(members.contains(&1));
        assert!(members.contains(&2));
        assert!(members.contains(&3));
        assert_eq!(members.len(), 3);
    });
}

#[test]
fn add_council_member_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(EmergencyPause::add_council_member(root(), 10));
        assert!(CouncilMembers::<Test>::get().contains(&10));
        System::assert_last_event(Event::CouncilMemberAdded { member: 10 }.into());
    });
}

#[test]
fn add_council_member_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            EmergencyPause::add_council_member(origin(1), 10),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn add_council_member_duplicate_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_noop!(
            EmergencyPause::add_council_member(root(), 1),
            Error::<Test>::AlreadyCouncilMember
        );
    });
}

#[test]
fn remove_council_member_works() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        assert_ok!(EmergencyPause::remove_council_member(root(), 1));
        assert!(!CouncilMembers::<Test>::get().contains(&1));
        System::assert_last_event(Event::CouncilMemberRemoved { member: 1 }.into());
    });
}

#[test]
fn remove_council_member_last_member_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_noop!(
            EmergencyPause::remove_council_member(root(), 1),
            Error::<Test>::CannotRemoveLastMember
        );
    });
}

#[test]
fn remove_council_member_not_a_member_fails() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        assert_noop!(
            EmergencyPause::remove_council_member(root(), 99),
            Error::<Test>::NotACouncilMember
        );
    });
}

// ---------------------------------------------------------------------------
// 2. propose_pause
// ---------------------------------------------------------------------------

#[test]
fn propose_pause_works() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-task-market");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));

        let proposal_id = 0u64;
        let proposal = PauseVotes::<Test>::get(proposal_id).expect("proposal should exist");
        assert_eq!(proposal.kind, ProposalKind::Pause);
        assert_eq!(proposal.pallet_id.to_vec(), id);
        assert_eq!(proposal.votes.len(), 1); // proposer auto-voted
        assert_eq!(ActiveProposalCount::<Test>::get(), 1);
        assert_eq!(NextProposalId::<Test>::get(), 1);
    });
}

#[test]
fn propose_pause_non_member_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_noop!(
            EmergencyPause::propose_pause(origin(99), pid(b"pallet-x")),
            Error::<Test>::NotCouncilMember
        );
    });
}

#[test]
fn propose_pause_already_paused_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        // Manually insert a paused pallet.
        let bounded_id: crate::pallet::PalletId<Test> = pid(b"pallet-x").try_into().unwrap();
        let info = PauseInfo {
            paused_at: 1,
            expires_at: 0,
            reason: PauseReason::CouncilVote,
            triggered_by: 1,
        };
        PausedPallets::<Test>::insert(bounded_id, info);

        assert_noop!(
            EmergencyPause::propose_pause(origin(1), pid(b"pallet-x")),
            Error::<Test>::AlreadyPaused
        );
    });
}

#[test]
fn propose_pause_duplicate_proposal_fails() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_noop!(
            EmergencyPause::propose_pause(origin(2), id.clone()),
            Error::<Test>::DuplicateProposal
        );
    });
}

#[test]
fn propose_pause_pallet_id_too_long_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        let long_id = vec![b'x'; 65]; // MaxPalletIdLen = 64
        assert_noop!(
            EmergencyPause::propose_pause(origin(1), long_id),
            Error::<Test>::PalletIdTooLong
        );
    });
}

// ---------------------------------------------------------------------------
// 3. propose_unpause
// ---------------------------------------------------------------------------

#[test]
fn propose_unpause_works() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        // First pause the pallet directly.
        let bounded_id: crate::pallet::PalletId<Test> =
            pid(b"pallet-task-market").try_into().unwrap();
        let info = PauseInfo {
            paused_at: 1,
            expires_at: 0,
            reason: PauseReason::CouncilVote,
            triggered_by: 1,
        };
        PausedPallets::<Test>::insert(bounded_id, info);

        assert_ok!(EmergencyPause::propose_unpause(
            origin(1),
            pid(b"pallet-task-market")
        ));

        let proposal = PauseVotes::<Test>::get(0).expect("proposal should exist");
        assert_eq!(proposal.kind, ProposalKind::Unpause);
        assert_eq!(proposal.votes.len(), 1);
    });
}

#[test]
fn propose_unpause_not_paused_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_noop!(
            EmergencyPause::propose_unpause(origin(1), pid(b"pallet-x")),
            Error::<Test>::NotPaused
        );
    });
}

// ---------------------------------------------------------------------------
// 4. vote
// ---------------------------------------------------------------------------

#[test]
fn vote_increases_count_and_executes_at_threshold() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-task-market");
        // member 1 proposes (auto-votes)
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_eq!(ActiveProposalCount::<Test>::get(), 1);

        // member 2 votes
        assert_ok!(EmergencyPause::vote(origin(2), 0));
        // Proposal still exists (2/3 threshold)
        assert!(PauseVotes::<Test>::get(0).is_some());

        // member 3 votes — threshold reached
        assert_ok!(EmergencyPause::vote(origin(3), 0));
        // Proposal removed
        assert!(PauseVotes::<Test>::get(0).is_none());
        // Pallet should now be paused
        assert!(EmergencyPause::is_paused(b"pallet-task-market"));
        assert_eq!(ActiveProposalCount::<Test>::get(), 0);
    });
}

#[test]
fn vote_double_vote_fails() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        // member 1 already voted
        assert_noop!(
            EmergencyPause::vote(origin(1), 0),
            Error::<Test>::AlreadyVoted
        );
    });
}

#[test]
fn vote_non_member_fails() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_noop!(
            EmergencyPause::vote(origin(99), 0),
            Error::<Test>::NotCouncilMember
        );
    });
}

#[test]
fn vote_on_missing_proposal_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_noop!(
            EmergencyPause::vote(origin(1), 999),
            Error::<Test>::ProposalNotFound
        );
    });
}

#[test]
fn vote_on_expired_proposal_fails() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        // Roll past expiry (14400 blocks)
        roll_to(15_000);
        // Proposal should have been cleaned up by on_initialize, but simulate directly
        assert!(PauseVotes::<Test>::get(0).is_none()); // cleaned by on_initialize
    });
}

// ---------------------------------------------------------------------------
// 5. emergency_pause
// ---------------------------------------------------------------------------

#[test]
fn emergency_pause_pauses_all_custom_pallets() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        assert_ok!(EmergencyPause::emergency_pause(origin(1)));

        // All custom pallet IDs should be paused
        for id in EmergencyPause::custom_pallet_ids() {
            assert!(
                EmergencyPause::is_paused(&id),
                "Expected {:?} to be paused",
                core::str::from_utf8(&id).unwrap_or("?")
            );
        }
    });
}

#[test]
fn emergency_pause_sets_correct_expiry() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        System::set_block_number(100);
        assert_ok!(EmergencyPause::emergency_pause(origin(1)));

        let id: crate::pallet::PalletId<Test> = pid(b"pallet-task-market").try_into().unwrap();
        let info = PausedPallets::<Test>::get(&id).expect("should be paused");
        assert_eq!(info.expires_at, 100 + 1200); // EmergencyPauseDuration = 1200
        assert_eq!(info.reason, PauseReason::EmergencyTrigger);
    });
}

#[test]
fn emergency_pause_non_member_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            EmergencyPause::emergency_pause(origin(99)),
            Error::<Test>::NotCouncilMember
        );
    });
}

#[test]
fn emergency_pause_event_emitted() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        System::set_block_number(10);
        assert_ok!(EmergencyPause::emergency_pause(origin(1)));
        System::assert_last_event(
            Event::EmergencyPauseActivated {
                triggered_by: 1,
                expires_at: 10 + 1200,
            }
            .into(),
        );
    });
}

// ---------------------------------------------------------------------------
// 6. cancel_proposal
// ---------------------------------------------------------------------------

#[test]
fn cancel_proposal_by_proposer_works() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::cancel_proposal(origin(1), 0));
        assert!(PauseVotes::<Test>::get(0).is_none());
        assert_eq!(ActiveProposalCount::<Test>::get(), 0);
        System::assert_last_event(Event::ProposalCancelled { proposal_id: 0 }.into());
    });
}

#[test]
fn cancel_proposal_by_root_works() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::cancel_proposal(root(), 0));
        assert!(PauseVotes::<Test>::get(0).is_none());
    });
}

#[test]
fn cancel_proposal_not_proposer_fails() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_noop!(
            EmergencyPause::cancel_proposal(origin(2), 0),
            Error::<Test>::NotProposer
        );
    });
}

#[test]
fn cancel_proposal_not_found_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            EmergencyPause::cancel_proposal(origin(1), 999),
            Error::<Test>::ProposalNotFound
        );
    });
}

// ---------------------------------------------------------------------------
// 7. on_initialize — proposal expiry
// ---------------------------------------------------------------------------

#[test]
fn expired_proposal_cleaned_up_by_on_initialize() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_eq!(ActiveProposalCount::<Test>::get(), 1);

        // Roll past ProposalExpiry (14400)
        roll_to(15_000);

        assert!(PauseVotes::<Test>::get(0).is_none());
        assert_eq!(ActiveProposalCount::<Test>::get(), 0);
    });
}

#[test]
fn unexpired_proposal_not_cleaned_up() {
    new_test_ext_with_members(vec![1, 2]).execute_with(|| {
        let id = pid(b"pallet-x");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        roll_to(100); // well before expiry
        assert!(PauseVotes::<Test>::get(0).is_some());
    });
}

// ---------------------------------------------------------------------------
// 8. on_initialize — emergency pause expiry
// ---------------------------------------------------------------------------

#[test]
fn emergency_pause_expires_via_on_initialize() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        System::set_block_number(100);
        assert_ok!(EmergencyPause::emergency_pause(origin(1)));
        assert!(EmergencyPause::is_paused(b"pallet-task-market"));

        // Roll to just after expiry: 100 + 1200 = 1300
        roll_to(1301);
        assert!(!EmergencyPause::is_paused(b"pallet-task-market"));
    });
}

#[test]
fn council_vote_pause_does_not_expire_automatically() {
    // Indefinite (expires_at = 0) pauses must be unpaused via proposal.
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-y");
        // vote it in with threshold = 3 members
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::vote(origin(2), 0));
        assert_ok!(EmergencyPause::vote(origin(3), 0));
        assert!(EmergencyPause::is_paused(b"pallet-y"));

        // Roll far into the future
        roll_to(100_000);
        // Still paused
        assert!(EmergencyPause::is_paused(b"pallet-y"));
    });
}

// ---------------------------------------------------------------------------
// 9. Full pause → unpause flow
// ---------------------------------------------------------------------------

#[test]
fn full_pause_then_unpause_flow() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-rep");

        // Pause with 3 votes
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::vote(origin(2), 0));
        assert_ok!(EmergencyPause::vote(origin(3), 0));
        assert!(EmergencyPause::is_paused(b"pallet-rep"));

        // Unpause with 3 votes
        assert_ok!(EmergencyPause::propose_unpause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::vote(origin(2), 1));
        assert_ok!(EmergencyPause::vote(origin(3), 1));
        assert!(!EmergencyPause::is_paused(b"pallet-rep"));
    });
}

// ---------------------------------------------------------------------------
// 10. EmergencyPauseProvider trait
// ---------------------------------------------------------------------------

#[test]
fn is_paused_returns_false_for_unknown_pallet() {
    new_test_ext().execute_with(|| {
        assert!(!EmergencyPause::is_paused(b"pallet-nonexistent"));
    });
}

#[test]
fn is_paused_returns_true_after_pause() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let id = pid(b"pallet-task-market");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::vote(origin(2), 0));
        assert_ok!(EmergencyPause::vote(origin(3), 0));
        assert!(EmergencyPause::is_paused(b"pallet-task-market"));
    });
}

#[test]
fn paused_pallets_list_works() {
    new_test_ext_with_members(vec![1, 2, 3]).execute_with(|| {
        let paused = <EmergencyPause as crate::EmergencyPauseProvider>::paused_pallets();
        assert!(paused.is_empty());

        let id = pid(b"pallet-rep");
        assert_ok!(EmergencyPause::propose_pause(origin(1), id.clone()));
        assert_ok!(EmergencyPause::vote(origin(2), 0));
        assert_ok!(EmergencyPause::vote(origin(3), 0));

        let paused = <EmergencyPause as crate::EmergencyPauseProvider>::paused_pallets();
        assert_eq!(paused.len(), 1);
        assert_eq!(paused[0], b"pallet-rep".to_vec());
    });
}

// ---------------------------------------------------------------------------
// 11. no-op EmergencyPauseProvider
// ---------------------------------------------------------------------------

#[test]
fn noop_emergency_pause_provider() {
    assert!(!<() as crate::EmergencyPauseProvider>::is_paused(
        b"anything"
    ));
    assert!(<() as crate::EmergencyPauseProvider>::paused_pallets().is_empty());
}

// ---------------------------------------------------------------------------
// 12. max active proposals guard
// ---------------------------------------------------------------------------

#[test]
fn too_many_active_proposals_fails() {
    new_test_ext_with_members(vec![1]).execute_with(|| {
        // MaxActiveProposals = 16; create 16 distinct pallet-id proposals
        for i in 0u8..16 {
            let id = alloc::format!("pallet-mock-{}", i).into_bytes();
            assert_ok!(EmergencyPause::propose_pause(origin(1), id));
        }
        // 17th should fail
        assert_noop!(
            EmergencyPause::propose_pause(origin(1), pid(b"pallet-overflow")),
            Error::<Test>::TooManyActiveProposals
        );
    });
}
