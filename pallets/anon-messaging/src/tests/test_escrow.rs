use crate::{
    pallet::{Error, Event, MessageEscrow},
    tests::mock::*,
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
fn test_send_message_pay_for_reply_locks_escrow() {
    new_test_ext().execute_with(|| {
        let escrow_amount: u64 = 1000;

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            escrow_amount,
            None,
            None,
        ));

        // Escrow record should exist
        let record = MessageEscrow::<Test>::get(0u64).expect("escrow stored");
        assert_eq!(record.sender, ALICE);
        assert_eq!(record.receiver, BOB);
        assert_eq!(record.amount, escrow_amount);

        // ALICE's balance should be reduced by reserved amount
        assert_eq!(
            pallet_balances::Pallet::<Test>::reserved_balance(ALICE),
            escrow_amount
        );
    });
}

#[test]
fn test_claim_reply_escrow_success() {
    new_test_ext().execute_with(|| {
        let escrow_amount: u64 = 500;

        // ALICE sends with escrow
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            escrow_amount,
            None,
            None,
        ));
        let original_msg_id = 0u64;

        // BOB sends a reply referencing original
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(BOB),
            ALICE,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            Some(original_msg_id),
        ));

        let alice_free_before = pallet_balances::Pallet::<Test>::free_balance(BOB);

        // BOB claims the escrow
        assert_ok!(AnonMessaging::claim_reply_escrow(
            RuntimeOrigin::signed(BOB),
            original_msg_id,
        ));

        // BOB's balance should have increased
        let alice_free_after = pallet_balances::Pallet::<Test>::free_balance(BOB);
        assert_eq!(alice_free_after, alice_free_before + escrow_amount);

        // Escrow record should be gone
        assert!(MessageEscrow::<Test>::get(original_msg_id).is_none());
    });
}

#[test]
fn test_claim_reply_escrow_no_reply_error() {
    new_test_ext().execute_with(|| {
        // ALICE sends with escrow, nobody replies
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            100,
            None,
            None,
        ));

        assert_noop!(
            AnonMessaging::claim_reply_escrow(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NoReplyFound
        );
    });
}

#[test]
fn test_claim_reply_escrow_already_claimed() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            100,
            None,
            None,
        ));

        // BOB replies
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(BOB),
            ALICE,
            zero_hash(),
            zero_nonce(),
            0,
            0,
            None,
            Some(0u64),
        ));

        // First claim succeeds
        assert_ok!(AnonMessaging::claim_reply_escrow(
            RuntimeOrigin::signed(BOB),
            0
        ));

        // Second claim fails — both escrow record and reply record were removed
        assert_noop!(
            AnonMessaging::claim_reply_escrow(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NoReplyFound
        );
    });
}

#[test]
fn test_delete_message_refunds_escrow() {
    new_test_ext().execute_with(|| {
        let escrow_amount: u64 = 300;

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0,
            escrow_amount,
            None,
            None,
        ));

        let alice_free_before = pallet_balances::Pallet::<Test>::free_balance(ALICE);
        let alice_reserved_before = pallet_balances::Pallet::<Test>::reserved_balance(ALICE);
        assert_eq!(alice_reserved_before, escrow_amount);

        // BOB deletes the message — escrow should be refunded to ALICE
        assert_ok!(AnonMessaging::delete_message(RuntimeOrigin::signed(BOB), 0));

        let alice_free_after = pallet_balances::Pallet::<Test>::free_balance(ALICE);
        let alice_reserved_after = pallet_balances::Pallet::<Test>::reserved_balance(ALICE);

        assert_eq!(alice_reserved_after, 0);
        assert_eq!(alice_free_after, alice_free_before + escrow_amount);
    });
}

#[test]
fn test_escrow_amount_exceeds_max() {
    new_test_ext().execute_with(|| {
        // MaxEscrowAmount = 1_000_000_000
        assert_noop!(
            AnonMessaging::send_message(
                RuntimeOrigin::signed(ALICE),
                BOB,
                zero_hash(),
                zero_nonce(),
                0,
                2_000_000_000, // over MaxEscrowAmount
                None,
                None,
            ),
            Error::<Test>::EscrowTooLarge
        );
    });
}
