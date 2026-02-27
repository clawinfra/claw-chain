//! # Service Market Pallet v2
//!
//! Reputation-gated service marketplace for ClawChain agents.
//!
//! ## Overview
//!
//! This pallet replaces `pallet-task-market` with a persistent service catalog model:
//!
//! - **ServiceListing**: A provider's persistent, reusable service offering with tags,
//!   pricing, SLA parameters, and reputation gating.
//! - **ServiceInvocation**: A single execution request against a listing, with escrow
//!   and milestone-based partial payment release.
//! - **Reputation Gating**: Minimum reputation score required to list services; optional
//!   per-listing gate for invokers.
//! - **Dispute Resolution**: Either party can raise a dispute; governance resolves.
//!
//! ## Extrinsics (Phase 1 — indices 10–27 where implemented)
//!
//! - `list_service` (10) — Create a service listing
//! - `update_listing` (11) — Update listing metadata
//! - `delist_service` (12) — Deactivate a listing
//! - `invoke_service` (13) — Invoke a service (escrow path)
//! - `submit_invocation_work` (18) — Provider submits work proof
//! - `approve_milestone` (19) — Invoker approves milestone, releases partial escrow
//! - `raise_dispute` (20) — Either party raises a dispute
//! - `resolve_dispute_governance` (23) — Governance resolves escalated dispute
//! - `cancel_invocation` (26) — Invoker cancels pending invocation
//! - `try_expire_invocation` (27) — Anyone triggers expiry after deadline

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use pallet_reputation::ReputationManager;
    use sp_runtime::traits::AccountIdConversion;

    // =========================================================
    // Type Aliases
    // =========================================================

    pub type ListingId = u64;
    pub type InvocationId = u64;
    pub type DisputeId = u64;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // =========================================================
    // Enums
    // =========================================================

    /// Payment mode for a service listing.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub enum PaymentMode {
        /// Lock CLAW in escrow before invocation.
        #[default]
        Escrow,
        /// Pay off-chain, register claim after (X402 — Phase 2).
        X402,
        /// Provider accepts either payment mode.
        Either,
    }

    /// Status of a service invocation.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub enum InvocationStatus {
        /// Created, awaiting provider acceptance.
        #[default]
        Pending,
        /// Provider acknowledged.
        Accepted,
        /// Work in progress.
        InProgress,
        /// Provider submitted proof.
        WorkSubmitted,
        /// All milestones approved, payment released.
        FullyApproved,
        /// Under dispute resolution.
        Disputed,
        /// Invoker cancelled.
        Cancelled,
        /// Deadline passed with no action.
        Expired,
    }

    /// Status of a milestone within an invocation.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub enum MilestoneStatus {
        #[default]
        Pending,
        Submitted,
        Approved,
        Disputed,
    }

    /// Status of a dispute.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub enum DisputeStatus {
        #[default]
        Open,
        Resolved,
        Escalated,
    }

    /// Proof type for work submission.
    #[derive(
        Clone,
        Encode,
        Decode,
        Eq,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        Default,
        codec::DecodeWithMemTracking,
    )]
    pub enum ProofType {
        /// On-chain content hash.
        #[default]
        Hash,
        /// IPFS CID.
        Cid,
        /// Signed attestation.
        Attestation,
    }

    // =========================================================
    // Structs
    // =========================================================

    /// A persistent service listing by a provider.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ServiceListing<T: Config> {
        pub id: ListingId,
        pub provider: T::AccountId,
        pub name: BoundedVec<u8, T::MaxNameLength>,
        pub description: BoundedVec<u8, T::MaxDescriptionLength>,
        pub tags: BoundedVec<BoundedVec<u8, T::MaxTagLength>, T::MaxTagsPerListing>,
        pub min_price: BalanceOf<T>,
        pub max_price: BalanceOf<T>,
        pub payment_mode: PaymentMode,
        pub sla_response_blocks: u32,
        pub sla_completion_blocks: u32,
        pub auto_approve_delay_blocks: u32,
        pub min_invoker_reputation: Option<u32>,
        pub milestones_required: bool,
        pub active: bool,
        pub created_at: BlockNumberFor<T>,
        pub total_invocations: u32,
        pub successful_invocations: u32,
    }

    impl<T: Config> codec::DecodeWithMemTracking for ServiceListing<T> {}

    /// A single milestone within an invocation.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Milestone<T: Config> {
        pub description: BoundedVec<u8, T::MaxMilestoneDescLength>,
        /// Percentage of total price (milestones must sum to 100).
        pub pct_of_total: u8,
        pub status: MilestoneStatus,
        pub submitted_at: Option<BlockNumberFor<T>>,
        pub approved_at: Option<BlockNumberFor<T>>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for Milestone<T> {}

    /// A service invocation (single execution of a listing).
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ServiceInvocation<T: Config> {
        pub id: InvocationId,
        pub listing_id: ListingId,
        pub invoker: T::AccountId,
        pub provider: T::AccountId,
        pub requirements: BoundedVec<u8, T::MaxDescriptionLength>,
        pub price: BalanceOf<T>,
        pub payment_mode: PaymentMode,
        pub status: InvocationStatus,
        pub milestones: BoundedVec<Milestone<T>, T::MaxMilestones>,
        pub deadline: BlockNumberFor<T>,
        pub created_at: BlockNumberFor<T>,
        pub accepted_at: Option<BlockNumberFor<T>>,
        pub completed_at: Option<BlockNumberFor<T>>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for ServiceInvocation<T> {}

    /// A work proof submitted by a provider.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct WorkProof<T: Config> {
        pub proof_cid: BoundedVec<u8, T::MaxCidLength>,
        pub proof_type: ProofType,
        pub submitted_at: BlockNumberFor<T>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for WorkProof<T> {}

    /// A dispute record.
    #[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct DisputeRecord<T: Config> {
        pub id: DisputeId,
        pub invocation_id: InvocationId,
        pub raised_by: T::AccountId,
        pub reason: BoundedVec<u8, T::MaxDescriptionLength>,
        pub evidence_cid: Option<BoundedVec<u8, T::MaxCidLength>>,
        pub status: DisputeStatus,
        pub raised_at: BlockNumberFor<T>,
        pub winner: Option<T::AccountId>,
    }

    impl<T: Config> codec::DecodeWithMemTracking for DisputeRecord<T> {}

    /// Spec for a milestone provided at invocation time.
    #[derive(
        Clone,
        Encode,
        Decode,
        PartialEq,
        RuntimeDebug,
        TypeInfo,
        MaxEncodedLen,
        codec::DecodeWithMemTracking,
    )]
    pub struct MilestoneSpec {
        pub pct_of_total: u8,
    }

    // =========================================================
    // Config
    // =========================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type WeightInfo: WeightInfo;

        type Currency: Currency<Self::AccountId>;

        type ReputationManager: ReputationManager<Self::AccountId, BalanceOf<Self>>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Minimum reputation score to create a service listing (basis points, 0–10000).
        #[pallet::constant]
        type MinListingReputation: Get<u32>;

        /// Maximum number of tags per listing.
        #[pallet::constant]
        type MaxTagsPerListing: Get<u32>;

        /// Maximum length of a single tag (bytes).
        #[pallet::constant]
        type MaxTagLength: Get<u32>;

        /// Maximum number of listings indexed per tag.
        #[pallet::constant]
        type MaxListingsPerTag: Get<u32>;

        /// Maximum number of listings per provider.
        #[pallet::constant]
        type MaxListingsPerProvider: Get<u32>;

        /// Maximum milestones per invocation.
        #[pallet::constant]
        type MaxMilestones: Get<u32>;

        /// Maximum length of milestone description (bytes).
        #[pallet::constant]
        type MaxMilestoneDescLength: Get<u32>;

        /// Maximum active invocations per account.
        #[pallet::constant]
        type MaxActiveInvocationsPerAccount: Get<u32>;

        /// Maximum length of a service name (bytes).
        #[pallet::constant]
        type MaxNameLength: Get<u32>;

        /// Maximum length of description / requirements (bytes).
        #[pallet::constant]
        type MaxDescriptionLength: Get<u32>;

        /// Maximum length of a CID (bytes).
        #[pallet::constant]
        type MaxCidLength: Get<u32>;

        /// Safety cap on auto_approve_delay_blocks.
        #[pallet::constant]
        type AutoApproveMaxDelay: Get<u32>;

        /// Bounty paid to the caller of `try_expire_invocation`.
        #[pallet::constant]
        type ExpireBounty: Get<BalanceOf<Self>>;

        /// Maximum expirations processed per `on_initialize` block.
        #[pallet::constant]
        type MaxExpirationsPerBlock: Get<u32>;
    }

    // =========================================================
    // Pallet
    // =========================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // =========================================================
    // Storage
    // =========================================================

    #[pallet::storage]
    pub type ServiceListings<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ServiceListing<T>, OptionQuery>;

    #[pallet::storage]
    pub type ListingCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type ListingsByTag<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BoundedVec<u8, T::MaxTagLength>,
        BoundedVec<ListingId, T::MaxListingsPerTag>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type ListingsByProvider<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<ListingId, T::MaxListingsPerProvider>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type ServiceInvocations<T: Config> =
        StorageMap<_, Blake2_128Concat, InvocationId, ServiceInvocation<T>, OptionQuery>;

    #[pallet::storage]
    pub type InvocationCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type InvocationsByListing<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ListingId,
        Blake2_128Concat,
        InvocationId,
        (),
        OptionQuery,
    >;

    #[pallet::storage]
    pub type InvocationsByDeadline<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        InvocationId,
        (),
        OptionQuery,
    >;

    #[pallet::storage]
    pub type InvocationsByInvoker<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<InvocationId, T::MaxActiveInvocationsPerAccount>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type InvocationProofs<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        InvocationId,
        Twox64Concat,
        u32,
        WorkProof<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    pub type Disputes<T: Config> =
        StorageMap<_, Blake2_128Concat, DisputeId, DisputeRecord<T>, OptionQuery>;

    #[pallet::storage]
    pub type DisputeCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    // =========================================================
    // Hooks
    // =========================================================

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::expire_overdue_invocations(n)
        }
    }

    // =========================================================
    // Events
    // =========================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ServiceListed {
            listing_id: ListingId,
            provider: T::AccountId,
            min_price: BalanceOf<T>,
        },
        ListingUpdated {
            listing_id: ListingId,
        },
        ServiceDelisted {
            listing_id: ListingId,
        },
        ServiceInvoked {
            invocation_id: InvocationId,
            listing_id: ListingId,
            invoker: T::AccountId,
            provider: T::AccountId,
            price: BalanceOf<T>,
        },
        WorkSubmitted {
            invocation_id: InvocationId,
            milestone_index: Option<u32>,
        },
        MilestoneApproved {
            invocation_id: InvocationId,
            milestone_index: u32,
            amount_released: BalanceOf<T>,
        },
        InvocationFullyApproved {
            invocation_id: InvocationId,
            total_paid: BalanceOf<T>,
        },
        InvocationCancelled {
            invocation_id: InvocationId,
        },
        InvocationExpired {
            invocation_id: InvocationId,
            expired_by: T::AccountId,
        },
        DisputeRaised {
            invocation_id: InvocationId,
            dispute_id: DisputeId,
            raised_by: T::AccountId,
        },
        DisputeResolvedByGovernance {
            dispute_id: DisputeId,
            winner: T::AccountId,
        },
    }

    // =========================================================
    // Errors
    // =========================================================

    #[pallet::error]
    pub enum Error<T> {
        ListingNotFound,
        ListingNotActive,
        NotProvider,
        NotInvoker,
        InsufficientReputation,
        InvocationNotFound,
        InvalidInvocationStatus,
        MilestoneIndexOutOfBounds,
        MilestoneAlreadyApproved,
        MilestoneNotSubmitted,
        MilestonePercentagesInvalid,
        TooManyMilestones,
        TooManyTags,
        TagTooLong,
        TooManyListingsForTag,
        TooManyListingsForProvider,
        TooManyActiveInvocations,
        NameTooLong,
        DescriptionTooLong,
        CidTooLong,
        AutoApproveDelayTooLong,
        PriceBelowMinimum,
        PriceAboveMaximum,
        CannotCancelActiveInvocation,
        DeadlineNotPassed,
        ListingHasActiveInvocations,
        InsufficientBalance,
        DisputeNotFound,
        DisputeNotEscalated,
        InvocationAlreadyDisputed,
        NotPartyToInvocation,
        RequirementsEmpty,
    }

    // =========================================================
    // Weight trait
    // =========================================================

    pub trait WeightInfo {
        fn list_service() -> Weight;
        fn update_listing() -> Weight;
        fn delist_service() -> Weight;
        fn invoke_service() -> Weight;
        fn submit_invocation_work() -> Weight;
        fn approve_milestone() -> Weight;
        fn raise_dispute() -> Weight;
        fn resolve_dispute_governance() -> Weight;
        fn cancel_invocation() -> Weight;
        fn try_expire_invocation() -> Weight;
    }

    pub struct SubstrateWeight<T>(core::marker::PhantomData<T>);

    impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
        fn list_service() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn update_listing() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn delist_service() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn invoke_service() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn submit_invocation_work() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn approve_milestone() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn raise_dispute() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn resolve_dispute_governance() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn cancel_invocation() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn try_expire_invocation() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }

    // =========================================================
    // Extrinsics
    // =========================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// (Index 10) Create a persistent service listing.
        ///
        /// The caller must have sufficient reputation (`MinListingReputation`).
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::list_service())]
        pub fn list_service(
            origin: OriginFor<T>,
            name: Vec<u8>,
            description: Vec<u8>,
            tags: Vec<Vec<u8>>,
            min_price: BalanceOf<T>,
            max_price: BalanceOf<T>,
            payment_mode: PaymentMode,
            sla_response_blocks: u32,
            sla_completion_blocks: u32,
            auto_approve_delay_blocks: u32,
            min_invoker_reputation: Option<u32>,
            milestones_required: bool,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            // Reputation gate
            ensure!(
                T::ReputationManager::meets_minimum_reputation(
                    &provider,
                    T::MinListingReputation::get()
                ),
                Error::<T>::InsufficientReputation
            );

            ensure!(
                auto_approve_delay_blocks <= T::AutoApproveMaxDelay::get(),
                Error::<T>::AutoApproveDelayTooLong
            );

            let name: BoundedVec<u8, T::MaxNameLength> =
                name.try_into().map_err(|_| Error::<T>::NameTooLong)?;
            let description: BoundedVec<u8, T::MaxDescriptionLength> = description
                .try_into()
                .map_err(|_| Error::<T>::DescriptionTooLong)?;

            ensure!(
                tags.len() <= T::MaxTagsPerListing::get() as usize,
                Error::<T>::TooManyTags
            );

            let mut bounded_tags: BoundedVec<
                BoundedVec<u8, T::MaxTagLength>,
                T::MaxTagsPerListing,
            > = BoundedVec::new();
            for tag in &tags {
                let bounded_tag: BoundedVec<u8, T::MaxTagLength> =
                    tag.clone().try_into().map_err(|_| Error::<T>::TagTooLong)?;
                bounded_tags
                    .try_push(bounded_tag)
                    .map_err(|_| Error::<T>::TooManyTags)?;
            }

            let listing_id = ListingCount::<T>::get();

            let now = <frame_system::Pallet<T>>::block_number();

            let listing = ServiceListing {
                id: listing_id,
                provider: provider.clone(),
                name,
                description,
                tags: bounded_tags.clone(),
                min_price,
                max_price,
                payment_mode,
                sla_response_blocks,
                sla_completion_blocks,
                auto_approve_delay_blocks,
                min_invoker_reputation,
                milestones_required,
                active: true,
                created_at: now,
                total_invocations: 0,
                successful_invocations: 0,
            };

            ServiceListings::<T>::insert(listing_id, listing);
            ListingCount::<T>::put(listing_id + 1);

            // Update indexes
            ListingsByProvider::<T>::try_mutate(&provider, |ids| {
                ids.try_push(listing_id)
                    .map_err(|_| Error::<T>::TooManyListingsForProvider)
            })?;

            for tag in &bounded_tags {
                ListingsByTag::<T>::try_mutate(tag, |ids| {
                    ids.try_push(listing_id)
                        .map_err(|_| Error::<T>::TooManyListingsForTag)
                })?;
            }

            Self::deposit_event(Event::ServiceListed {
                listing_id,
                provider,
                min_price,
            });

            Ok(())
        }

        /// (Index 11) Update listing metadata.
        ///
        /// Only the listing provider may update. Cannot update if there are active invocations.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::update_listing())]
        pub fn update_listing(
            origin: OriginFor<T>,
            listing_id: ListingId,
            name: Option<Vec<u8>>,
            description: Option<Vec<u8>>,
            min_price: Option<BalanceOf<T>>,
            max_price: Option<BalanceOf<T>>,
            sla_response_blocks: Option<u32>,
            sla_completion_blocks: Option<u32>,
            auto_approve_delay_blocks: Option<u32>,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            ServiceListings::<T>::try_mutate(listing_id, |maybe_listing| {
                let listing = maybe_listing.as_mut().ok_or(Error::<T>::ListingNotFound)?;
                ensure!(listing.provider == provider, Error::<T>::NotProvider);

                // Ensure no active invocations
                let active_count = InvocationsByListing::<T>::iter_prefix(listing_id).count();
                ensure!(active_count == 0, Error::<T>::ListingHasActiveInvocations);

                if let Some(n) = name {
                    listing.name = n.try_into().map_err(|_| Error::<T>::NameTooLong)?;
                }
                if let Some(d) = description {
                    listing.description =
                        d.try_into().map_err(|_| Error::<T>::DescriptionTooLong)?;
                }
                if let Some(p) = min_price {
                    listing.min_price = p;
                }
                if let Some(p) = max_price {
                    listing.max_price = p;
                }
                if let Some(s) = sla_response_blocks {
                    listing.sla_response_blocks = s;
                }
                if let Some(s) = sla_completion_blocks {
                    listing.sla_completion_blocks = s;
                }
                if let Some(d) = auto_approve_delay_blocks {
                    ensure!(
                        d <= T::AutoApproveMaxDelay::get(),
                        Error::<T>::AutoApproveDelayTooLong
                    );
                    listing.auto_approve_delay_blocks = d;
                }

                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::ListingUpdated { listing_id });
            Ok(())
        }

        /// (Index 12) Deactivate a service listing.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::delist_service())]
        pub fn delist_service(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            ServiceListings::<T>::try_mutate(listing_id, |maybe_listing| {
                let listing = maybe_listing.as_mut().ok_or(Error::<T>::ListingNotFound)?;
                ensure!(listing.provider == provider, Error::<T>::NotProvider);
                listing.active = false;
                Ok::<(), DispatchError>(())
            })?;

            // Remove from tag indexes
            let listing =
                ServiceListings::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            for tag in &listing.tags {
                ListingsByTag::<T>::mutate(tag, |ids| {
                    ids.retain(|&id| id != listing_id);
                });
            }

            Self::deposit_event(Event::ServiceDelisted { listing_id });
            Ok(())
        }

        /// (Index 13) Invoke a service listing (escrow path).
        ///
        /// Locks `agreed_price` in the pallet's escrow sub-account derived from
        /// the invocation ID. Provider must accept before work starts.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::invoke_service())]
        pub fn invoke_service(
            origin: OriginFor<T>,
            listing_id: ListingId,
            requirements: Vec<u8>,
            milestones: Option<Vec<MilestoneSpec>>,
            agreed_price: BalanceOf<T>,
            deadline_blocks: u32,
        ) -> DispatchResult {
            let invoker = ensure_signed(origin)?;

            let listing =
                ServiceListings::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;

            ensure!(listing.active, Error::<T>::ListingNotActive);
            ensure!(
                agreed_price >= listing.min_price,
                Error::<T>::PriceBelowMinimum
            );
            ensure!(
                listing.max_price == listing.min_price || agreed_price <= listing.max_price,
                Error::<T>::PriceAboveMaximum
            );

            // Per-listing invoker reputation gate
            if let Some(min_rep) = listing.min_invoker_reputation {
                ensure!(
                    T::ReputationManager::meets_minimum_reputation(&invoker, min_rep),
                    Error::<T>::InsufficientReputation
                );
            }

            let requirements: BoundedVec<u8, T::MaxDescriptionLength> = requirements
                .try_into()
                .map_err(|_| Error::<T>::DescriptionTooLong)?;

            // Build milestones
            let bounded_milestones = Self::build_milestones(milestones)?;

            let invocation_id = InvocationCount::<T>::get();
            let now = <frame_system::Pallet<T>>::block_number();
            let deadline = now + deadline_blocks.into();

            // Lock escrow (transfer from invoker to pallet escrow sub-account)
            let escrow_account = Self::invocation_escrow_account(invocation_id);
            T::Currency::transfer(
                &invoker,
                &escrow_account,
                agreed_price,
                ExistenceRequirement::KeepAlive,
            )
            .map_err(|_| Error::<T>::InsufficientBalance)?;

            let invocation = ServiceInvocation {
                id: invocation_id,
                listing_id,
                invoker: invoker.clone(),
                provider: listing.provider.clone(),
                requirements,
                price: agreed_price,
                payment_mode: PaymentMode::Escrow,
                status: InvocationStatus::Pending,
                milestones: bounded_milestones,
                deadline,
                created_at: now,
                accepted_at: None,
                completed_at: None,
            };

            ServiceInvocations::<T>::insert(invocation_id, invocation);
            InvocationCount::<T>::put(invocation_id + 1);
            InvocationsByListing::<T>::insert(listing_id, invocation_id, ());
            InvocationsByDeadline::<T>::insert(deadline, invocation_id, ());

            InvocationsByInvoker::<T>::try_mutate(&invoker, |ids| {
                ids.try_push(invocation_id)
                    .map_err(|_| Error::<T>::TooManyActiveInvocations)
            })?;

            // Update listing stats
            ServiceListings::<T>::mutate(listing_id, |maybe| {
                if let Some(l) = maybe {
                    l.total_invocations = l.total_invocations.saturating_add(1);
                }
            });

            Self::deposit_event(Event::ServiceInvoked {
                invocation_id,
                listing_id,
                invoker,
                provider: listing.provider,
                price: agreed_price,
            });

            Ok(())
        }

        /// (Index 18) Provider submits work proof for an invocation.
        #[pallet::call_index(18)]
        #[pallet::weight(T::WeightInfo::submit_invocation_work())]
        pub fn submit_invocation_work(
            origin: OriginFor<T>,
            invocation_id: InvocationId,
            milestone_index: Option<u32>,
            proof_cid: Vec<u8>,
            proof_type: ProofType,
        ) -> DispatchResult {
            let provider = ensure_signed(origin)?;

            let proof_cid: BoundedVec<u8, T::MaxCidLength> =
                proof_cid.try_into().map_err(|_| Error::<T>::CidTooLong)?;

            ServiceInvocations::<T>::try_mutate(invocation_id, |maybe| {
                let inv = maybe.as_mut().ok_or(Error::<T>::InvocationNotFound)?;
                ensure!(inv.provider == provider, Error::<T>::NotProvider);
                ensure!(
                    matches!(
                        inv.status,
                        InvocationStatus::Pending
                            | InvocationStatus::Accepted
                            | InvocationStatus::InProgress
                            | InvocationStatus::WorkSubmitted
                    ),
                    Error::<T>::InvalidInvocationStatus
                );

                let now = <frame_system::Pallet<T>>::block_number();

                // Update milestone status if milestone_index provided
                if let Some(idx) = milestone_index {
                    let ms = inv
                        .milestones
                        .get_mut(idx as usize)
                        .ok_or(Error::<T>::MilestoneIndexOutOfBounds)?;
                    ms.status = MilestoneStatus::Submitted;
                    ms.submitted_at = Some(now);
                }

                // Mark invocation as work submitted if no milestones or all submitted
                inv.status = InvocationStatus::WorkSubmitted;
                inv.completed_at = Some(now);

                Ok::<(), DispatchError>(())
            })?;

            let now = <frame_system::Pallet<T>>::block_number();
            let proof = WorkProof {
                proof_cid,
                proof_type,
                submitted_at: now,
            };
            let key = milestone_index.unwrap_or(u32::MAX);
            InvocationProofs::<T>::insert(invocation_id, key, proof);

            Self::deposit_event(Event::WorkSubmitted {
                invocation_id,
                milestone_index,
            });

            Ok(())
        }

        /// (Index 19) Invoker approves a milestone and releases partial escrow.
        #[pallet::call_index(19)]
        #[pallet::weight(T::WeightInfo::approve_milestone())]
        pub fn approve_milestone(
            origin: OriginFor<T>,
            invocation_id: InvocationId,
            milestone_index: u32,
        ) -> DispatchResult {
            let invoker = ensure_signed(origin)?;

            let (provider, amount_released, fully_approved) =
                ServiceInvocations::<T>::try_mutate(invocation_id, |maybe| {
                    let inv = maybe.as_mut().ok_or(Error::<T>::InvocationNotFound)?;
                    ensure!(inv.invoker == invoker, Error::<T>::NotInvoker);
                    ensure!(
                        matches!(
                            inv.status,
                            InvocationStatus::WorkSubmitted
                                | InvocationStatus::InProgress
                                | InvocationStatus::Accepted
                        ),
                        Error::<T>::InvalidInvocationStatus
                    );

                    let total_price = inv.price;
                    let provider = inv.provider.clone();

                    if inv.milestones.is_empty() {
                        // Single-milestone: release everything
                        inv.status = InvocationStatus::FullyApproved;
                        return Ok((provider, total_price, true));
                    }

                    let ms = inv
                        .milestones
                        .get_mut(milestone_index as usize)
                        .ok_or(Error::<T>::MilestoneIndexOutOfBounds)?;

                    ensure!(
                        !matches!(ms.status, MilestoneStatus::Approved),
                        Error::<T>::MilestoneAlreadyApproved
                    );
                    ensure!(
                        matches!(ms.status, MilestoneStatus::Submitted),
                        Error::<T>::MilestoneNotSubmitted
                    );

                    let pct = ms.pct_of_total as u128;
                    // Use saturating arithmetic to avoid overflow
                    // Simple percentage calc: total_price * pct / 100
                    let amount_released: BalanceOf<T> = Self::percent_of(total_price, pct);

                    let now = <frame_system::Pallet<T>>::block_number();
                    ms.status = MilestoneStatus::Approved;
                    ms.approved_at = Some(now);

                    // Check if all milestones are approved
                    let all_approved = inv
                        .milestones
                        .iter()
                        .all(|m| matches!(m.status, MilestoneStatus::Approved));
                    if all_approved {
                        inv.status = InvocationStatus::FullyApproved;
                    }

                    Ok::<_, DispatchError>((provider, amount_released, all_approved))
                })?;

            // Transfer from escrow to provider
            let escrow_account = Self::invocation_escrow_account(invocation_id);
            T::Currency::transfer(
                &escrow_account,
                &provider,
                amount_released,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::InsufficientBalance)?;

            Self::deposit_event(Event::MilestoneApproved {
                invocation_id,
                milestone_index,
                amount_released,
            });

            if fully_approved {
                // Update listing success count
                let listing_id = ServiceInvocations::<T>::get(invocation_id)
                    .map(|i| i.listing_id)
                    .unwrap_or(0);
                ServiceListings::<T>::mutate(listing_id, |maybe| {
                    if let Some(l) = maybe {
                        l.successful_invocations = l.successful_invocations.saturating_add(1);
                    }
                });

                // Reputation updates
                T::ReputationManager::on_task_completed(&provider, amount_released);

                Self::deposit_event(Event::InvocationFullyApproved {
                    invocation_id,
                    total_paid: amount_released,
                });

                // Clean up indexes
                Self::cleanup_invocation(invocation_id);
            }

            Ok(())
        }

        /// (Index 20) Raise a dispute on an invocation.
        #[pallet::call_index(20)]
        #[pallet::weight(T::WeightInfo::raise_dispute())]
        pub fn raise_dispute(
            origin: OriginFor<T>,
            invocation_id: InvocationId,
            reason: Vec<u8>,
            evidence_cid: Option<Vec<u8>>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let reason: BoundedVec<u8, T::MaxDescriptionLength> = reason
                .try_into()
                .map_err(|_| Error::<T>::DescriptionTooLong)?;

            let evidence: Option<BoundedVec<u8, T::MaxCidLength>> = evidence_cid
                .map(|c| c.try_into().map_err(|_| Error::<T>::CidTooLong))
                .transpose()?;

            ServiceInvocations::<T>::try_mutate(invocation_id, |maybe| {
                let inv = maybe.as_mut().ok_or(Error::<T>::InvocationNotFound)?;

                ensure!(
                    inv.invoker == caller || inv.provider == caller,
                    Error::<T>::NotPartyToInvocation
                );
                ensure!(
                    !matches!(inv.status, InvocationStatus::Disputed),
                    Error::<T>::InvocationAlreadyDisputed
                );
                ensure!(
                    matches!(
                        inv.status,
                        InvocationStatus::Pending
                            | InvocationStatus::Accepted
                            | InvocationStatus::InProgress
                            | InvocationStatus::WorkSubmitted
                    ),
                    Error::<T>::InvalidInvocationStatus
                );

                inv.status = InvocationStatus::Disputed;
                Ok::<(), DispatchError>(())
            })?;

            let dispute_id = DisputeCount::<T>::get();
            let now = <frame_system::Pallet<T>>::block_number();

            let dispute = DisputeRecord {
                id: dispute_id,
                invocation_id,
                raised_by: caller.clone(),
                reason,
                evidence_cid: evidence,
                status: DisputeStatus::Open,
                raised_at: now,
                winner: None,
            };

            Disputes::<T>::insert(dispute_id, dispute);
            DisputeCount::<T>::put(dispute_id + 1);

            Self::deposit_event(Event::DisputeRaised {
                invocation_id,
                dispute_id,
                raised_by: caller,
            });

            Ok(())
        }

        /// (Index 23) Governance resolves an escalated dispute.
        #[pallet::call_index(23)]
        #[pallet::weight(T::WeightInfo::resolve_dispute_governance())]
        pub fn resolve_dispute_governance(
            origin: OriginFor<T>,
            dispute_id: DisputeId,
            winner: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let invocation_id = Disputes::<T>::try_mutate(dispute_id, |maybe| {
                let dispute = maybe.as_mut().ok_or(Error::<T>::DisputeNotFound)?;
                // Allow resolution from Open or Escalated (governance can always resolve)
                dispute.status = DisputeStatus::Resolved;
                dispute.winner = Some(winner.clone());
                Ok::<InvocationId, DispatchError>(dispute.invocation_id)
            })?;

            // Transfer escrow to winner
            let escrow_account = Self::invocation_escrow_account(invocation_id);
            let escrow_balance = T::Currency::free_balance(&escrow_account);
            if escrow_balance > T::Currency::minimum_balance() {
                T::Currency::transfer(
                    &escrow_account,
                    &winner,
                    escrow_balance - T::Currency::minimum_balance(),
                    ExistenceRequirement::AllowDeath,
                )
                .ok(); // Best effort
            }

            // Mark invocation resolved
            ServiceInvocations::<T>::mutate(invocation_id, |maybe| {
                if let Some(inv) = maybe {
                    inv.status = InvocationStatus::FullyApproved;
                }
            });

            // Reputation update via dispute resolution
            let inv = ServiceInvocations::<T>::get(invocation_id);
            if let Some(inv) = inv {
                let loser = if inv.invoker == winner {
                    inv.provider.clone()
                } else {
                    inv.invoker.clone()
                };
                T::ReputationManager::on_dispute_resolved(&winner, &loser);
            }

            Self::cleanup_invocation(invocation_id);

            Self::deposit_event(Event::DisputeResolvedByGovernance { dispute_id, winner });

            Ok(())
        }

        /// (Index 26) Cancel a pending invocation and refund escrow.
        #[pallet::call_index(26)]
        #[pallet::weight(T::WeightInfo::cancel_invocation())]
        pub fn cancel_invocation(
            origin: OriginFor<T>,
            invocation_id: InvocationId,
        ) -> DispatchResult {
            let invoker = ensure_signed(origin)?;

            let price = ServiceInvocations::<T>::try_mutate(invocation_id, |maybe| {
                let inv = maybe.as_mut().ok_or(Error::<T>::InvocationNotFound)?;
                ensure!(inv.invoker == invoker, Error::<T>::NotInvoker);
                ensure!(
                    matches!(inv.status, InvocationStatus::Pending),
                    Error::<T>::CannotCancelActiveInvocation
                );
                inv.status = InvocationStatus::Cancelled;
                Ok::<BalanceOf<T>, DispatchError>(inv.price)
            })?;

            // Refund escrow
            let escrow_account = Self::invocation_escrow_account(invocation_id);
            T::Currency::transfer(
                &escrow_account,
                &invoker,
                price,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::InsufficientBalance)?;

            Self::cleanup_invocation(invocation_id);

            Self::deposit_event(Event::InvocationCancelled { invocation_id });

            Ok(())
        }

        /// (Index 27) Permissionless: trigger expiry of a deadline-passed invocation.
        ///
        /// Caller receives `ExpireBounty` from the escrow as reward.
        #[pallet::call_index(27)]
        #[pallet::weight(T::WeightInfo::try_expire_invocation())]
        pub fn try_expire_invocation(
            origin: OriginFor<T>,
            invocation_id: InvocationId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let now = <frame_system::Pallet<T>>::block_number();

            let (invoker, price) = ServiceInvocations::<T>::try_mutate(invocation_id, |maybe| {
                let inv = maybe.as_mut().ok_or(Error::<T>::InvocationNotFound)?;
                ensure!(
                    matches!(
                        inv.status,
                        InvocationStatus::Pending
                            | InvocationStatus::Accepted
                            | InvocationStatus::InProgress
                    ),
                    Error::<T>::InvalidInvocationStatus
                );
                ensure!(inv.deadline < now, Error::<T>::DeadlineNotPassed);
                inv.status = InvocationStatus::Expired;
                Ok::<(T::AccountId, BalanceOf<T>), DispatchError>((inv.invoker.clone(), inv.price))
            })?;

            let escrow_account = Self::invocation_escrow_account(invocation_id);
            let bounty = T::ExpireBounty::get();

            // Pay bounty to caller
            if bounty > T::Currency::minimum_balance() {
                T::Currency::transfer(
                    &escrow_account,
                    &caller,
                    bounty,
                    ExistenceRequirement::AllowDeath,
                )
                .ok();
            }

            // Refund remainder to invoker
            use sp_runtime::traits::Saturating;
            let remainder = price.saturating_sub(bounty);
            if remainder > T::Currency::minimum_balance() {
                T::Currency::transfer(
                    &escrow_account,
                    &invoker,
                    remainder,
                    ExistenceRequirement::AllowDeath,
                )
                .ok();
            }

            Self::cleanup_invocation(invocation_id);

            Self::deposit_event(Event::InvocationExpired {
                invocation_id,
                expired_by: caller,
            });

            Ok(())
        }
    }

    // =========================================================
    // Internal helpers
    // =========================================================

    impl<T: Config> Pallet<T> {
        /// Derive the escrow sub-account for a given invocation.
        pub fn invocation_escrow_account(invocation_id: InvocationId) -> T::AccountId {
            T::PalletId::get().into_sub_account_truncating(invocation_id)
        }

        /// Compute `(value * pct) / 100` for balance types.
        fn percent_of(value: BalanceOf<T>, pct: u128) -> BalanceOf<T> {
            use sp_runtime::traits::SaturatedConversion;
            let v: u128 = value.saturated_into();
            let result = v.saturating_mul(pct) / 100u128;
            result.saturated_into()
        }

        /// Build a BoundedVec of Milestones from caller-provided specs.
        ///
        /// If `specs` is None or empty, returns an empty vec (single-milestone mode).
        fn build_milestones(
            specs: Option<Vec<MilestoneSpec>>,
        ) -> Result<BoundedVec<Milestone<T>, T::MaxMilestones>, DispatchError> {
            let mut milestones: BoundedVec<Milestone<T>, T::MaxMilestones> = BoundedVec::new();

            let specs = match specs {
                None => return Ok(milestones),
                Some(s) if s.is_empty() => return Ok(milestones),
                Some(s) => s,
            };

            ensure!(
                specs.len() <= T::MaxMilestones::get() as usize,
                Error::<T>::TooManyMilestones
            );

            let total_pct: u32 = specs.iter().map(|s| s.pct_of_total as u32).sum();
            ensure!(total_pct == 100, Error::<T>::MilestonePercentagesInvalid);

            for spec in specs {
                let ms = Milestone {
                    description: BoundedVec::new(),
                    pct_of_total: spec.pct_of_total,
                    status: MilestoneStatus::Pending,
                    submitted_at: None,
                    approved_at: None,
                };
                milestones
                    .try_push(ms)
                    .map_err(|_| Error::<T>::TooManyMilestones)?;
            }

            Ok(milestones)
        }

        /// Remove an invocation from the deadline and invoker indexes.
        fn cleanup_invocation(invocation_id: InvocationId) {
            if let Some(inv) = ServiceInvocations::<T>::get(invocation_id) {
                InvocationsByDeadline::<T>::remove(inv.deadline, invocation_id);
                InvocationsByListing::<T>::remove(inv.listing_id, invocation_id);
                InvocationsByInvoker::<T>::mutate(&inv.invoker, |ids| {
                    ids.retain(|&id| id != invocation_id);
                });
            }
        }

        /// Process expired invocations for blocks up to `n`.
        ///
        /// Refunds invokers and marks invocations `Expired`.
        /// Returns the weight consumed.
        pub fn expire_overdue_invocations(n: BlockNumberFor<T>) -> Weight {
            let max = T::MaxExpirationsPerBlock::get();
            let mut count = 0u32;

            // Collect expired invocation IDs first (can't mutate while iterating)
            let expired: Vec<(BlockNumberFor<T>, InvocationId)> =
                InvocationsByDeadline::<T>::iter()
                    .filter(|(deadline, _, _)| *deadline < n)
                    .take(max as usize)
                    .map(|(deadline, id, _)| (deadline, id))
                    .collect();

            for (deadline, invocation_id) in expired {
                ServiceInvocations::<T>::mutate(invocation_id, |maybe| {
                    if let Some(inv) = maybe {
                        if matches!(
                            inv.status,
                            InvocationStatus::Pending
                                | InvocationStatus::Accepted
                                | InvocationStatus::InProgress
                        ) {
                            inv.status = InvocationStatus::Expired;

                            // Refund escrow
                            let escrow = Self::invocation_escrow_account(invocation_id);
                            let bal = T::Currency::free_balance(&escrow);
                            let min = T::Currency::minimum_balance();
                            if bal > min {
                                T::Currency::transfer(
                                    &escrow,
                                    &inv.invoker,
                                    bal - min,
                                    ExistenceRequirement::AllowDeath,
                                )
                                .ok();
                            }
                        }
                    }
                });

                InvocationsByDeadline::<T>::remove(deadline, invocation_id);
                count += 1;
            }

            Weight::from_parts(10_000u64 * count as u64, 0)
        }
    }
}
