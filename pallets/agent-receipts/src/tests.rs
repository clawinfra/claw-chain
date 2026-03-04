//! Unit tests for the Agent Receipts pallet.

use crate as pallet_agent_receipts;
use crate::pallet::{AgentNonce, ReceiptCount, Receipts};
use frame_support::{
    assert_noop, assert_ok, derive_impl,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime for testing.
frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        AgentReceiptsPallet: pallet_agent_receipts,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
}

impl pallet_agent_receipts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAgentIdLen = ConstU32<64>;
    type MaxActionTypeLen = ConstU32<64>;
    type MaxMetadataLen = ConstU32<512>;
}

// Build test externalities from genesis storage.
fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account(id: u64) -> <Test as frame_system::Config>::RuntimeOrigin {
    frame_system::RawOrigin::Signed(id).into()
}

fn bounded_agent_id(s: &[u8]) -> crate::pallet::AgentIdOf<Test> {
    s.to_vec().try_into().expect("agent id too long for test")
}

// ========== Tests ==========

#[test]
fn submit_receipt_works() {
    new_test_ext().execute_with(|| {
        let agent_id = b"evoclaw-agent-001".to_vec();
        let action_type = b"tool_call".to_vec();
        let input_hash = H256::repeat_byte(0xAA);
        let output_hash = H256::repeat_byte(0xBB);
        let metadata = b"{\"tool\": \"web_search\"}".to_vec();
        let timestamp = 1708500000000u64;

        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(1),
            agent_id.clone(),
            action_type.clone(),
            input_hash,
            output_hash,
            metadata.clone(),
            timestamp,
        ));

        // Check receipt was stored at nonce 0
        let bid = bounded_agent_id(b"evoclaw-agent-001");
        let receipt = Receipts::<Test>::get(&bid, 0u64).expect("Receipt should exist");
        assert_eq!(receipt.agent_id.to_vec(), agent_id);
        assert_eq!(receipt.action_type.to_vec(), action_type);
        assert_eq!(receipt.input_hash, input_hash);
        assert_eq!(receipt.output_hash, output_hash);
        assert_eq!(receipt.metadata.to_vec(), metadata);
        assert_eq!(receipt.block_number, 1);
        assert_eq!(receipt.timestamp, timestamp);

        // Global counter
        assert_eq!(ReceiptCount::<Test>::get(), 1);
    });
}

#[test]
fn submit_receipt_increments_nonce() {
    new_test_ext().execute_with(|| {
        let agent_id = b"agent-nonce-test".to_vec();
        let bid = bounded_agent_id(b"agent-nonce-test");

        // Nonce starts at 0
        assert_eq!(AgentNonce::<Test>::get(&bid), 0);

        // Submit first receipt
        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(1),
            agent_id.clone(),
            b"action_a".to_vec(),
            H256::repeat_byte(0x01),
            H256::repeat_byte(0x02),
            b"".to_vec(),
            1000,
        ));
        assert_eq!(AgentNonce::<Test>::get(&bid), 1);

        // Submit second receipt
        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(1),
            agent_id.clone(),
            b"action_b".to_vec(),
            H256::repeat_byte(0x03),
            H256::repeat_byte(0x04),
            b"".to_vec(),
            2000,
        ));
        assert_eq!(AgentNonce::<Test>::get(&bid), 2);

        // Submit third receipt
        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(2),
            agent_id.clone(),
            b"action_c".to_vec(),
            H256::repeat_byte(0x05),
            H256::repeat_byte(0x06),
            b"".to_vec(),
            3000,
        ));
        assert_eq!(AgentNonce::<Test>::get(&bid), 3);
    });
}

#[test]
fn submit_multiple_receipts_same_agent() {
    new_test_ext().execute_with(|| {
        let agent_id = b"multi-receipt-agent".to_vec();
        let bid = bounded_agent_id(b"multi-receipt-agent");

        for i in 0u8..5 {
            assert_ok!(AgentReceiptsPallet::submit_receipt(
                account(1),
                agent_id.clone(),
                format!("action_{}", i).into_bytes(),
                H256::repeat_byte(i),
                H256::repeat_byte(i + 100),
                b"{}".to_vec(),
                (i as u64) * 1000,
            ));
        }

        // Verify all 5 receipts exist
        for i in 0u64..5 {
            assert!(Receipts::<Test>::get(&bid, i).is_some());
        }

        // Nonce should be 5
        assert_eq!(AgentNonce::<Test>::get(&bid), 5);

        // Global counter should be 5
        assert_eq!(ReceiptCount::<Test>::get(), 5);
    });
}

