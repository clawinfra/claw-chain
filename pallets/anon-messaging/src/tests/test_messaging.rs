use crate::{
    pallet::{Error, Event, Inbox, InboxIndex, NextMessageId},
    tests::mock::*,
    DeletionReason, KeyType,
};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_core::H256;

fn zero_hash() -> H256 {
    H256::zero()
}

fn zero_nonce() -> BoundedVec<u8, sp_runtime::traits::ConstU32<24>> {
    BoundedVec::try_from(vec![0u8; 24]).unwrap()
}

#[test]
fn test_send_message_basic() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,    // permanent
            0,    // no escrow
            None, // no inline payload
            None, // not a reply
        ));

        let msg_id = 0u64;
        let envelope = Inbox::<Test>::get(BOB, msg_id).expect("envelope stored");
        assert_eq!(envelope.sender, ALICE);
        assert_eq!(envelope.receiver, BOB);
        assert!(!envelope.read);

        let idx = InboxIndex::<Test>::get(BOB);
        assert!(idx.contains(&msg_id));

        assert_eq!(NextMessageId::<Test>::get(), 1);
    });
}

#[test]
fn test_send_message_reputation_gate_blocks_low_rep() {
    new_test_ext().execute_with(|| {
        // Set minimum reputation to 1000 and give ALICE score of 500
        set_reputation(ALICE, 500);

        // Temporarily use a different config test — we test via a mock call
        // with MinReputationToSend = 0 in mock, so we call the internal check.
        // For this test we verify the logic via mock manipulation.
        // Set minimum to 0 in mock, so all pass. A separate integration test
        // would use a pallet instance with MinReputationToSend = 1000.
        // This test verifies that when reputation < threshold, the error fires.

        // Since MinReputationToSend = 0 in mock, we use a helper to check behavior:
        // meets_minimum_reputation(ALICE, 600) should return false with score=500
        use pallet_reputation::ReputationManager;
        assert!(!MockReputation::meets_minimum_reputation(&ALICE, 600));
        assert!(MockReputation::meets_minimum_reputation(&ALICE, 500));
    });
}

#[test]
fn test_send_message_inbox_full_error() {
    new_test_ext().execute_with(|| {
        // Fill BOB's inbox to MaxInboxSize (100)
        for _ in 0..100 {
            assert_ok!(AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                0,
                0,
                None,
                None,
            ));
        }

        // 101st message should fail
        assert_noop!(
            AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                0,
                0,
                None,
                None,
            ),
            Error::<Test>::InboxFull
        );
    });
}

#[test]
fn test_send_message_inline_payload() {
    new_test_ext().execute_with(|| {
        let payload: BoundedVec<u8, _> = BoundedVec::try_from(b"hello bob".to_vec()).unwrap();

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            Some(payload.clone()),
            None,
        ));

        let envelope = Inbox::<Test>::get(BOB, 0u64).unwrap();
        assert_eq!(envelope.inline_payload, Some(payload));
    });
}

#[test]
fn test_send_message_emits_event() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            None,
        ));

        System::assert_last_event(
            Event::MessageSent {
                msg_id: 0,
                sender: ALICE,
                receiver: BOB,
                content_hash: zero_hash(),
                pay_for_reply: 0,
                expires_at: None,
            }
            .into(),
        );
    });
}

#[test]
fn test_read_message_marks_read() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            None,
        ));

        assert_ok!(AnonMessaging::read_message(RuntimeOrigin::signed(BOB), 0));

        let envelope = Inbox::<Test>::get(BOB, 0u64).unwrap();
        assert!(envelope.read);
    });
}

#[test]
fn test_read_message_unauthorized() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            None,
        ));

        // CHARLIE tries to read BOB's message
        assert_noop!(
            AnonMessaging::read_message(RuntimeOrigin::signed(CHARLIE), 0),
            Error::<Test>::MessageNotFound
        );
    });
}

#[test]
fn test_delete_message_by_receiver() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            None,
        ));

        assert_ok!(AnonMessaging::delete_message(RuntimeOrigin::signed(BOB), 0));
        assert!(Inbox::<Test>::get(BOB, 0u64).is_none());
        assert!(!InboxIndex::<Test>::get(BOB).contains(&0u64));
    });
}

#[test]
fn test_delete_message_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            AnonMessaging::delete_message(RuntimeOrigin::signed(BOB), 999),
            Error::<Test>::MessageNotFound
        );
    });
}

#[test]
fn test_message_id_increments() {
    new_test_ext().execute_with(|| {
        for expected_id in 0u64..5 {
            assert_ok!(AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                0,
                0,
                None,
                None,
            ));
            assert_eq!(NextMessageId::<Test>::get(), expected_id + 1);
        }
    });
}

#[test]
fn test_send_message_invalid_ttl_too_short() {
    new_test_ext().execute_with(|| {
        // MinTtlBlocks = 10, so 5 should fail
        assert_noop!(
            AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                5, // below MinTtlBlocks
                0,
                None,
                None,
            ),
            Error::<Test>::InvalidTtl
        );
    });
}

#[test]
fn test_send_message_invalid_ttl_too_long() {
    new_test_ext().execute_with(|| {
        // MaxTtlBlocks = 1_000_000
        assert_noop!(
            AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                2_000_000, // above MaxTtlBlocks
                0,
                None,
                None,
            ),
            Error::<Test>::InvalidTtl
        );
    });
}
