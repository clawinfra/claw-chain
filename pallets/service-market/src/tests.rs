//! Unit tests for pallet-service-market v2.

use crate::{self as pallet_service_market, pallet::*, *};
use frame_support::traits::Hooks;
use frame_support::{assert_noop, assert_ok, parameter_types, BoundedVec, PalletId};
use sp_core::H256;
use sp_runtime::DispatchResult;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Reputation: pallet_reputation,
        ServiceMarket: pallet_service_market,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MaxCommentLength: u32 = 256;
    pub const InitialReputation: u32 = 5000;
    pub const MaxReputationDelta: u32 = 500;
    pub const MaxHistoryLength: u32 = 100;
}

impl pallet_reputation::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type MaxCommentLength = MaxCommentLength;
    type InitialReputation = InitialReputation;
    type MaxReputationDelta = MaxReputationDelta;
    type MaxHistoryLength = MaxHistoryLength;
}

parameter_types! {
    pub const ServiceMarketPalletId: PalletId = PalletId(*b"svc-mkt!");
    pub const MinListingReputation: u32 = 1000; // 10% — below InitialReputation (5000)
    pub const HighMinListingReputation: u32 = 9000; // 90% — above InitialReputation
    pub const MaxTagsPerListing: u32 = 8;
    pub const MaxTagLength: u32 = 32;
    pub const MaxListingsPerTag: u32 = 100;
    pub const MaxListingsPerProvider: u32 = 50;
    pub const MaxMilestones: u32 = 10;
    pub const MaxMilestoneDescLength: u32 = 256;
    pub const MaxActiveInvocationsPerAccount: u32 = 20;
    pub const MaxNameLength: u32 = 128;
    pub const MaxDescriptionLength: u32 = 512;
    pub const MaxCidLength: u32 = 96;
    pub const AutoApproveMaxDelay: u32 = 1000;
    pub const ExpireBounty: u64 = 10;
    pub const MaxExpirationsPerBlock: u32 = 5;
}

impl pallet_service_market::Config for Test {
    type WeightInfo = SubstrateWeight<Test>;
    type Currency = Balances;
    type ReputationManager = Reputation;
    type PalletId = ServiceMarketPalletId;
    type MinListingReputation = MinListingReputation;
    type MaxTagsPerListing = MaxTagsPerListing;
    type MaxTagLength = MaxTagLength;
    type MaxListingsPerTag = MaxListingsPerTag;
    type MaxListingsPerProvider = MaxListingsPerProvider;
    type MaxMilestones = MaxMilestones;
    type MaxMilestoneDescLength = MaxMilestoneDescLength;
    type MaxActiveInvocationsPerAccount = MaxActiveInvocationsPerAccount;
    type MaxNameLength = MaxNameLength;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MaxCidLength = MaxCidLength;
    type AutoApproveMaxDelay = AutoApproveMaxDelay;
    type ExpireBounty = ExpireBounty;
    type MaxExpirationsPerBlock = MaxExpirationsPerBlock;
}

// =========================================================
// Test helpers
// =========================================================

const ALICE: u64 = 1;
const BOB: u64 = 2;
const CHARLIE: u64 = 3;
const DAVE: u64 = 4;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 100_000),
            (BOB, 100_000),
            (CHARLIE, 100_000),
            (DAVE, 100_000),
        ],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn list_service_default(provider: u64) -> DispatchResult {
    ServiceMarket::list_service(
        RuntimeOrigin::signed(provider),
        b"AI Inference Service".to_vec(),
        b"Fast LLM inference at scale".to_vec(),
        vec![b"ai/llm-inference".to_vec()],
        100, // min_price
        100, // max_price (fixed price)
        PaymentMode::Escrow,
        10,   // sla_response_blocks
        50,   // sla_completion_blocks
        0,    // auto_approve_delay_blocks
        None, // min_invoker_reputation
        false,
    )
}

fn invoke_service_default(invoker: u64, listing_id: ListingId) -> DispatchResult {
    ServiceMarket::invoke_service(
        RuntimeOrigin::signed(invoker),
        listing_id,
        b"Please run inference on my dataset".to_vec(),
        None,
        100,
        100,
    )
}

