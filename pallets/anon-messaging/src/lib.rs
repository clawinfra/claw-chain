//! # Anonymous Messaging Pallet
//!
//! End-to-end encrypted messaging system for ClawChain agents.
//!
//! ## Overview
//!
//! - Agents register X25519 public keys for key exchange
//! - Messages are encrypted off-chain using XChaCha20-Poly1305
//! - Only metadata and content hashes are stored on-chain
//! - Optional inline storage for small messages (≤ 512 bytes)
//!
//! ## Dispatchable Functions
//!
//! - `register_public_key` - Register an X25519 public key for ECDH
//! - `send_message` - Send an encrypted message (with optional inline payload)
//! - `read_message` - Mark a message as read
//! - `delete_message` - Delete a message (sender or receiver only)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod types;

use alloc::vec::Vec;
use types::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    // =========================================================
    // Pallet struct
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Config trait
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        /// Max byte length of a public key (32 bytes for X25519)
        #[pallet::constant]
        type MaxKeyBytes: Get<u32>;

        /// Max byte length of a message ID
        #[pallet::constant]
        type MaxMessageIdLength: Get<u32>;

        /// Max byte length of a content hash (32 bytes for Blake2b-256)
        #[pallet::constant]
        type MaxHashLength: Get<u32>;

        /// Max byte length of a nonce (24 bytes for XChaCha20)
        #[pallet::constant]
        type MaxNonceLength: Get<u32>;

        /// Max inline payload size (default 512 bytes)
        #[pallet::constant]
        type MaxInlinePayloadBytes: Get<u32>;

        /// Maximum number of messages per account (prevents spam)
        #[pallet::constant]
        type MaxMessagesPerAccount: Get<u32>;
    }

    // =========================================================
    // Storage
    // =========================================================

    /// Public keys registered by agents
    /// Maps AccountId -> PublicKeyRecord
    #[pallet::storage]
    pub type PublicKeys<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        PublicKeyRecord<T>,
    >;

    /// Message envelopes
    /// Maps (receiver_account, msg_id) -> MessageEnvelope
    #[pallet::storage]
    pub type Messages<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,  // receiver
        Blake2_128Concat,
        BoundedVec<u8, T::MaxMessageIdLength>,  // msg_id
        MessageEnvelope<T>,
    >;

    /// Message counter per account (for generating unique msg_ids)
    /// Maps AccountId -> u32
    #[pallet::storage]
    pub type MessageCounters<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u32,
        ValueQuery,
    >;

    /// Ephemeral message queue for auto-deletion (stub for Phase 1)
    /// Maps BlockNumber -> Vec<(receiver, msg_id)>
    #[pallet::storage]
    pub type EphemeralQueue<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Vec<(T::AccountId, BoundedVec<u8, T::MaxMessageIdLength>)>,
    >;

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A public key was registered
        /// [account, key_type]
        KeyRegistered { account: T::AccountId, key_type: KeyType },
        /// A message was sent
        /// [sender, receiver, msg_id, has_inline_payload]
        MessageSent {
            sender: T::AccountId,
            receiver: T::AccountId,
            msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
            has_inline_payload: bool,
        },
        /// A message was marked as read
        /// [receiver, msg_id]
        MessageRead {
            receiver: T::AccountId,
            msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
        },
        /// A message was deleted
        /// [who, msg_id]
        MessageDeleted {
            who: T::AccountId,
            msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Public key not found for this account
        KeyNotFound,
        /// Invalid cryptographic signature
        InvalidSignature,
        /// Message has expired (TTL elapsed)
        MessageExpired,
        /// Message not found
        MessageNotFound,
        /// Only sender or receiver can perform this action
        NotAuthorized,
        /// Message ID already exists
        MessageIdExists,
        /// Maximum messages per account reached
        MaxMessagesReached,
        /// Inline payload exceeds maximum size
        InlinePayloadTooLarge,
        /// Invalid nonce length (must be 24 bytes for XChaCha20)
        InvalidNonceLength,
        /// Invalid hash length (must be 32 bytes for Blake2b-256)
        InvalidHashLength,
        /// Invalid key length (must be 32 bytes for X25519)
        InvalidKeyLength,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    /// Register a public key for key exchange
    ///
    /// # Args
    /// * `key` - Public key bytes (32 bytes for X25519)
    /// * `key_type` - Type of key (X25519 or Ed25519)
    ///
    /// # Weight
    /// TODO: benchmark and calculate proper weight
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::register_public_key())]
    pub fn register_public_key(
        origin: OriginFor<T>,
        key: BoundedVec<u8, T::MaxKeyBytes>,
        key_type: KeyType,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;

        // Validate key length (32 bytes for X25519)
        if key_type == KeyType::X25519 && key.len() != 32 {
            return Err(Error::<T>::InvalidKeyLength.into());
        }

        let current_block = frame_system::Pallet::<T>::block_number();

        // Create public key record
        let key_record = PublicKeyRecord {
            key,
            registered_at: current_block,
            key_type,
        };

        // Store the public key (overwrites any existing key)
        PublicKeys::<T>::insert(&who, key_record);

        // Emit event
        Self::deposit_event(Event::KeyRegistered {
            account: who,
            key_type,
        });

        Ok(())
    }

    /// Send an encrypted message
    ///
    /// # Args
    /// * `receiver` - Recipient account ID
    /// * `msg_id` - Unique message ID
    /// * `content_hash` - Blake2b-256 hash of encrypted payload
    /// * `nonce` - Encryption nonce (24 bytes for XChaCha20)
    /// * `ttl` - Time-to-live in blocks (0 = no expiration)
    /// * `flags` - Message flags (inline_payload, ephemeral, etc.)
    /// * `inline_payload` - Optional inline encrypted payload (small messages only)
    ///
    /// # Weight
    /// TODO: benchmark and calculate proper weight
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::send_message())]
    pub fn send_message(
        origin: OriginFor<T>,
        receiver: T::AccountId,
        msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
        content_hash: BoundedVec<u8, T::MaxHashLength>,
        nonce: BoundedVec<u8, T::MaxNonceLength>,
        ttl: BlockNumberFor<T>,
        flags: MessageFlags,
        inline_payload: Option<BoundedVec<u8, T::MaxInlinePayloadBytes>>,
    ) -> DispatchResult {
        let sender = ensure_signed(origin)?;

        // Ensure receiver has a public key registered
        PublicKeys::<T>::get(&receiver)
            .ok_or(Error::<T>::KeyNotFound)?;

        // Validate hash length (32 bytes for Blake2b-256)
        if content_hash.len() != 32 {
            return Err(Error::<T>::InvalidHashLength.into());
        }

        // Validate nonce length (24 bytes for XChaCha20)
        if nonce.len() != 24 {
            return Err(Error::<T>::InvalidNonceLength.into());
        }

        // Check if message ID already exists
        if Messages::<T>::contains_key(&receiver, &msg_id) {
            return Err(Error::<T>::MessageIdExists.into());
        }

        // Validate inline payload
        if let Some(ref payload) = inline_payload {
            if !flags.inline_payload {
                return Err(Error::<T>::InlinePayloadTooLarge.into()); // Mis-flagged
            }
        } else if flags.inline_payload {
            return Err(Error::<T>::InlinePayloadTooLarge.into()); // Missing payload
        }

        let current_block = frame_system::Pallet::<T>::block_number();

        // Create message envelope
        let envelope = MessageEnvelope {
            sender: sender.clone(),
            receiver: receiver.clone(),
            msg_id: msg_id.clone(),
            content_hash,
            nonce,
            sent_at: current_block,
            ttl,
            flags,
            inline_payload,
        };

        // Store the message
        Messages::<T>::insert(&receiver, &msg_id, envelope);

        // Increment sender's message counter
        MessageCounters::<T>::mutate(&sender, |counter| {
            *counter += 1;
        });

        // Emit event
        Self::deposit_event(Event::MessageSent {
            sender,
            receiver,
            msg_id,
            has_inline_payload: inline_payload.is_some(),
        });

        Ok(())
    }

    /// Mark a message as read
    ///
    /// # Args
    /// * `msg_id` - Message ID to mark as read
    ///
    /// # Weight
    /// TODO: benchmark and calculate proper weight
    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::read_message())]
    pub fn read_message(
        origin: OriginFor<T>,
        msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;

        // Get the message
        let mut envelope = Messages::<T>::get(&who, &msg_id)
            .ok_or(Error::<T>::MessageNotFound)?;

        // Mark as read
        envelope.flags.mark_read();

        // Update the message
        Messages::<T>::insert(&who, &msg_id, envelope);

        // Emit event
        Self::deposit_event(Event::MessageRead {
            receiver: who,
            msg_id,
        });

        Ok(())
    }

    /// Delete a message
    ///
    /// Only the sender or receiver can delete a message.
    ///
    /// # Args
    /// * `msg_id` - Message ID to delete
    /// * `as_sender` - If true, act as sender (lookup by sender account)
    ///
    /// # Weight
    /// TODO: benchmark and calculate proper weight
    #[pallet::call_index(3)]
    #[pallet::weight(T::WeightInfo::delete_message())]
    pub fn delete_message(
        origin: OriginFor<T>,
        receiver: T::AccountId,
        msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;

        // Get the message
        let envelope = Messages::<T>::get(&receiver, &msg_id)
            .ok_or(Error::<T>::MessageNotFound)?;

        // Check authorization (must be sender or receiver)
        if envelope.sender != who && envelope.receiver != who {
            return Err(Error::<T>::NotAuthorized.into());
        }

        // Delete the message
        Messages::<T>::remove(&receiver, &msg_id);

        // Emit event
        Self::deposit_event(Event::MessageDeleted {
            who,
            msg_id,
        });

        Ok(())
    }
}

// =========================================================
// Weight info stub (to be benchmarked)
// =========================================================

pub trait WeightInfo {
    fn register_public_key() -> Weight;
    fn send_message() -> Weight;
    fn read_message() -> Weight;
    fn delete_message() -> Weight;
}

impl WeightInfo for () {
    fn register_public_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn send_message() -> Weight {
        Weight::from_parts(50_000, 0)
    }

    fn read_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn delete_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}