#[test]
fn receipt_stored_correctly() {
    new_test_ext().execute_with(|| {
        let agent_id = b"verification-agent".to_vec();
        let action_type = b"trade".to_vec();
        let input_hash = H256::from_slice(&[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
            0x1D, 0x1E, 0x1F, 0x20,
        ]);
        let output_hash = H256::from_slice(&[
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99,
        ]);
        let metadata = b"{\"pair\":\"CLAW/USDT\",\"side\":\"buy\",\"amount\":\"1000\"}".to_vec();
        let timestamp = 1708512345678u64;

        // Set block number to something specific
        System::set_block_number(42);

        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(7),
            agent_id.clone(),
            action_type.clone(),
            input_hash,
            output_hash,
            metadata.clone(),
            timestamp,
        ));

        let bid = bounded_agent_id(b"verification-agent");
        let receipt = Receipts::<Test>::get(&bid, 0u64).expect("Receipt should exist");

        // Verify every single field
        assert_eq!(receipt.agent_id.to_vec(), agent_id, "agent_id mismatch");
        assert_eq!(
            receipt.action_type.to_vec(),
            action_type,
            "action_type mismatch"
        );
        assert_eq!(receipt.input_hash, input_hash, "input_hash mismatch");
        assert_eq!(receipt.output_hash, output_hash, "output_hash mismatch");
        assert_eq!(receipt.metadata.to_vec(), metadata, "metadata mismatch");
        assert_eq!(receipt.block_number, 42, "block_number mismatch");
        assert_eq!(receipt.timestamp, timestamp, "timestamp mismatch");
    });
}

#[test]
fn clear_old_receipts_works() {
    new_test_ext().execute_with(|| {
        let agent_id = b"pruning-agent".to_vec();
        let bid = bounded_agent_id(b"pruning-agent");

        // Submit 5 receipts (nonces 0..4)
        for i in 0u8..5 {
            assert_ok!(AgentReceiptsPallet::submit_receipt(
                account(1),
                agent_id.clone(),
                b"action".to_vec(),
                H256::repeat_byte(i),
                H256::repeat_byte(i),
                b"".to_vec(),
                (i as u64) * 1000,
            ));
        }

        // All 5 exist
        for i in 0u64..5 {
            assert!(Receipts::<Test>::get(&bid, i).is_some());
        }

        // Clear receipts with nonce < 3 (i.e., nonces 0, 1, 2)
        assert_ok!(AgentReceiptsPallet::clear_old_receipts(
            account(2), // any signed caller
            agent_id.clone(),
            3,
        ));

        // Nonces 0, 1, 2 should be gone
        assert!(Receipts::<Test>::get(&bid, 0u64).is_none());
        assert!(Receipts::<Test>::get(&bid, 1u64).is_none());
        assert!(Receipts::<Test>::get(&bid, 2u64).is_none());

        // Nonces 3, 4 should still exist
        assert!(Receipts::<Test>::get(&bid, 3u64).is_some());
        assert!(Receipts::<Test>::get(&bid, 4u64).is_some());

        // Nonce counter should NOT have changed (still 5)
        assert_eq!(AgentNonce::<Test>::get(&bid), 5);
    });
}

#[test]
fn submit_receipt_fails_with_too_long_agent_id() {
    new_test_ext().execute_with(|| {
        let agent_id = vec![0u8; 65]; // Exceeds MaxAgentIdLen of 64

        assert_noop!(
            AgentReceiptsPallet::submit_receipt(
                account(1),
                agent_id,
                b"action".to_vec(),
                H256::zero(),
                H256::zero(),
                b"".to_vec(),
                0,
            ),
            crate::Error::<Test>::AgentIdTooLong
        );
    });
}

#[test]
fn submit_receipt_fails_with_too_long_action_type() {
    new_test_ext().execute_with(|| {
        let action_type = vec![0u8; 65]; // Exceeds MaxActionTypeLen of 64

        assert_noop!(
            AgentReceiptsPallet::submit_receipt(
                account(1),
                b"agent".to_vec(),
                action_type,
                H256::zero(),
                H256::zero(),
                b"".to_vec(),
                0,
            ),
            crate::Error::<Test>::ActionTypeTooLong
        );
    });
}

#[test]
fn submit_receipt_fails_with_too_long_metadata() {
    new_test_ext().execute_with(|| {
        let metadata = vec![0u8; 513]; // Exceeds MaxMetadataLen of 512

        assert_noop!(
            AgentReceiptsPallet::submit_receipt(
                account(1),
                b"agent".to_vec(),
                b"action".to_vec(),
                H256::zero(),
                H256::zero(),
                metadata,
                0,
            ),
            crate::Error::<Test>::MetadataTooLong
        );
    });
}

#[test]
fn different_agents_have_independent_nonces() {
    new_test_ext().execute_with(|| {
        let agent_a = b"agent-alpha".to_vec();
        let agent_b = b"agent-beta".to_vec();
        let bid_a = bounded_agent_id(b"agent-alpha");
        let bid_b = bounded_agent_id(b"agent-beta");

        // Submit 3 for agent A
        for _ in 0..3 {
            assert_ok!(AgentReceiptsPallet::submit_receipt(
                account(1),
                agent_a.clone(),
                b"act".to_vec(),
                H256::zero(),
                H256::zero(),
                b"".to_vec(),
                0,
            ));
        }

        // Submit 1 for agent B
        assert_ok!(AgentReceiptsPallet::submit_receipt(
            account(1),
            agent_b.clone(),
            b"act".to_vec(),
            H256::zero(),
            H256::zero(),
            b"".to_vec(),
            0,
        ));

        assert_eq!(AgentNonce::<Test>::get(&bid_a), 3);
        assert_eq!(AgentNonce::<Test>::get(&bid_b), 1);

        // Global counter = 4
        assert_eq!(ReceiptCount::<Test>::get(), 4);
    });
}