// =========================================================
// Listing tests
// =========================================================

#[test]
fn list_service_succeeds_with_valid_params() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert_eq!(listing.provider, ALICE);
        assert_eq!(listing.min_price, 100);
        assert!(listing.active);
        assert_eq!(listing.total_invocations, 0);
    });
}

#[test]
fn list_service_increments_listing_count() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(list_service_default(BOB));
        assert_eq!(ListingCount::<Test>::get(), 2);
    });
}

#[test]
fn list_service_indexes_by_provider() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(list_service_default(ALICE));
        let ids = ListingsByProvider::<Test>::get(ALICE);
        assert_eq!(ids.len(), 2);
    });
}

#[test]
fn list_service_indexes_by_tag() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        let tag: BoundedVec<u8, MaxTagLength> = b"ai/llm-inference".to_vec().try_into().unwrap();
        let ids = ListingsByTag::<Test>::get(&tag);
        assert!(ids.contains(&0));
    });
}

#[test]
fn list_service_fails_below_min_reputation() {
    // Override config: set MinListingReputation to 9000 (above InitialReputation 5000)
    new_test_ext().execute_with(|| {
        // We can't easily swap the constant, so let's test using a fresh account with 0 rep.
        // New account has initial reputation 5000 by pallet-reputation default.
        // To test failure, we test with a constant that's too high.
        // Instead, slash ALICE's reputation to 0 first.
        assert_ok!(pallet_reputation::Pallet::<Test>::slash_reputation(
            RuntimeOrigin::root(),
            ALICE,
            5000,
            b"test slash".to_vec(),
        ));
        // Now ALICE has 0 reputation, but MinListingReputation is 1000 → fails
        assert_noop!(
            list_service_default(ALICE),
            Error::<Test>::InsufficientReputation
        );
    });
}

#[test]
fn list_service_fails_too_many_tags() {
    new_test_ext().execute_with(|| {
        let tags: Vec<Vec<u8>> = (0..9).map(|i| format!("tag/{}", i).into_bytes()).collect();
        assert_noop!(
            ServiceMarket::list_service(
                RuntimeOrigin::signed(ALICE),
                b"name".to_vec(),
                b"desc".to_vec(),
                tags,
                100,
                100,
                PaymentMode::Escrow,
                10,
                50,
                0,
                None,
                false,
            ),
            Error::<Test>::TooManyTags
        );
    });
}

#[test]
fn list_service_fails_name_too_long() {
    new_test_ext().execute_with(|| {
        let long_name = vec![b'x'; 200];
        assert_noop!(
            ServiceMarket::list_service(
                RuntimeOrigin::signed(ALICE),
                long_name,
                b"desc".to_vec(),
                vec![],
                100,
                100,
                PaymentMode::Escrow,
                10,
                50,
                0,
                None,
                false,
            ),
            Error::<Test>::NameTooLong
        );
    });
}

#[test]
fn update_listing_succeeds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::update_listing(
            RuntimeOrigin::signed(ALICE),
            0,
            Some(b"Updated Name".to_vec()),
            None,
            Some(200),
            None,
            None,
            None,
            None,
        ));
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert_eq!(&listing.name[..], b"Updated Name");
        assert_eq!(listing.min_price, 200);
    });
}

#[test]
fn update_listing_fails_if_not_provider() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_noop!(
            ServiceMarket::update_listing(
                RuntimeOrigin::signed(BOB),
                0,
                Some(b"Hack".to_vec()),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::NotProvider
        );
    });
}

#[test]
fn update_listing_fails_with_active_invocations() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_noop!(
            ServiceMarket::update_listing(
                RuntimeOrigin::signed(ALICE),
                0,
                Some(b"New Name".to_vec()),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            Error::<Test>::ListingHasActiveInvocations
        );
    });
}

#[test]
fn delist_service_succeeds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::delist_service(
            RuntimeOrigin::signed(ALICE),
            0
        ));
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert!(!listing.active);
    });
}

#[test]
fn delist_service_fails_if_not_provider() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_noop!(
            ServiceMarket::delist_service(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::NotProvider
        );
    });
}

