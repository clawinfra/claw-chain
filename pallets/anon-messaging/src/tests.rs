//! Tests for pallet-anon-messaging

use super::*;
use crate as pallet_anon_messaging;
use frame_support::{assert_err, assert_ok, parameter_types};
use frame_system::mocking::MockBlock;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type BlockNumber = u64;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        AnonMessaging: pallet_anon_messaging::{Pallet, Call, Storage, Event<T>},
    }
);

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MaxKeyBytes: u32 = 32;
    pub const MaxMessageIdLength: u32 = 64;
    pub const MaxHashLength: u32 = 32;
    pub const MaxNonceLength: u32 = 24;
    pub const MaxInlinePayloadBytes: u32 = 512;
    pub const MaxMessagesPerAccount: u32 = 1000;
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxKeyBytes = MaxKeyBytes;
    type MaxMessageIdLength = MaxMessageIdLength;
    type MaxHashLength = MaxHashLength;
    type MaxNonceLength = MaxNonceLength;
    type MaxInlinePayloadBytes = MaxInlinePayloadBytes;
    type MaxMessagesPerAccount = MaxMessagesPerAccount;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    sp_io::TestExternalities::new(t)
}

#[test]
fn test_register_public_key() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let mut key_bytes = vec![0u8; 32];
        key_bytes[0] = 1; // X25519 key

        let key: BoundedVec<u8, MaxKeyBytes> =
            key_bytes.try_into().unwrap();

        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(alice),
            key.clone(),
            KeyType::X25519,
        ));

        // Check key was stored
        let stored = PublicKeys::<Test>::get(alice);
        assert!(stored.is_some());
        let record = stored.unwrap();
        assert_eq!(record.key.to_vec(), key.to_vec());
        assert_eq!(record.key_type, KeyType::X25519);
    });
}

#[test]
fn test_register_public_key_invalid_length() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let key_bytes = vec![0u8; 16]; // Wrong length

        let key: BoundedVec<u8, MaxKeyBytes> =
            key_bytes.try_into().unwrap();

        assert_err!(
            AnonMessaging::register_public_key(
                RuntimeOrigin::signed(alice),
                key,
                KeyType::X25519,
            ),
            Error::<Test>::InvalidKeyLength
        );
    });
}

#[test]
fn test_send_and_read_message() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let bob = 2u64;

        // Register Bob's public key first
        let bob_key_bytes = vec![2u8; 32];
        let bob_key: BoundedVec<u8, MaxKeyBytes> =
            bob_key_bytes.try_into().unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(bob),
            bob_key,
            KeyType::X25519,
        ));

        // Alice sends message to Bob
        let msg_id = b"msg_001".to_vec();
        let msg_id_bounded: BoundedVec<u8, MaxMessageIdLength> =
            msg_id.clone().try_into().unwrap();

        let content_hash = vec![3u8; 32];
        let hash_bounded: BoundedVec<u8, MaxHashLength> =
            content_hash.try_into().unwrap();

        let nonce = vec![4u8; 24];
        let nonce_bounded: BoundedVec<u8, MaxNonceLength> =
            nonce.try_into().unwrap();

        let flags = MessageFlags::new();

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(alice),
            bob,
            msg_id_bounded.clone(),
            hash_bounded,
            nonce_bounded,
            0, // No TTL
            flags,
            None, // No inline payload
        ));

        // Bob reads the message
        assert_ok!(AnonMessaging::read_message(
            RuntimeOrigin::signed(bob),
            msg_id_bounded.clone(),
        ));

        // Check message is marked as read
        let envelope = Messages::<Test>::get(bob, msg_id_bounded);
        assert!(envelope.is_some());
        assert!(envelope.unwrap().flags.read);
    });
}

#[test]
fn test_send_message_requires_receiver_key() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let bob = 2u64;

        // Bob hasn't registered a key yet
        let msg_id = b"msg_002".to_vec();
        let msg_id_bounded: BoundedVec<u8, MaxMessageIdLength> =
            msg_id.clone().try_into().unwrap();

        let content_hash = vec![3u8; 32];
        let hash_bounded: BoundedVec<u8, MaxHashLength> =
            content_hash.try_into().unwrap();

        let nonce = vec![4u8; 24];
        let nonce_bounded: BoundedVec<u8, MaxNonceLength> =
            nonce.try_into().unwrap();

        assert_err!(
            AnonMessaging::send_message(
                RuntimeOrigin::signed(alice),
                bob,
                msg_id_bounded,
                hash_bounded,
                nonce_bounded,
                0,
                MessageFlags::new(),
                None,
            ),
            Error::<Test>::KeyNotFound
        );
    });
}

#[test]
fn test_delete_message_as_receiver() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let bob = 2u64;

        // Register Bob's key
        let bob_key_bytes = vec![2u8; 32];
        let bob_key: BoundedVec<u8, MaxKeyBytes> =
            bob_key_bytes.try_into().unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(bob),
            bob_key,
            KeyType::X25519,
        ));

        // Alice sends message to Bob
        let msg_id = b"msg_003".to_vec();
        let msg_id_bounded: BoundedVec<u8, MaxMessageIdLength> =
            msg_id.clone().try_into().unwrap();

        let content_hash = vec![3u8; 32];
        let hash_bounded: BoundedVec<u8, MaxHashLength> =
            content_hash.try_into().unwrap();

        let nonce = vec![4u8; 24];
        let nonce_bounded: BoundedVec<u8, MaxNonceLength> =
            nonce.try_into().unwrap();

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(alice),
            bob,
            msg_id_bounded.clone(),
            hash_bounded,
            nonce_bounded,
            0,
            MessageFlags::new(),
            None,
        ));

        // Bob deletes the message
        assert_ok!(AnonMessaging::delete_message(
            RuntimeOrigin::signed(bob),
            bob,
            msg_id_bounded.clone(),
        ));

        // Message should be gone
        let envelope = Messages::<Test>::get(bob, msg_id_bounded);
        assert!(envelope.is_none());
    });
}

#[test]
fn test_delete_message_unauthorized() {
    new_test_ext().execute_with(|| {
        let alice = 1u64;
        let bob = 2u64;
        let charlie = 3u64;

        // Register Bob's key
        let bob_key_bytes = vec![2u8; 32];
        let bob_key: BoundedVec<u8, MaxKeyBytes> =
            bob_key_bytes.try_into().unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(bob),
            bob_key,
            KeyType::X25519,
        ));

        // Alice sends message to Bob
        let msg_id = b"msg_004".to_vec();
        let msg_id_bounded: BoundedVec<u8, MaxMessageIdLength> =
            msg_id.clone().try_into().unwrap();

        let content_hash = vec![3u8; 32];
        let hash_bounded: BoundedVec<u8, MaxHashLength> =
            content_hash.try_into().unwrap();

        let nonce = vec![4u8; 24];
        let nonce_bounded: BoundedVec<u8, MaxNonceLength> =
            nonce.try_into().unwrap();

        assert_ok!(AnonMessaging::send_message(
            RuntimeOrigin::signed(alice),
            bob,
            msg_id_bounded.clone(),
            hash_bounded,
            nonce_bounded,
            0,
            MessageFlags::new(),
            None,
        ));

        // Charlie tries to delete the message (should fail)
        assert_err!(
            AnonMessaging::delete_message(
                RuntimeOrigin::signed(charlie),
                bob,
                msg_id_bounded,
            ),
            Error::<Test>::NotAuthorized
        );
    });
}
