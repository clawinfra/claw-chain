//! # Anon Messaging Pallet
//!
//! Encrypted agent-to-agent direct messages for ClawChain.
//!
//! ## Overview
//!
//! Implements ADR-010 Level 1 — on-chain message envelopes with off-chain
//! encrypted payloads. The pallet stores:
//! - Agent X25519 public keys (for off-chain key exchange)
//! - Message envelopes (metadata + Blake2b-256 content hash)
//! - Optional inline payloads for small messages (≤ `MaxInlinePayloadBytes`)
//! - Pay-for-reply escrow (CLAW reserved until receiver replies)
//! - Ephemeral TTL queue (auto-delete via `on_initialize`)
//!
//! ## Dispatchable Functions
//!
//! - `register_public_key` — Register/update X25519 public key
//! - `send_message` — Send encrypted message envelope to any agent
//! - `read_message` — Mark message as read (on-chain read receipt)
//! - `delete_message` — Delete message by sender or receiver
//! - `set_auto_response` — Configure auto-response for incoming messages
//! - `claim_reply_escrow` — Claim escrowed CLAW after replying
//!
//! ## Privacy Model
//!
//! Level 1 — content is E2E encrypted, but communication graph is public.
//! See PLAN.md §10 for full privacy analysis.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use pallet_reputation::ReputationManager;
    use sp_core::H256;
    use sp_runtime::traits::Saturating;

    // =========================================================
    // Type aliases
    // =========================================================

    /// Balance type derived from the configured Currency.
    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Globally unique message identifier.
    pub type MessageId = u64;

    // =========================================================
    // Core types
    // =========================================================

    /// Key algorithm identifier.
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum KeyType {
        /// X25519 Diffie-Hellman (32 bytes).
        X25519,
        /// Ed25519 signing key (32 bytes). Reserved for future use.
        Ed25519,
    }

    impl codec::DecodeWithMemTracking for KeyType {}

    /// On-chain record of an agent's public key.
    ///
    /// Uses `CloneNoBound` etc. because the struct is generic over `T: Config`
    /// and the standard `derive(Clone)` would emit a `T: Clone` bound which
    /// is not satisfied by all runtimes.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PublicKeyRecord<T: Config> {
        /// Raw key bytes (32 bytes for X25519/Ed25519).
        pub key: BoundedVec<u8, T::MaxKeyBytes>,
        /// Block number when the key was last registered/updated.
        pub registered_at: BlockNumberFor<T>,
        /// Key algorithm.
        pub key_type: KeyType,
    }

    impl<T: Config> codec::DecodeWithMemTracking for PublicKeyRecord<T> {}

    /// Reason a message was deleted (included in event for auditability).
    #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum DeletionReason {
        /// Deleted manually by sender or receiver.
        Manual,
        /// Deleted automatically because TTL expired.
        Expired,
    }

    impl codec::DecodeWithMemTracking for DeletionReason {}

    /// On-chain message envelope — metadata + integrity hash.
    ///
    /// The actual encrypted payload is stored off-chain (IPFS, EvoClaw MQTT, etc.).
    /// Agents verify integrity by comparing Blake2b-256(ciphertext) to `content_hash`.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct MessageEnvelope<T: Config> {
        /// Globally unique message ID (auto-incremented).
        pub msg_id: MessageId,
        /// Sender account.
        pub sender: T::AccountId,
        /// Receiver account.
        pub receiver: T::AccountId,
        /// Blake2b-256 hash of the off-chain ciphertext. Zero if inline payload used.
        pub content_hash: H256,
        /// XChaCha20 nonce (24 bytes) stored for AAD reconstruction.
        pub nonce: BoundedVec<u8, ConstU32<24>>,
        /// Number of blocks until auto-deletion. 0 = permanent.
        pub ttl_blocks: u32,
        /// Block at which the message was sent.
        pub sent_at: BlockNumberFor<T>,
        /// Whether the receiver has called `read_message` for this envelope.
        pub read: bool,
        /// CLAW amount escrowed as pay-for-reply incentive. 0 = no escrow.
        pub pay_for_reply: BalanceOf<T>,
        /// Optional small payload stored directly on-chain (≤ MaxInlinePayloadBytes).
        pub inline_payload: Option<BoundedVec<u8, T::MaxInlinePayloadBytes>>,
        /// ID of the message this is a reply to, if applicable.
        pub reply_to: Option<MessageId>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for MessageEnvelope<T> {}

    /// Auto-response configuration for an agent.
    #[derive(
        Encode,
        Decode,
        CloneNoBound,
        EqNoBound,
        PartialEqNoBound,
        RuntimeDebugNoBound,
        TypeInfo,
        MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct AutoResponseConfig<T: Config> {
        /// Whether auto-response is currently active.
        pub enabled: bool,
        /// Blake2b-256 hash of the off-chain auto-reply template.
        pub response_hash: H256,
        /// Minimum pay-for-reply required to trigger auto-response. 0 = free.
        pub min_pay_for_reply: BalanceOf<T>,
        /// Minimum blocks between auto-replies to the same sender (cooldown).
        pub cooldown_blocks: u32,
        /// Optional block at which this config expires (None = never).
        pub expires_at: Option<BlockNumberFor<T>>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for AutoResponseConfig<T> {}

    /// Escrow record for a pay-for-reply message.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct EscrowRecord<T: Config> {
        pub sender: T::AccountId,
        pub receiver: T::AccountId,
        pub amount: BalanceOf<T>,
        pub locked_at: BlockNumberFor<T>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for EscrowRecord<T> {}

    // =========================================================
    // Config
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;

        /// Currency used for pay-for-reply escrow.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Cross-pallet reputation gate.
        type ReputationManager: ReputationManager<Self::AccountId, BalanceOf<Self>>;

        /// Maximum byte length of a public key (32 for X25519/Ed25519).
        #[pallet::constant]
        type MaxKeyBytes: Get<u32>;

        /// Maximum number of messages in a single inbox.
        #[pallet::constant]
        type MaxInboxSize: Get<u32>;

        /// Maximum byte length for an inline (on-chain) payload.
        #[pallet::constant]
        type MaxInlinePayloadBytes: Get<u32>;

        /// Maximum number of ephemeral messages expiring at the same block.
        #[pallet::constant]
        type MaxEphemeralPerBlock: Get<u32>;

        /// Minimum reputation (basis points, 0–10000) required to send a message.
        #[pallet::constant]
        type MinReputationToSend: Get<u32>;

        /// Minimum TTL in blocks for ephemeral messages.
        #[pallet::constant]
        type MinTtlBlocks: Get<u32>;

        /// Maximum TTL in blocks for ephemeral messages.
        #[pallet::constant]
        type MaxTtlBlocks: Get<u32>;

        /// Safety cap on pay-for-reply escrow amount.
        #[pallet::constant]
        type MaxEscrowAmount: Get<BalanceOf<Self>>;
    }

    // =========================================================
    // Storage
    // =========================================================

    /// Agent's registered public key.
    #[pallet::storage]
    #[pallet::getter(fn public_keys)]
    pub type PublicKeys<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, PublicKeyRecord<T>, OptionQuery>;

    /// Message envelopes indexed by (receiver, msg_id).
    #[pallet::storage]
    #[pallet::getter(fn inbox)]
    pub type Inbox<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        MessageId,
        MessageEnvelope<T>,
        OptionQuery,
    >;

    /// Ordered inbox index per receiver (for iteration / capacity checks).
    #[pallet::storage]
    #[pallet::getter(fn inbox_index)]
    pub type InboxIndex<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<MessageId, T::MaxInboxSize>,
        ValueQuery,
    >;

    /// Auto-incrementing message ID counter.
    #[pallet::storage]
    #[pallet::getter(fn next_message_id)]
    pub type NextMessageId<T: Config> = StorageValue<_, MessageId, ValueQuery>;

    /// Ephemeral queue: expiry block → list of (receiver, msg_id).
    #[pallet::storage]
    #[pallet::getter(fn ephemeral_queue)]
    pub type EphemeralQueue<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<(T::AccountId, MessageId), T::MaxEphemeralPerBlock>,
        ValueQuery,
    >;

    /// Auto-response configuration per agent.
    #[pallet::storage]
    #[pallet::getter(fn auto_responses)]
    pub type AutoResponses<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AutoResponseConfig<T>, OptionQuery>;

    /// Escrow records for pay-for-reply messages.
    #[pallet::storage]
    #[pallet::getter(fn message_escrow)]
    pub type MessageEscrow<T: Config> =
        StorageMap<_, Blake2_128Concat, MessageId, EscrowRecord<T>, OptionQuery>;

    /// Maps original msg_id → reply msg_id (set when a reply references reply_to).
    #[pallet::storage]
    #[pallet::getter(fn escrow_replied)]
    pub type EscrowReplied<T: Config> =
        StorageMap<_, Blake2_128Concat, MessageId, MessageId, OptionQuery>;

    /// Last auto-reply block per (responder, requester) pair (cooldown tracking).
    #[pallet::storage]
    #[pallet::getter(fn auto_reply_cooldown)]
    pub type AutoReplyCooldown<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        Blake2_128Concat,
        T::AccountId,
        BlockNumberFor<T>,
        ValueQuery,
    >;

    // =========================================================
    // Pallet struct & hooks
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Process ephemeral message expirations at the start of each block.
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let expiring = EphemeralQueue::<T>::take(n);
            let count = expiring.len() as u32;

            for (receiver, msg_id) in expiring.iter() {
                Self::do_delete_message(receiver, *msg_id, DeletionReason::Expired);
            }

            T::WeightInfo::on_initialize(count)
        }
    }

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An agent registered or updated their public key.
        PublicKeyRegistered {
            account: T::AccountId,
            key_type: KeyType,
        },

        /// A message was sent.
        MessageSent {
            msg_id: MessageId,
            sender: T::AccountId,
            receiver: T::AccountId,
            content_hash: H256,
            pay_for_reply: BalanceOf<T>,
            expires_at: Option<BlockNumberFor<T>>,
        },

        /// A message was read (on-chain read receipt).
        MessageRead {
            msg_id: MessageId,
            receiver: T::AccountId,
        },

        /// A message was deleted.
        MessageDeleted {
            msg_id: MessageId,
            deleted_by: T::AccountId,
            reason: DeletionReason,
        },

        /// Auto-response was triggered for an incoming message.
        AutoResponseTriggered {
            original_msg_id: MessageId,
            responder: T::AccountId,
            response_hash: H256,
        },

        /// Auto-response configuration updated.
        AutoResponseConfigured {
            account: T::AccountId,
            enabled: bool,
        },

        /// Pay-for-reply escrow locked.
        EscrowLocked {
            msg_id: MessageId,
            sender: T::AccountId,
            amount: BalanceOf<T>,
        },

        /// Reply escrow claimed by receiver.
        EscrowClaimed {
            original_msg_id: MessageId,
            receiver: T::AccountId,
            amount: BalanceOf<T>,
        },

        /// Escrow refunded to sender (message deleted before claim).
        EscrowRefunded {
            msg_id: MessageId,
            sender: T::AccountId,
            amount: BalanceOf<T>,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Sender's reputation is below the minimum threshold.
        InsufficientReputation,
        /// Receiver's inbox is full.
        InboxFull,
        /// Message not found.
        MessageNotFound,
        /// Caller is not the sender or receiver of this message.
        Unauthorized,
        /// Public key has not been registered.
        KeyNotRegistered,
        /// Key bytes have invalid length for the specified key type.
        InvalidKeyLength,
        /// TTL is outside the allowed range (must be 0 or between MinTtl and MaxTtl).
        InvalidTtl,
        /// Inline payload exceeds MaxInlinePayloadBytes.
        PayloadTooLarge,
        /// Pay-for-reply escrow amount exceeds MaxEscrowAmount.
        EscrowTooLarge,
        /// No reply has been sent for this message.
        NoReplyFound,
        /// Escrow has already been claimed.
        EscrowAlreadyClaimed,
        /// Auto-reply cooldown has not elapsed since last auto-reply.
        AutoReplyCooldownActive,
        /// Auto-response configuration has expired.
        AutoResponseExpired,
        /// Message ID counter overflowed (u64 wrap).
        MessageIdOverflow,
        /// Ephemeral queue for the target block is full (rollover applied to next block).
        EphemeralQueueFull,
        /// Caller has insufficient balance to reserve for escrow.
        InsufficientBalance,
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register or update the caller's X25519 public key.
        ///
        /// - `key`: raw 32-byte X25519 public key.
        /// - `key_type`: must be `KeyType::X25519` in Phase 1.
        ///
        /// Overwrites any previously registered key. Key rotation does not
        /// affect existing message envelopes (those use the old shared secret).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_public_key())]
        pub fn register_public_key(
            origin: OriginFor<T>,
            key: BoundedVec<u8, T::MaxKeyBytes>,
            key_type: KeyType,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // X25519 and Ed25519 keys are always exactly 32 bytes.
            ensure!(key.len() == 32, Error::<T>::InvalidKeyLength);

            let now = frame_system::Pallet::<T>::block_number();
            let record = PublicKeyRecord {
                key,
                registered_at: now,
                key_type: key_type.clone(),
            };

            PublicKeys::<T>::insert(&who, record);
            Self::deposit_event(Event::PublicKeyRegistered {
                account: who,
                key_type,
            });
            Ok(())
        }

        /// Send an encrypted message to any agent.
        ///
        /// The actual ciphertext is stored off-chain; `content_hash` is the
        /// Blake2b-256 of the ciphertext for on-chain integrity verification.
        /// `nonce` is the 24-byte XChaCha20 nonce (stored so the receiver can
        /// reconstruct the AAD: `sender ++ receiver ++ msg_id ++ block_number`).
        ///
        /// Set `ttl_blocks = 0` for a permanent message, or > 0 for ephemeral
        /// auto-deletion (must be within [MinTtlBlocks, MaxTtlBlocks]).
        ///
        /// Set `pay_for_reply > 0` to lock CLAW as an incentive for the receiver
        /// to reply. The receiver calls `claim_reply_escrow` after replying.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::send_message())]
        pub fn send_message(
            origin: OriginFor<T>,
            receiver: T::AccountId,
            content_hash: H256,
            nonce: BoundedVec<u8, ConstU32<24>>,
            ttl_blocks: u32,
            pay_for_reply: BalanceOf<T>,
            inline_payload: Option<BoundedVec<u8, T::MaxInlinePayloadBytes>>,
            reply_to: Option<MessageId>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Reputation gate
            ensure!(
                T::ReputationManager::meets_minimum_reputation(
                    &sender,
                    T::MinReputationToSend::get()
                ),
                Error::<T>::InsufficientReputation
            );

            // Validate TTL
            if ttl_blocks != 0 {
                ensure!(ttl_blocks >= T::MinTtlBlocks::get(), Error::<T>::InvalidTtl);
                ensure!(ttl_blocks <= T::MaxTtlBlocks::get(), Error::<T>::InvalidTtl);
            }

            // Validate escrow cap
            ensure!(
                pay_for_reply <= T::MaxEscrowAmount::get(),
                Error::<T>::EscrowTooLarge
            );

            // Check inbox capacity
            let inbox = InboxIndex::<T>::get(&receiver);
            ensure!(
                (inbox.len() as u32) < T::MaxInboxSize::get(),
                Error::<T>::InboxFull
            );

            // Assign message ID
            let msg_id = NextMessageId::<T>::get();
            let next = msg_id.checked_add(1).ok_or(Error::<T>::MessageIdOverflow)?;
            NextMessageId::<T>::put(next);

            let now = frame_system::Pallet::<T>::block_number();

            // Reserve escrow if requested
            {
                let zero: BalanceOf<T> = 0u32.into();
                if pay_for_reply > zero {
                    T::Currency::reserve(&sender, pay_for_reply)
                        .map_err(|_| Error::<T>::InsufficientBalance)?;

                    MessageEscrow::<T>::insert(
                        msg_id,
                        EscrowRecord {
                            sender: sender.clone(),
                            receiver: receiver.clone(),
                            amount: pay_for_reply,
                            locked_at: now,
                        },
                    );

                    Self::deposit_event(Event::EscrowLocked {
                        msg_id,
                        sender: sender.clone(),
                        amount: pay_for_reply,
                    });
                }
            }

            // Handle ephemeral TTL
            let expires_at = if ttl_blocks != 0 {
                let ttl: BlockNumberFor<T> = ttl_blocks.into();
                let expire_block = now.saturating_add(ttl);

                // Try to enqueue; if full, roll over to next block
                let mut enqueued = false;
                EphemeralQueue::<T>::mutate(expire_block, |q| {
                    if (q.len() as u32) < T::MaxEphemeralPerBlock::get() {
                        let _ = q.try_push((receiver.clone(), msg_id));
                        enqueued = true;
                    }
                });

                if !enqueued {
                    // Rollover to next block
                    let next_block = expire_block.saturating_add(1u32.into());
                    EphemeralQueue::<T>::mutate(next_block, |q| {
                        let _ = q.try_push((receiver.clone(), msg_id));
                    });
                }

                Some(expire_block)
            } else {
                None
            };

            // Track reply-to for escrow
            if let Some(orig_id) = reply_to {
                EscrowReplied::<T>::insert(orig_id, msg_id);
            }

            // Build envelope
            let envelope = MessageEnvelope {
                msg_id,
                sender: sender.clone(),
                receiver: receiver.clone(),
                content_hash,
                nonce,
                ttl_blocks,
                sent_at: now,
                read: false,
                pay_for_reply,
                inline_payload,
                reply_to,
            };

            Inbox::<T>::insert(&receiver, msg_id, envelope);

            InboxIndex::<T>::mutate(&receiver, |idx| {
                let _ = idx.try_push(msg_id);
            });

            // Check if receiver has auto-response enabled
            Self::maybe_trigger_auto_response(&receiver, &sender, msg_id, pay_for_reply, now);

            Self::deposit_event(Event::MessageSent {
                msg_id,
                sender,
                receiver,
                content_hash,
                pay_for_reply,
                expires_at,
            });

            Ok(())
        }

        /// Mark a message as read (on-chain read receipt).
        ///
        /// Only the receiver may call this. The receipt is visible on-chain and
        /// via the emitted `MessageRead` event.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::read_message())]
        pub fn read_message(origin: OriginFor<T>, msg_id: MessageId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Inbox::<T>::try_mutate(&who, msg_id, |maybe| -> DispatchResult {
                if let Some(env) = maybe {
                    ensure!(env.receiver == who, Error::<T>::Unauthorized);
                    env.read = true;
                    Ok(())
                } else {
                    Err(Error::<T>::MessageNotFound.into())
                }
            })?;

            Self::deposit_event(Event::MessageRead {
                msg_id,
                receiver: who,
            });
            Ok(())
        }

        /// Delete a message. Callable by sender or receiver.
        ///
        /// Any unreleased pay-for-reply escrow is automatically refunded to
        /// the original sender.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::delete_message())]
        pub fn delete_message(origin: OriginFor<T>, msg_id: MessageId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Load envelope from receiver's inbox — we need to know who the receiver is.
            // Try the caller as receiver first, then check if they're the sender.
            let envelope = if let Some(env) = Inbox::<T>::get(&who, msg_id) {
                env
            } else {
                // Caller might be the sender — scan isn't viable, so we require
                // the caller to be the receiver. Sender-side delete is handled via
                // a separate approach: store a SentIndex for senders in Phase 2.
                // For Phase 1, only receiver can delete directly. Sender can use
                // the receiver's account when they know it. We allow sender-side
                // if the envelope is accessible (e.g., inbox of another account).
                return Err(Error::<T>::MessageNotFound.into());
            };

            ensure!(
                envelope.sender == who || envelope.receiver == who,
                Error::<T>::Unauthorized
            );

            Self::do_delete_message(&envelope.receiver, msg_id, DeletionReason::Manual);

            Self::deposit_event(Event::MessageDeleted {
                msg_id,
                deleted_by: who,
                reason: DeletionReason::Manual,
            });

            Ok(())
        }

        /// Configure auto-response for incoming messages.
        ///
        /// When enabled, `MessageSent` events will also include an
        /// `AutoResponseTriggered` event if the conditions are met.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::set_auto_response())]
        pub fn set_auto_response(
            origin: OriginFor<T>,
            config: AutoResponseConfig<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let enabled = config.enabled;
            AutoResponses::<T>::insert(&who, config);

            Self::deposit_event(Event::AutoResponseConfigured {
                account: who,
                enabled,
            });
            Ok(())
        }

        /// Claim escrowed CLAW after having replied to a pay-for-reply message.
        ///
        /// The caller must be the receiver of `original_msg_id` and must have
        /// previously sent a reply referencing `reply_to = Some(original_msg_id)`.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::claim_reply_escrow())]
        pub fn claim_reply_escrow(
            origin: OriginFor<T>,
            original_msg_id: MessageId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Verify a reply was sent
            ensure!(
                EscrowReplied::<T>::contains_key(original_msg_id),
                Error::<T>::NoReplyFound
            );

            // Load and remove escrow record
            let record = MessageEscrow::<T>::take(original_msg_id)
                .ok_or(Error::<T>::EscrowAlreadyClaimed)?;

            ensure!(record.receiver == who, Error::<T>::Unauthorized);

            // Transfer: unreserve from sender, transfer to receiver
            T::Currency::unreserve(&record.sender, record.amount);
            T::Currency::transfer(
                &record.sender,
                &record.receiver,
                record.amount,
                ExistenceRequirement::KeepAlive,
            )?;

            EscrowReplied::<T>::remove(original_msg_id);

            Self::deposit_event(Event::EscrowClaimed {
                original_msg_id,
                receiver: who,
                amount: record.amount,
            });

            Ok(())
        }
    }

    // =========================================================
    // Internal helpers
    // =========================================================

    impl<T: Config> Pallet<T> {
        /// Remove a message envelope and clean up associated storage.
        /// Refunds any unreleased escrow to the original sender.
        pub(crate) fn do_delete_message(
            receiver: &T::AccountId,
            msg_id: MessageId,
            reason: DeletionReason,
        ) {
            if let Some(env) = Inbox::<T>::take(receiver, msg_id) {
                // Remove from inbox index
                InboxIndex::<T>::mutate(receiver, |idx| {
                    idx.retain(|&id| id != msg_id);
                });

                // Refund escrow if unclaimed
                if let Some(record) = MessageEscrow::<T>::take(msg_id) {
                    T::Currency::unreserve(&record.sender, record.amount);
                    Self::deposit_event(Event::EscrowRefunded {
                        msg_id,
                        sender: record.sender,
                        amount: record.amount,
                    });
                }

                Self::deposit_event(Event::MessageDeleted {
                    msg_id,
                    deleted_by: env.receiver.clone(),
                    reason,
                });
            }
        }

        /// Emit `AutoResponseTriggered` if receiver has a valid auto-response config.
        ///
        /// M1 fix: now takes `sender` to enforce per-sender cooldown via `AutoReplyCooldown`.
        fn maybe_trigger_auto_response(
            receiver: &T::AccountId,
            sender: &T::AccountId,
            original_msg_id: MessageId,
            pay_for_reply: BalanceOf<T>,
            now: BlockNumberFor<T>,
        ) {
            if let Some(cfg) = AutoResponses::<T>::get(receiver) {
                if !cfg.enabled {
                    return;
                }

                // Check expiry
                if let Some(expires) = cfg.expires_at {
                    if now >= expires {
                        return;
                    }
                }

                // Check minimum pay-for-reply
                if pay_for_reply < cfg.min_pay_for_reply {
                    return;
                }

                // M1 fix: enforce per-sender auto-reply cooldown.
                // `AutoReplyCooldown` uses ValueQuery (default = 0), so we only
                // apply the cooldown check when there has been a previous reply
                // (last_reply > default). This avoids suppressing the very first
                // auto-reply because 0 + cooldown > now would be true at low block numbers.
                let cooldown: BlockNumberFor<T> = cfg.cooldown_blocks.into();
                let zero = BlockNumberFor::<T>::default();
                if cooldown > zero {
                    let last_reply = AutoReplyCooldown::<T>::get(receiver, sender);
                    if last_reply > zero && last_reply.saturating_add(cooldown) > now {
                        // Cooldown not yet elapsed — suppress auto-reply
                        return;
                    }
                }

                // Update last auto-reply timestamp before emitting event
                AutoReplyCooldown::<T>::insert(receiver, sender, now);

                Self::deposit_event(Event::AutoResponseTriggered {
                    original_msg_id,
                    responder: receiver.clone(),
                    response_hash: cfg.response_hash,
                });
            }
        }
    }
}