#[test]
fn delist_service_removes_from_tag_index() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        let tag: BoundedVec<u8, MaxTagLength> = b"ai/llm-inference".to_vec().try_into().unwrap();
        assert!(!ListingsByTag::<Test>::get(&tag).is_empty());

        assert_ok!(ServiceMarket::delist_service(
            RuntimeOrigin::signed(ALICE),
            0
        ));
        // Tag index cleaned up
        assert!(ListingsByTag::<Test>::get(&tag).is_empty());
    });
}

// =========================================================
// Invocation tests
// =========================================================

#[test]
fn invoke_service_escrow_path_locks_funds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));

        let bob_before = Balances::free_balance(BOB);
        assert_ok!(invoke_service_default(BOB, 0));
        let bob_after = Balances::free_balance(BOB);

        assert_eq!(bob_before - bob_after, 100); // 100 locked in escrow

        let escrow = ServiceMarket::invocation_escrow_account(0);
        assert_eq!(Balances::free_balance(escrow), 100);
    });
}

#[test]
fn invoke_service_fails_listing_not_active() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::delist_service(
            RuntimeOrigin::signed(ALICE),
            0
        ));

        assert_noop!(
            invoke_service_default(BOB, 0),
            Error::<Test>::ListingNotActive
        );
    });
}

#[test]
fn invoke_service_fails_price_below_minimum() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE)); // min_price = 100
        assert_noop!(
            ServiceMarket::invoke_service(
                RuntimeOrigin::signed(BOB),
                0,
                b"requirements".to_vec(),
                None,
                50, // below min
                100,
            ),
            Error::<Test>::PriceBelowMinimum
        );
    });
}

#[test]
fn invoke_service_fails_reputation_gate() {
    new_test_ext().execute_with(|| {
        // Create listing with high invoker reputation gate
        assert_ok!(ServiceMarket::list_service(
            RuntimeOrigin::signed(ALICE),
            b"Premium Service".to_vec(),
            b"desc".to_vec(),
            vec![],
            100,
            100,
            PaymentMode::Escrow,
            10,
            50,
            0,
            Some(9000), // min_invoker_reputation = 90%
            false,
        ));

        // BOB has default reputation (5000 < 9000) → fails
        assert_noop!(
            invoke_service_default(BOB, 0),
            Error::<Test>::InsufficientReputation
        );
    });
}

#[test]
fn invoke_service_fails_listing_not_found() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            invoke_service_default(BOB, 999),
            Error::<Test>::ListingNotFound
        );
    });
}

#[test]
fn invoke_service_increments_total_invocations() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert_eq!(listing.total_invocations, 1);
    });
}

#[test]
fn invoke_service_indexes_by_listing_and_invoker() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert!(InvocationsByListing::<Test>::get(0, 0).is_some());
        let invoker_ids = InvocationsByInvoker::<Test>::get(BOB);
        assert!(invoker_ids.contains(&0));
    });
}

#[test]
fn submit_work_succeeds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            None,
            b"QmProofHash123".to_vec(),
            ProofType::Hash,
        ));

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::WorkSubmitted);
    });
}

#[test]
fn submit_work_fails_not_provider() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_noop!(
            ServiceMarket::submit_invocation_work(
                RuntimeOrigin::signed(BOB), // not provider
                0,
                None,
                b"proof".to_vec(),
                ProofType::Hash,
            ),
            Error::<Test>::NotProvider
        );
    });
}

#[test]
fn approve_milestone_single_releases_full_escrow() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        // Submit work
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            None,
            b"proof".to_vec(),
            ProofType::Hash,
        ));

        let alice_before = Balances::free_balance(ALICE);

        // Approve (single-milestone mode)
        assert_ok!(ServiceMarket::approve_milestone(
            RuntimeOrigin::signed(BOB),
            0,
            0,
        ));

        let alice_after = Balances::free_balance(ALICE);
        assert_eq!(alice_after - alice_before, 100); // full escrow released

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::FullyApproved);
    });
}

