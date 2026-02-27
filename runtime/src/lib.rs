//! ClawChain Runtime
//!
//! The runtime is the state transition function of the ClawChain blockchain.
//! It compiles to WASM and defines the business logic for all on-chain operations.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::{vec, vec::Vec};
// codec and scale_info used by FRAME macros
use frame_election_provider_support::{
    bounds::ElectionBoundsBuilder, onchain, SequentialPhragmen, VoteWeight,
};
use frame_support::{
    derive_impl,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{tokens::PayFromAccount, ConstBool, ConstU128, ConstU32, ConstU64, ConstU8},
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        IdentityFee, Weight,
    },
    PalletId,
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_transaction_payment::FungibleAdapter;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
    create_runtime_str,
    curve::PiecewiseLinear,
    generic, impl_opaque_keys,
    traits::{
        AccountIdConversion, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, OpaqueKeys,
        Verify,
    },
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature, Permill,
};
use sp_staking::SessionIndex;
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
use sp_version::NativeVersion;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through://upgrades.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

/// The version information used to identify this runtime when compiled natively.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("clawchain"),
    impl_name: create_runtime_str!("clawchain-node"),
    authoring_version: 1,
    spec_version: 200,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const SLOT_DURATION: u64 = 6000; // 6 seconds

/// Number of blocks produced per minute.
pub const MINUTES: BlockNumber = 60_000 / (SLOT_DURATION as BlockNumber);
/// Number of blocks produced per hour.
pub const HOURS: BlockNumber = MINUTES * 60;
/// Number of blocks produced per day.
pub const DAYS: BlockNumber = HOURS * 24;

/// The existential deposit. Set to 1/10th of a CLAW.
pub const EXISTENTIAL_DEPOSIT: u128 = 100_000_000_000; // 0.1 CLAW

/// CLAW token unit (1 CLAW = 1e12 base units)
pub const UNITS: Balance = 1_000_000_000_000;

// Staking constants
/// Session length: 100 blocks = 10 minutes at 6s block time
pub const SESSION_LENGTH: BlockNumber = 100;
/// Era length: 6 sessions = 1 hour (for testnet)
pub const SESSIONS_PER_ERA: SessionIndex = 6;
/// Minimum validator bond: 10,000 CLAW
pub const MIN_VALIDATOR_BOND: Balance = 10_000 * UNITS;
/// Minimum nominator bond: 100 CLAW
pub const MIN_NOMINATOR_BOND: Balance = 100 * UNITS;

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: sp_runtime::Perbill = sp_runtime::Perbill::from_percent(10);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by Operational extrinsics.
const NORMAL_DISPATCH_RATIO: sp_runtime::Perbill = sp_runtime::Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;

    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(frame_support::dispatch::DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(frame_support::dispatch::DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(frame_support::dispatch::DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
    /// The block body type.
    type Block = Block;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<32>;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = ConstU32<32>;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
    type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (Staking, ());
}

parameter_types! {
    pub const Period: u32 = SESSION_LENGTH;
    pub const Offset: u32 = 0;
}

pub struct ValidatorIdOf;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
    fn convert(a: AccountId) -> Option<AccountId> {
        Some(a)
    }
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ValidatorIdOf;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = opaque::SessionKeys;
    type WeightInfo = ();
    type Currency = Balances;
    type KeyDeposit = ConstU128<UNITS>;
    type DisablingStrategy = ();
}

impl pallet_session::historical::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::DefaultExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const SessionsPerEra: SessionIndex = SESSIONS_PER_ERA;
    pub const BondingDuration: sp_staking::EraIndex = 7; // 7 eras = 7 hours on testnet
    pub const SlashDeferDuration: sp_staking::EraIndex = 2; // 2 eras
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
    pub const MaxExposurePageSize: u32 = 64;
    pub const MaxControllersInDeprecationBatch: u32 = 100;
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
    type System = Runtime;
    type Solver = SequentialPhragmen<AccountId, sp_runtime::Perbill>;
    type DataProvider = Staking;
    type WeightInfo = ();
    type Bounds = ElectionBounds;
    type Sort = ();
    type MaxBackersPerWinner = ConstU32<50>;
    type MaxWinnersPerPage = ConstU32<100>;
}

parameter_types! {
    pub ElectionBounds: frame_election_provider_support::bounds::ElectionBounds =
        ElectionBoundsBuilder::default()
            .voters_count(10_000.into())
            .targets_count(1_000.into())
            .build();
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxValidators = ConstU32<100>;
    type MaxNominators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
    type RewardRemainder = ();
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    type AdminOrigin = frame_system::EnsureRoot<AccountId>;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type HistoryDepth = ConstU32<84>;
    type MaxExposurePageSize = MaxExposurePageSize;
    type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type VoterList = BagsList;
    type TargetList = pallet_staking::UseValidatorsMap<Runtime>;
    type MaxUnlockingChunks = ConstU32<32>;
    type EventListeners = ();
    type WeightInfo = ();
    type BenchmarkingConfig = StakingBenchmarkingConfig;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
    type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
    type OldCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxValidatorSet = ConstU32<100>;
    type Filter = ();
}

