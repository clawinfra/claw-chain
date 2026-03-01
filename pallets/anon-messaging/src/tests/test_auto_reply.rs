//! Tests for M1 — auto-reply cooldown enforcement.

use crate::{
    pallet::{AutoReplyCooldown, AutoResponses, Event},
    tests::mock::*,
    AutoResponseConfig, KeyType,
};
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::traits::ConstU32;

fn zero_hash() -> H256 {
    H256::zero()
}

fn zero_nonce() -> BoundedVec<u8, ConstU32<24>> {
    BoundedVec::try_from(vec![0u8; 24]).unwrap()
}

/// Helper: send a message from sender to receiver with no escrow.
fn send_msg(sender: u64, receiver: u64) {
    assert_ok!(AnonMessaging::send_message(
        RawOrigin::Signed(sender).into(),
        receiver,
        zero_hash(),
        zero_nonce(),
        0, // permanent
        0, // no pay-for-reply
        None,
        None,
    ));
}

/// Enable auto-response for `account` with the given cooldown (blocks).
fn enable_auto_response(account: u64, cooldown_blocks: u32) {
    let cfg = AutoResponseConfig::<Test> {
        enabled: true,
        response_hash: H256::repeat_byte(0xab),
        min_pay_for_reply: 0,
        cooldown_blocks,
        expires_at: None,
    };
    assert_ok!(AnonMessaging::set_auto_response(
        RawOrigin::Signed(account).into(),
        cfg,
    ));
}

#[test]
fn auto_reply_fires_when_no_cooldown() {
    new_test_ext().execute_with(|| {
        // BOB sets up auto-response with cooldown = 0 (no cooldown)
        enable_auto_response(BOB, 0);

        // First message from ALICE → BOB should trigger auto-response
        send_msg(ALICE, BOB);

        let events = System::events();
        let triggered = events.iter().any(|r| {
            matches!(
                r.event,
                RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
            )
        });
        assert!(triggered, "auto-response should fire with cooldown=0");
    });
}

#[test]
fn auto_reply_rejected_during_cooldown() {
    new_test_ext().execute_with(|| {
        // BOB sets a 10-block cooldown for auto-replies
        enable_auto_response(BOB, 10);

        // Block 1: first message from ALICE fires the auto-reply
        System::set_block_number(1);
        send_msg(ALICE, BOB);
        {
            let events = System::events();
            let triggered = events.iter().any(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
                )
            });
            assert!(triggered, "first auto-reply should fire at block 1");
        }

        // Verify cooldown was recorded
        let last = AutoReplyCooldown::<Test>::get(BOB, ALICE);
        assert_eq!(last, 1);

        // Block 5: still within cooldown (1 + 10 = 11 > 5), auto-reply should be suppressed
        System::set_block_number(5);
        System::reset_events();
        send_msg(ALICE, BOB);
        {
            let events = System::events();
            let triggered = events.iter().any(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
                )
            });
            assert!(
                !triggered,
                "auto-reply must be suppressed during cooldown (block 5, cooldown until 11)"
            );
        }

        // Block 11: cooldown elapsed (1 + 10 = 11 <= 11), auto-reply should fire again
        System::set_block_number(11);
        System::reset_events();
        send_msg(ALICE, BOB);
        {
            let events = System::events();
            let triggered = events.iter().any(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
                )
            });
            assert!(
                triggered,
                "auto-reply should fire again after cooldown expires (block 11)"
            );
        }
    });
}

#[test]
fn auto_reply_cooldown_is_per_sender() {
    new_test_ext().execute_with(|| {
        // BOB sets a 10-block cooldown
        enable_auto_response(BOB, 10);

        System::set_block_number(1);

        // ALICE sends → auto-reply fires, cooldown set for (BOB, ALICE)
        send_msg(ALICE, BOB);
        System::reset_events();

        // CHARLIE sends at block 3 → different sender, cooldown not hit
        System::set_block_number(3);
        send_msg(CHARLIE, BOB);
        {
            let events = System::events();
            let triggered = events.iter().any(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
                )
            });
            assert!(
                triggered,
                "CHARLIE is a different sender — no cooldown should apply"
            );
        }

        // ALICE sends again at block 5 → still in cooldown, suppressed
        System::set_block_number(5);
        System::reset_events();
        send_msg(ALICE, BOB);
        {
            let events = System::events();
            let triggered = events.iter().any(|r| {
                matches!(
                    r.event,
                    RuntimeEvent::AnonMessaging(Event::AutoResponseTriggered { .. })
                )
            });
            assert!(
                !triggered,
                "ALICE auto-reply should still be in cooldown at block 5"
            );
        }
    });
}