#[test]
fn approve_milestone_partial_multi_milestone() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));

        // Invoke with 2 milestones: 60% + 40%
        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"requirements".to_vec(),
            Some(vec![
                MilestoneSpec { pct_of_total: 60 },
                MilestoneSpec { pct_of_total: 40 },
            ]),
            100,
            100,
        ));

        // Submit work for milestone 0
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            Some(0),
            b"proof1".to_vec(),
            ProofType::Hash,
        ));

        let alice_before = Balances::free_balance(ALICE);

        // Approve milestone 0 → 60 tokens released
        assert_ok!(ServiceMarket::approve_milestone(
            RuntimeOrigin::signed(BOB),
            0,
            0,
        ));

        let alice_after = Balances::free_balance(ALICE);
        assert_eq!(alice_after - alice_before, 60);

        // Invocation should not be fully approved yet
        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_ne!(inv.status, InvocationStatus::FullyApproved);
    });
}

#[test]
fn approve_all_milestones_marks_fully_approved() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));

        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"requirements".to_vec(),
            Some(vec![
                MilestoneSpec { pct_of_total: 50 },
                MilestoneSpec { pct_of_total: 50 },
            ]),
            100,
            100,
        ));

        // Submit and approve milestone 0
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            Some(0),
            b"proof1".to_vec(),
            ProofType::Hash,
        ));
        assert_ok!(ServiceMarket::approve_milestone(
            RuntimeOrigin::signed(BOB),
            0,
            0
        ));

        // Submit and approve milestone 1
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            Some(1),
            b"proof2".to_vec(),
            ProofType::Hash,
        ));
        assert_ok!(ServiceMarket::approve_milestone(
            RuntimeOrigin::signed(BOB),
            0,
            1
        ));

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::FullyApproved);
    });
}

#[test]
fn approve_milestone_fails_not_invoker() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            None,
            b"proof".to_vec(),
            ProofType::Hash,
        ));
        assert_noop!(
            ServiceMarket::approve_milestone(
                RuntimeOrigin::signed(CHARLIE), // not invoker
                0,
                0,
            ),
            Error::<Test>::NotInvoker
        );
    });
}

#[test]
fn approve_milestone_fails_already_approved() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"req".to_vec(),
            Some(vec![
                MilestoneSpec { pct_of_total: 60 },
                MilestoneSpec { pct_of_total: 40 },
            ]),
            100,
            100,
        ));
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            Some(0),
            b"proof".to_vec(),
            ProofType::Hash,
        ));
        assert_ok!(ServiceMarket::approve_milestone(
            RuntimeOrigin::signed(BOB),
            0,
            0
        ));

        // Second approval on same milestone
        assert_noop!(
            ServiceMarket::approve_milestone(RuntimeOrigin::signed(BOB), 0, 0),
            Error::<Test>::MilestoneAlreadyApproved
        );
    });
}

#[test]
fn milestone_percentages_must_sum_to_100() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_noop!(
            ServiceMarket::invoke_service(
                RuntimeOrigin::signed(BOB),
                0,
                b"req".to_vec(),
                Some(vec![
                    MilestoneSpec { pct_of_total: 60 },
                    MilestoneSpec { pct_of_total: 30 }, // sums to 90, not 100
                ]),
                100,
                100,
            ),
            Error::<Test>::MilestonePercentagesInvalid
        );
    });
}

#[test]
fn cancel_invocation_refunds_escrow() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        let bob_before = Balances::free_balance(BOB);
        assert_ok!(ServiceMarket::cancel_invocation(
            RuntimeOrigin::signed(BOB),
            0
        ));
        let bob_after = Balances::free_balance(BOB);

        assert_eq!(bob_after - bob_before, 100); // refunded

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::Cancelled);
    });
}

#[test]
fn cancel_invocation_fails_not_invoker() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_noop!(
            ServiceMarket::cancel_invocation(RuntimeOrigin::signed(CHARLIE), 0),
            Error::<Test>::NotInvoker
        );
    });
}

#[test]
fn cancel_invocation_fails_not_pending() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        // Submit work → status = WorkSubmitted
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            None,
            b"proof".to_vec(),
            ProofType::Hash,
        ));

        assert_noop!(
            ServiceMarket::cancel_invocation(RuntimeOrigin::signed(BOB), 0),
            Error::<Test>::CannotCancelActiveInvocation
        );
    });
}