parameter_types! {
    pub const BagThresholds: &'static [u64] = &[
        100_000_000_000_000,
        200_000_000_000_000,
        500_000_000_000_000,
        1_000_000_000_000_000,
        2_000_000_000_000_000,
        5_000_000_000_000_000,
        10_000_000_000_000_000,
    ];
}

impl pallet_bags_list::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ScoreProvider = Staking;
    type WeightInfo = ();
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
    type MaxAutoRebagPerBlock = ConstU32<100>;
}

impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 100 * UNITS;
    pub const ProposalBondMaximum: Balance = 500 * UNITS;
    pub const SpendPeriod: BlockNumber = 6 * DAYS;
    pub const Burn: Permill = Permill::from_percent(1);
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const MaxApprovals: u32 = 100;
    pub TreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

pub struct TreasuryAccountGetter;
impl frame_support::traits::Get<AccountId> for TreasuryAccountGetter {
    fn get() -> AccountId {
        TreasuryPalletId::get().into_account_truncating()
    }
}

impl frame_support::traits::TypedGet for TreasuryAccountGetter {
    type Type = AccountId;
    fn get() -> Self::Type {
        TreasuryPalletId::get().into_account_truncating()
    }
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = ();
    type MaxApprovals = MaxApprovals;
    type WeightInfo = ();
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
    type AssetKind = ();
    type Beneficiary = AccountId;
    type BeneficiaryLookup = sp_runtime::traits::IdentityLookup<AccountId>;
    type Paymaster = PayFromAccount<Balances, TreasuryAccountGetter>;
    type BalanceConverter = frame_support::traits::tokens::UnityAssetBalanceConversion;
    type PayoutPeriod = ConstU32<0>;
    type BlockNumberProvider = System;
    type RejectOrigin = frame_system::EnsureRoot<AccountId>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

/// Configure the agent registry pallet.
impl pallet_agent_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxDidLength = ConstU32<256>;
    type MaxMetadataLength = ConstU32<4096>;
    type MaxAgentsPerOwner = ConstU32<100>;
    type ReputationOracle = ReputationOracleAccount;
}

/// No reputation oracle configured — falls back to root-only reputation updates.
pub struct ReputationOracleAccount;
impl frame_support::traits::Get<Option<AccountId>> for ReputationOracleAccount {
    fn get() -> Option<AccountId> {
        None
    }
}

/// Configure the CLAW token pallet.
impl pallet_claw_token::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type AirdropPool = ConstU128<{ 400_000_000 * 1_000_000_000_000u128 }>; // 40% of 1B CLAW
    type MaxContributionScore = ConstU64<{ u64::MAX }>;
}

parameter_types! {
    // Reputation parameters
    pub const MaxCommentLength: u32 = 256;
    pub const InitialReputation: u32 = 5000;
    pub const MaxReputationDelta: u32 = 500;
    pub const MaxHistoryLength: u32 = 100;

    // Task Market parameters
    pub const TaskMarketPalletId: PalletId = PalletId(*b"taskmark");
    pub const MaxTitleLength: u32 = 128;
    pub const MaxDescriptionLength: u32 = 1024;
    pub const MaxProposalLength: u32 = 512;
    pub const MaxBidsPerTask: u32 = 20;
    pub const MinTaskReward: Balance = 100 * UNITS; // 100 CLAW minimum
    pub const MaxActiveTasksPerAccount: u32 = 50;
}

impl pallet_reputation::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type MaxCommentLength = MaxCommentLength;
    type InitialReputation = InitialReputation;
    type MaxReputationDelta = MaxReputationDelta;
    type MaxHistoryLength = MaxHistoryLength;
}

impl pallet_task_market::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type ReputationManager = Reputation;
    type PalletId = TaskMarketPalletId;
    type MaxTitleLength = MaxTitleLength;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MaxProposalLength = MaxProposalLength;
    type MaxBidsPerTask = MaxBidsPerTask;
    type MinTaskReward = MinTaskReward;
    type MaxActiveTasksPerAccount = MaxActiveTasksPerAccount;
}

/// Configure the RPC registry pallet.
impl pallet_rpc_registry::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxUrlLength = ConstU32<256>;
    type MaxRegionLength = ConstU32<32>;
    type MaxNodesPerOwner = ConstU32<10>;
    type MaxActiveNodes = ConstU32<1000>;
    type MaxHeartbeatInterval = ConstU32<300>; // 300 blocks = ~30 min at 6s/block
}
// Create the runtime by composing the FRAME pallets that were previously configured.
parameter_types! {
    pub const GasQuotaBlocksPerDay: u32 = 14_400; // 6s blocks × 14400 = 24h
    pub const GasQuotaStakePerFreeTx: u128 = 1_000_000_000_000; // 1 CLAW
    pub const GasQuotaUnlimitedThreshold: u128 = 10_000_000_000_000_000; // 10,000 CLAW
    pub const GasQuotaBaseFee: u128 = 1_000_000_000; // 0.001 CLAW
    pub const GasQuotaFeeDiscount: sp_runtime::Perbill = sp_runtime::Perbill::from_percent(90);
    pub const GasQuotaMinFree: u32 = 10;
}

