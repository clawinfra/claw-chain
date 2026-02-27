use crate::{
    pallet::{Error, Event, PublicKeys},
    tests::mock::*,
    KeyType,
};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::traits::ConstU32;

#[test]
fn test_register_public_key_success() {
    new_test_ext().execute_with(|| {
        let key: BoundedVec<u8, _> = BoundedVec::try_from(ALICE_KEY.to_vec()).unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(ALICE),
            key.clone(),
            KeyType::X25519,
        ));

        let record = PublicKeys::<Test>::get(ALICE).expect("key should be stored");
        assert_eq!(record.key.as_slice(), ALICE_KEY.as_ref());
        assert_eq!(record.registered_at, 1);
    });
}

#[test]
fn test_register_public_key_overwrites_previous() {
    new_test_ext().execute_with(|| {
        let key1: BoundedVec<u8, _> = BoundedVec::try_from(ALICE_KEY.to_vec()).unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(ALICE),
            key1,
            KeyType::X25519,
        ));

        System::set_block_number(5);
        let key2: BoundedVec<u8, _> = BoundedVec::try_from([9u8; 32].to_vec()).unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(ALICE),
            key2.clone(),
            KeyType::X25519,
        ));

        let record = PublicKeys::<Test>::get(ALICE).unwrap();
        assert_eq!(record.key.as_slice(), [9u8; 32].as_ref());
        assert_eq!(record.registered_at, 5);
    });
}

#[test]
fn test_register_public_key_invalid_length() {
    new_test_ext().execute_with(|| {
        // 31 bytes — too short
        let key: BoundedVec<u8, _> = BoundedVec::try_from(vec![1u8; 31]).unwrap();
        assert_noop!(
            AnonMessaging::register_public_key(RuntimeOrigin::signed(ALICE), key, KeyType::X25519,),
            Error::<Test>::InvalidKeyLength
        );
    });
}

#[test]
fn test_register_public_key_emits_event() {
    new_test_ext().execute_with(|| {
        let key: BoundedVec<u8, _> = BoundedVec::try_from(ALICE_KEY.to_vec()).unwrap();
        assert_ok!(AnonMessaging::register_public_key(
            RuntimeOrigin::signed(ALICE),
            key,
            KeyType::X25519,
        ));

        System::assert_last_event(
            Event::PublicKeyRegistered {
                account: ALICE,
                key_type: KeyType::X25519,
            }
            .into(),
        );
    });
}