// =========================================================
// Expiry tests
// =========================================================

#[test]
fn try_expire_invocation_pays_bounty_and_refunds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));

        // Invoke with deadline_blocks = 10
        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"req".to_vec(),
            None,
            100,
            10, // deadline_blocks
        ));

        // Advance past deadline
        System::set_block_number(50);

        let charlie_before = Balances::free_balance(CHARLIE);
        let bob_before = Balances::free_balance(BOB);

        assert_ok!(ServiceMarket::try_expire_invocation(
            RuntimeOrigin::signed(CHARLIE),
            0
        ));

        let charlie_after = Balances::free_balance(CHARLIE);
        let bob_after = Balances::free_balance(BOB);

        // Charlie got bounty
        assert_eq!(charlie_after - charlie_before, 10); // ExpireBounty = 10
                                                        // Bob got refund minus bounty
        assert_eq!(bob_after - bob_before, 90);

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::Expired);
    });
}

#[test]
fn try_expire_invocation_fails_deadline_not_passed() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"req".to_vec(),
            None,
            100,
            100, // deadline_blocks = 100
        ));

        // Current block = 1, deadline = 101 → not expired
        assert_noop!(
            ServiceMarket::try_expire_invocation(RuntimeOrigin::signed(CHARLIE), 0),
            Error::<Test>::DeadlineNotPassed
        );
    });
}

#[test]
fn on_initialize_expires_overdue_invocations() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::invoke_service(
            RuntimeOrigin::signed(BOB),
            0,
            b"req".to_vec(),
            None,
            100,
            5, // deadline = block 6
        ));

        // Advance to block 20
        System::set_block_number(20);
        <ServiceMarket as Hooks<u64>>::on_initialize(20u64);

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::Expired);
    });
}

// =========================================================
// Dispute tests
// =========================================================

#[test]
fn raise_dispute_succeeds() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_ok!(ServiceMarket::raise_dispute(
            RuntimeOrigin::signed(BOB),
            0,
            b"Provider did not deliver".to_vec(),
            None,
        ));

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::Disputed);

        let dispute = Disputes::<Test>::get(0).unwrap();
        assert_eq!(dispute.raised_by, BOB);
        assert_eq!(dispute.invocation_id, 0);
    });
}

#[test]
fn raise_dispute_fails_not_party() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_noop!(
            ServiceMarket::raise_dispute(
                RuntimeOrigin::signed(CHARLIE), // third party
                0,
                b"reason".to_vec(),
                None,
            ),
            Error::<Test>::NotPartyToInvocation
        );
    });
}

#[test]
fn raise_dispute_fails_already_disputed() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));
        assert_ok!(ServiceMarket::raise_dispute(
            RuntimeOrigin::signed(BOB),
            0,
            b"reason".to_vec(),
            None,
        ));

        assert_noop!(
            ServiceMarket::raise_dispute(
                RuntimeOrigin::signed(ALICE),
                0,
                b"counter".to_vec(),
                None,
            ),
            Error::<Test>::InvocationAlreadyDisputed
        );
    });
}

#[test]
fn provider_can_raise_dispute() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));

        assert_ok!(ServiceMarket::raise_dispute(
            RuntimeOrigin::signed(ALICE), // provider raises dispute
            0,
            b"Invoker not cooperating".to_vec(),
            None,
        ));

        let inv = ServiceInvocations::<Test>::get(0).unwrap();
        assert_eq!(inv.status, InvocationStatus::Disputed);
    });
}

#[test]
fn resolve_dispute_governance_transfers_to_winner() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0)); // BOB locked 100 in escrow
        assert_ok!(ServiceMarket::raise_dispute(
            RuntimeOrigin::signed(BOB),
            0,
            b"reason".to_vec(),
            None,
        ));

        let alice_before = Balances::free_balance(ALICE);

        assert_ok!(ServiceMarket::resolve_dispute_governance(
            RuntimeOrigin::root(),
            0,
            ALICE, // provider wins
        ));

        let alice_after = Balances::free_balance(ALICE);
        // Alice received the escrow (minus existential deposit remainder)
        assert!(alice_after > alice_before);

        let dispute = Disputes::<Test>::get(0).unwrap();
        assert_eq!(dispute.status, DisputeStatus::Resolved);
        assert_eq!(dispute.winner, Some(ALICE));
    });
}