impl pallet_gas_quota::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlocksPerDay = GasQuotaBlocksPerDay;
    type MinFreeQuota = GasQuotaMinFree;
    type StakePerFreeTx = GasQuotaStakePerFreeTx;
    type UnlimitedStakeThreshold = GasQuotaUnlimitedThreshold;
    type BaseFeePerTx = GasQuotaBaseFee;
    type FeeDiscountPerKStake = GasQuotaFeeDiscount;
}
// Configure the quadratic governance pallet.
parameter_types! {
    pub const GovMinProposalDeposit: Balance = 100 * UNITS;      // 100 CLAW
    pub const GovVotingPeriod: BlockNumber = 50_400;             // ~7 days at 6s/block
    pub const GovMinQuorumPct: u32 = 10;                         // require >= 10 total vote-weight
}

/// Configure the Quadratic Governance pallet (ADR-004).
impl pallet_quadratic_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MinProposalDeposit = GovMinProposalDeposit;
    type VotingPeriod = GovVotingPeriod;
    type MinQuorumPct = GovMinQuorumPct;
    type WeightInfo = ();
}

impl pallet_agent_did::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    // DID document context field
    type MaxContextLength = ConstU32<512>;
    // Service endpoint field bounds
    type MaxServiceIdLength = ConstU32<128>;
    type MaxServiceTypeLength = ConstU32<128>;
    type MaxEndpointLength = ConstU32<512>;
    type MaxServiceEndpoints = ConstU32<10>;
    // Verification method field bounds
    type MaxKeyIdLength = ConstU32<128>;
    type MaxKeyTypeLength = ConstU32<128>;
    type MaxKeyLength = ConstU32<256>;
    type MaxVerificationMethods = ConstU32<5>;
}

/// Configure the Agent Receipts pallet (ProvenanceChain — verifiable agent activity attestation).
impl pallet_agent_receipts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAgentIdLen = ConstU32<64>;
    type MaxActionTypeLen = ConstU32<64>;
    type MaxMetadataLen = ConstU32<512>;
    type MaxClearBatchSize = ConstU32<1000>;
}

// =========================================================
// IBC-lite Configuration
// =========================================================

/// Agent registry wrapper for IBC-lite.
pub struct IbcAgentRegistry;

impl pallet_ibc_lite::traits::AgentRegistryInterface<AccountId> for IbcAgentRegistry {
    fn agent_exists(agent_id: u64) -> bool {
        pallet_agent_registry::AgentRegistry::<Runtime>::contains_key(agent_id)
    }

    fn agent_owner(agent_id: u64) -> Option<AccountId> {
        pallet_agent_registry::AgentRegistry::<Runtime>::get(agent_id).map(|info| info.owner)
    }

    fn is_agent_active(agent_id: u64) -> bool {
        pallet_agent_registry::AgentRegistry::<Runtime>::get(agent_id)
            .map(|info| info.status == pallet_agent_registry::AgentStatus::Active)
            .unwrap_or(false)
    }
}

/// Configure the IBC-lite pallet.
impl pallet_ibc_lite::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type RelayerManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxRelayers = ConstU32<10>;
    type MaxChannelsPerChain = ConstU32<100>;
    type MaxChannelIdLen = ConstU32<128>;
    type MaxChainIdLen = ConstU32<128>;
    type MaxPayloadLen = ConstU32<4096>;
    type MaxPendingPackets = ConstU32<1000>;
    type PacketTimeoutBlocks = ConstU32<100>;
    type AgentRegistry = IbcAgentRegistry;
}

frame_support::construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,

        // Staking & Governance
        Authorship: pallet_authorship,
        Session: pallet_session,
        Historical: pallet_session::historical,
        Staking: pallet_staking,
        Offences: pallet_offences,
        BagsList: pallet_bags_list,
        Treasury: pallet_treasury,
        Sudo: pallet_sudo,

        // ClawChain custom pallets
        AgentRegistry: pallet_agent_registry,
        ClawToken: pallet_claw_token,
        Reputation: pallet_reputation,
        TaskMarket: pallet_task_market,
        RpcRegistry: pallet_rpc_registry,
        GasQuota: pallet_gas_quota,
        AgentDid: pallet_agent_did,
        QuadraticGovernance: pallet_quadratic_governance,
        AgentReceipts: pallet_agent_receipts,
        IbcLite: pallet_ibc_lite,
    }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// All migrations of the runtime, in order.
/// Add new migrations here.
type Migrations = ();

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,
>;

/// The native version of the runtime.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: <Block as BlockT>::LazyBlock) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: <Block as BlockT>::LazyBlock,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }
}
