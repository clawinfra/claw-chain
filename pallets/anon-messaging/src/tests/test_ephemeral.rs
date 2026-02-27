use crate::{
    pallet::{EphemeralQueue, Inbox, InboxIndex},
    tests::mock::*,
};
use frame_support::{assert_ok, traits::OnInitialize, BoundedVec};
use sp_core::H256;

fn zero_hash() -> H256 {
    H256::zero()
}

fn zero_nonce() -> BoundedVec<u8, sp_runtime::traits::ConstU32<24>> {
    BoundedVec::try_from(vec![0u8; 24]).unwrap()
}

#[test]
fn test_send_message_permanent_not_in_queue() {
    new_test_ext().execute_with(|| {
        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            0, // permanent
            0,
            None,
            None,
        ));

        // No ephemeral queue entries should exist
        let current = System::block_number();
        assert!(EphemeralQueue::<Test>::get(current + 100).is_empty());
    });
}

#[test]
fn test_send_message_ephemeral_adds_to_queue() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            100, // TTL = 100 blocks → expires at block 101
            0,
            None,
            None,
        ));

        let expire_block: u64 = 101;
        let queue = EphemeralQueue::<Test>::get(expire_block);
        assert!(!queue.is_empty());
        assert_eq!(queue[0], (BOB, 0u64));
    });
}

#[test]
fn test_ephemeral_auto_delete_on_initialize() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            10, // TTL = 10 → expires at block 11
            0,
            None,
            None,
        ));

        // Message should exist at block 10
        assert!(Inbox::<Test>::get(BOB, 0u64).is_some());

        // Trigger on_initialize at block 11
        System::set_block_number(11);
        AnonMessaging::on_initialize(11);

        // Message should be gone
        assert!(Inbox::<Test>::get(BOB, 0u64).is_none());
        assert!(!InboxIndex::<Test>::get(BOB).contains(&0u64));
    });
}

#[test]
fn test_ephemeral_message_not_deleted_before_expiry() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(ALICE),
            BOB,
            zero_hash(),
            zero_nonce(),
            20, // expires at block 21
            0,
            None,
            None,
        ));

        // on_initialize at block 10 — should NOT delete
        System::set_block_number(10);
        AnonMessaging::on_initialize(10);

        assert!(Inbox::<Test>::get(BOB, 0u64).is_some());
    });
}