#[test]
fn resolve_dispute_governance_fails_not_root() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));
        assert_ok!(ServiceMarket::raise_dispute(
            RuntimeOrigin::signed(BOB),
            0,
            b"reason".to_vec(),
            None,
        ));

        assert_noop!(
            ServiceMarket::resolve_dispute_governance(RuntimeOrigin::signed(CHARLIE), 0, ALICE),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn dispute_not_found_returns_error() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ServiceMarket::resolve_dispute_governance(RuntimeOrigin::root(), 999, ALICE),
            Error::<Test>::DisputeNotFound
        );
    });
}

// =========================================================
// Edge case tests
// =========================================================

#[test]
fn multiple_listings_multiple_invocations() {
    new_test_ext().execute_with(|| {
        // ALICE lists 2 services
        assert_ok!(list_service_default(ALICE));
        assert_ok!(ServiceMarket::list_service(
            RuntimeOrigin::signed(ALICE),
            b"Storage Service".to_vec(),
            b"Distributed storage".to_vec(),
            vec![b"infra/storage".to_vec()],
            200,
            200,
            PaymentMode::Escrow,
            5,
            20,
            0,
            None,
            false,
        ));

        // BOB and CHARLIE both invoke listing 0
        assert_ok!(invoke_service_default(BOB, 0));
        assert_ok!(invoke_service_default(CHARLIE, 0));

        assert_eq!(InvocationCount::<Test>::get(), 2);

        // Listing 0 total invocations = 2
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert_eq!(listing.total_invocations, 2);
    });
}

#[test]
fn listing_not_found_on_invoke() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            invoke_service_default(BOB, 42),
            Error::<Test>::ListingNotFound
        );
    });
}

#[test]
fn escrow_account_derivation_is_deterministic() {
    new_test_ext().execute_with(|| {
        // Calling twice with the same ID returns the same account.
        let escrow0a = ServiceMarket::invocation_escrow_account(0);
        let escrow0b = ServiceMarket::invocation_escrow_account(0);
        assert_eq!(escrow0a, escrow0b);
        // (Uniqueness across IDs is guaranteed in production with 32-byte AccountId;
        //  with u64 AccountId in tests the truncation collapses them — that is expected.)
    });
}

#[test]
fn submit_work_with_cid_proof_type() {
    new_test_ext().execute_with(|| {
        assert_ok!(list_service_default(ALICE));
        assert_ok!(invoke_service_default(BOB, 0));
        assert_ok!(ServiceMarket::submit_invocation_work(
            RuntimeOrigin::signed(ALICE),
            0,
            None,
            b"QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG".to_vec(),
            ProofType::Cid,
        ));
        let proof = InvocationProofs::<Test>::get(0, u32::MAX).unwrap();
        assert_eq!(proof.proof_type, ProofType::Cid);
    });
}

#[test]
fn invoke_service_with_x402_payment_mode_listed() {
    new_test_ext().execute_with(|| {
        // List a service with X402 payment mode
        assert_ok!(ServiceMarket::list_service(
            RuntimeOrigin::signed(ALICE),
            b"API Service".to_vec(),
            b"desc".to_vec(),
            vec![],
            100,
            100,
            PaymentMode::X402,
            10,
            50,
            0,
            None,
            false,
        ));
        let listing = ServiceListings::<Test>::get(0).unwrap();
        assert!(matches!(listing.payment_mode, PaymentMode::X402));
    });
}

#[test]
fn listing_count_starts_at_zero() {
    new_test_ext().execute_with(|| {
        assert_eq!(ListingCount::<Test>::get(), 0);
        assert_eq!(InvocationCount::<Test>::get(), 0);
        assert_eq!(DisputeCount::<Test>::get(), 0);
    });
}
