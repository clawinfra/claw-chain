//! Types for pallet-anon-messaging

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Key type for public keys (X25519 for DH, Ed25519 for signatures)
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum KeyType {
    /// X25519 Diffie-Hellman key (for key exchange)
    X25519,
    /// Ed25519 signature key (future: message authentication)
    Ed25519,
}

impl Default for KeyType {
    fn default() -> Self {
        KeyType::X25519
    }
}

/// Public key registration record
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct PublicKeyRecord<T: Config> {
    /// The public key bytes (32 bytes for X25519)
    pub key: BoundedVec<u8, T::MaxKeyBytes>,
    /// Block number when this key was registered
    pub registered_at: BlockNumberFor<T>,
    /// Type of key (X25519 or Ed25519)
    pub key_type: KeyType,
}

/// Message envelope stored on-chain
///
/// This contains only metadata and the content hash.
/// The actual encrypted payload is stored off-chain (IPFS, agent-local, or direct transfer).
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct MessageEnvelope<T: Config> {
    /// Sender account ID
    pub sender: T::AccountId,
    /// Receiver account ID
    pub receiver: T::AccountId,
    /// Unique message ID (sender-local counter or hash)
    pub msg_id: BoundedVec<u8, T::MaxMessageIdLength>,
    /// Blake2b-256 hash of the encrypted payload
    pub content_hash: BoundedVec<u8, T::MaxHashLength>,
    /// Nonce used for encryption (24 bytes for XChaCha20)
    pub nonce: BoundedVec<u8, T::MaxNonceLength>,
    /// Block number when message was sent
    pub sent_at: BlockNumberFor<T>,
    /// Time-to-live in blocks (0 = no expiration)
    pub ttl: BlockNumberFor<T>,
    /// Message flags (bitfield for future extensions)
    pub flags: MessageFlags,
    /// Optional inline payload (only for small messages ≤ MaxInlinePayloadBytes)
    pub inline_payload: Option<BoundedVec<u8, T::MaxInlinePayloadBytes>>,
}

/// Message flags bitfield
#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq, Default)]
pub struct MessageFlags {
    /// Payload is stored inline on-chain (not off-chain)
    pub inline_payload: bool,
    /// Message has been read by receiver
    pub read: bool,
    /// Message is flagged for ephemeral auto-deletion
    pub ephemeral: bool,
    /// Reserved for future use
    pub reserved: u8,
}

impl MessageFlags {
    /// Create a new MessageFlags with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set inline_payload flag
    #[inline(always)]
    pub fn with_inline_payload(mut self) -> Self {
        self.inline_payload = true;
        self
    }

    /// Set ephemeral flag
    #[inline(always)]
    pub fn with_ephemeral(mut self) -> Self {
        self.ephemeral = true;
        self
    }

    /// Mark as read
    #[inline(always)]
    pub fn mark_read(&mut self) {
        self.read = true;
    }
}

impl Default for MessageFlags {
    fn default() -> Self {
        Self {
            inline_payload: false,
            read: false,
            ephemeral: false,
            reserved: 0,
        }
    }
}
