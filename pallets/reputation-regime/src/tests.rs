//! Unit tests for the pallet-reputation-regime pallet.
//!
//! Coverage target: ≥ 90% on lib.rs and types.rs.

use crate::{self as pallet_reputation_regime, types::*, *};
use frame_support::{assert_noop, assert_ok, parameter_types};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

// ---------------------------------------------------------------------------
// Mock runtime
// ---------------------------------------------------------------------------

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        ReputationRegime: pallet_reputation_regime,
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
    type AccountData = ();
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
    pub const FearThreshold: u8 = 25;
    pub const GreedThreshold: u8 = 75;
    pub const FearMultiplierBps: u32 = 200;
    pub const NeutralMultiplierBps: u32 = 100;
    pub const GreedMultiplierBps: u32 = 50;
    pub const MaxRegimeHistory: u32 = 10; // small for testing FIFO eviction
}

impl pallet_reputation_regime::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type OracleOrigin = frame_system::EnsureRoot<u64>;
    type FearThreshold = FearThreshold;
    type GreedThreshold = GreedThreshold;
    type FearMultiplierBps = FearMultiplierBps;
    type NeutralMultiplierBps = NeutralMultiplierBps;
    type GreedMultiplierBps = GreedMultiplierBps;
    type MaxRegimeHistory = MaxRegimeHistory;
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a fresh test externality environment.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let config = frame_system::GenesisConfig::<Test>::default();
    let mut t = config.build_storage().unwrap();

    // Initialise the pallet genesis with default F&G = 50 (Neutral).
    pallet_reputation_regime::GenesisConfig::<Test> {
        initial_fear_greed: 50,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Convenience: call `update_regime` with root origin.
fn set_fear_greed(value: u8) -> frame_support::dispatch::DispatchResult {
    ReputationRegime::update_regime(RuntimeOrigin::root(), value)
}

// ===========================================================================
// 1. Initial State Tests
// ===========================================================================

#[test]
fn initial_state_is_neutral() {
    new_test_ext().execute_with(|| {
        assert_eq!(ReputationRegime::current_fear_greed_value(), 50);
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
        assert_eq!(ReputationRegime::regime_history().len(), 0);
    });
}

#[test]
fn initial_regime_getter_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
    });
}

#[test]
fn initial_multiplier_is_100() {
    new_test_ext().execute_with(|| {
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

// ===========================================================================
// 2. Regime Derivation Tests (Core Logic)
// ===========================================================================

#[test]
fn fear_regime_at_zero() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(0));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
    });
}

#[test]
fn fear_regime_at_24() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(24));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
    });
}

#[test]
fn neutral_regime_at_25() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(25));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

#[test]
fn neutral_regime_at_50() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

#[test]
fn neutral_regime_at_75() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(75));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

#[test]
fn greed_regime_at_76() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(76));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

#[test]
fn greed_regime_at_100() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(100));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

// ===========================================================================
// 3. Fixture Data Integration Tests — [16, 15, 23]
// ===========================================================================

#[test]
fn fixture_sequence_16_15_23() {
    new_test_ext().execute_with(|| {
        // All three values are < 25 → Fear regime throughout.
        // (Initial regime is Neutral from genesis at 50.)

        // Value 16 → transitions Neutral→Fear.
        assert_ok!(set_fear_greed(16));
        assert_eq!(ReputationRegime::current_fear_greed_value(), 16);
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
        assert_eq!(ReputationRegime::regime_history().len(), 1);
        assert_eq!(ReputationRegime::regime_history()[0].fear_greed_value, 16);
        assert_eq!(ReputationRegime::regime_history()[0].regime, Regime::Fear);
        assert_eq!(ReputationRegime::regime_history()[0].multiplier_bps, 200);

        // Value 15 → same regime, no transition → FearGreedUpdated event.
        assert_ok!(set_fear_greed(15));
        assert_eq!(ReputationRegime::current_fear_greed_value(), 15);
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
        assert_eq!(ReputationRegime::regime_history().len(), 2);

        // Value 23 → still Fear regime.
        assert_ok!(set_fear_greed(23));
        assert_eq!(ReputationRegime::current_fear_greed_value(), 23);
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
        assert_eq!(ReputationRegime::regime_history().len(), 3);
    });
}

#[test]
fn fixture_16_then_50_then_23() {
    new_test_ext().execute_with(|| {
        // 16 → Fear, 50 → Neutral, 23 → Fear (two transitions)
        assert_ok!(set_fear_greed(16));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);

        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);

        assert_ok!(set_fear_greed(23));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);

        assert_eq!(ReputationRegime::regime_history().len(), 3);
    });
}

#[test]
fn fixture_values_with_multiplier_check() {
    new_test_ext().execute_with(|| {
        let agent: u64 = 42;

        for &val in &[16u8, 15u8, 23u8] {
            assert_ok!(set_fear_greed(val));
            assert_eq!(
                <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
                    &agent,
                    ActionType::Uptime
                ),
                200,
                "Expected Fear multiplier 200 for F&G={val}"
            );
            assert_eq!(
                <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
                    &agent,
                    ActionType::Accuracy
                ),
                200,
                "Same multiplier regardless of action type in v1, F&G={val}"
            );
        }
    });
}

// ===========================================================================
// 4. Extrinsic Tests
// ===========================================================================

#[test]
fn update_regime_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ReputationRegime::update_regime(RuntimeOrigin::root(), 10));
        assert_eq!(ReputationRegime::current_fear_greed_value(), 10);
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
    });
}

#[test]
fn update_regime_emits_regime_updated_event() {
    new_test_ext().execute_with(|| {
        // Transition from Neutral (genesis) to Fear.
        assert_ok!(ReputationRegime::update_regime(RuntimeOrigin::root(), 10));

        let events = System::events();
        let has_event = events.iter().any(|e| {
            matches!(
                &e.event,
                RuntimeEvent::ReputationRegime(Event::RegimeUpdated {
                    fear_greed_value: 10,
                    old_regime: Regime::Neutral,
                    new_regime: Regime::Fear,
                    multiplier_bps: 200,
                    ..
                })
            )
        });
        assert!(has_event, "Expected RegimeUpdated event");
    });
}

#[test]
fn update_regime_emits_fear_greed_updated_event() {
    new_test_ext().execute_with(|| {
        // Both updates stay within Neutral regime (50 → 60).
        assert_ok!(ReputationRegime::update_regime(RuntimeOrigin::root(), 60));

        let events = System::events();
        let has_event = events.iter().any(|e| {
            matches!(
                &e.event,
                RuntimeEvent::ReputationRegime(Event::FearGreedUpdated {
                    fear_greed_value: 60,
                    regime: Regime::Neutral,
                    ..
                })
            )
        });
        assert!(
            has_event,
            "Expected FearGreedUpdated event (no regime change)"
        );
    });
}

#[test]
fn update_regime_value_out_of_range() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ReputationRegime::update_regime(RuntimeOrigin::root(), 101),
            Error::<Test>::ValueOutOfRange
        );
    });
}

#[test]
fn update_regime_value_255_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ReputationRegime::update_regime(RuntimeOrigin::root(), 255),
            Error::<Test>::ValueOutOfRange
        );
    });
}

#[test]
fn update_regime_unsigned_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ReputationRegime::update_regime(RuntimeOrigin::none(), 50),
            sp_runtime::traits::BadOrigin
        );
    });
}

#[test]
fn update_regime_signed_non_root_fails() {
    new_test_ext().execute_with(|| {
        // OracleOrigin = EnsureRoot, so any signed non-root origin must fail.
        assert_noop!(
            ReputationRegime::update_regime(RuntimeOrigin::signed(1), 50),
            sp_runtime::traits::BadOrigin
        );
    });
}

#[test]
fn update_regime_value_zero_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ReputationRegime::update_regime(RuntimeOrigin::root(), 0));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
    });
}

#[test]
fn update_regime_value_100_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(ReputationRegime::update_regime(RuntimeOrigin::root(), 100));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

// ===========================================================================
// 5. Regime Transition Tests
// ===========================================================================

#[test]
fn transition_neutral_to_fear() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(10));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);

        let events = System::events();
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::ReputationRegime(Event::RegimeUpdated {
                old_regime: Regime::Neutral,
                new_regime: Regime::Fear,
                ..
            })
        )));
    });
}

#[test]
fn transition_neutral_to_greed() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(80));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

#[test]
fn transition_fear_to_neutral() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(10));
        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

#[test]
fn transition_fear_to_greed() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(10));
        assert_ok!(set_fear_greed(90));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

#[test]
fn transition_greed_to_neutral() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(80));
        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);
    });
}

#[test]
fn transition_greed_to_fear() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(80));
        assert_ok!(set_fear_greed(5));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);
    });
}

#[test]
fn no_transition_same_regime() {
    new_test_ext().execute_with(|| {
        // 10 → Fear, 20 → still Fear → FearGreedUpdated (not RegimeUpdated).
        assert_ok!(set_fear_greed(10));
        System::reset_events();

        assert_ok!(set_fear_greed(20));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);

        let events = System::events();
        let has_fear_greed_updated = events.iter().any(|e| {
            matches!(
                &e.event,
                RuntimeEvent::ReputationRegime(Event::FearGreedUpdated {
                    fear_greed_value: 20,
                    regime: Regime::Fear,
                    ..
                })
            )
        });
        assert!(
            has_fear_greed_updated,
            "Expected FearGreedUpdated not RegimeUpdated"
        );

        // Must NOT have a RegimeUpdated event.
        let has_regime_updated = events.iter().any(|e| {
            matches!(
                &e.event,
                RuntimeEvent::ReputationRegime(Event::RegimeUpdated { .. })
            )
        });
        assert!(
            !has_regime_updated,
            "Must not emit RegimeUpdated when regime is unchanged"
        );
    });
}

// ===========================================================================
// 6. Boundary Tests
// ===========================================================================

#[test]
fn boundary_24_is_fear() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(24));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
    });
}

#[test]
fn boundary_25_is_neutral() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(25));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
    });
}

#[test]
fn boundary_75_is_neutral() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(75));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Neutral);
    });
}

#[test]
fn boundary_76_is_greed() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(76));
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Greed);
    });
}

// ===========================================================================
// 7. History Tests
// ===========================================================================

#[test]
fn history_records_updates() {
    new_test_ext().execute_with(|| {
        System::set_block_number(5);
        assert_ok!(set_fear_greed(10));

        System::set_block_number(10);
        assert_ok!(set_fear_greed(50));

        System::set_block_number(15);
        assert_ok!(set_fear_greed(80));

        let history = ReputationRegime::regime_history();
        assert_eq!(history.len(), 3);

        assert_eq!(history[0].fear_greed_value, 10);
        assert_eq!(history[0].changed_at, 5);
        assert_eq!(history[0].regime, Regime::Fear);

        assert_eq!(history[1].fear_greed_value, 50);
        assert_eq!(history[1].changed_at, 10);
        assert_eq!(history[1].regime, Regime::Neutral);

        assert_eq!(history[2].fear_greed_value, 80);
        assert_eq!(history[2].changed_at, 15);
        assert_eq!(history[2].regime, Regime::Greed);
    });
}

#[test]
fn history_fifo_at_capacity() {
    new_test_ext().execute_with(|| {
        // MaxRegimeHistory = 10 in test config.
        // Push 11 entries. Oldest (first) should be evicted.
        for i in 0u8..11 {
            System::set_block_number((i + 1) as u64);
            // Alternate values to keep updates going through.
            assert_ok!(set_fear_greed(if i % 2 == 0 { 10 } else { 20 }));
        }

        let history = ReputationRegime::regime_history();
        assert_eq!(
            history.len(),
            10,
            "History should be capped at MaxRegimeHistory=10"
        );

        // The first entry pushed (block 1, value=10) should be evicted.
        // The oldest remaining entry should be block 2.
        assert_eq!(
            history[0].changed_at, 2,
            "Oldest entry should have been evicted (FIFO)"
        );
        // The newest entry is at block 11.
        assert_eq!(history[9].changed_at, 11);
    });
}

#[test]
fn history_entries_have_correct_data() {
    new_test_ext().execute_with(|| {
        System::set_block_number(42);
        assert_ok!(set_fear_greed(5));

        let history = ReputationRegime::regime_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].fear_greed_value, 5);
        assert_eq!(history[0].regime, Regime::Fear);
        assert_eq!(history[0].multiplier_bps, 200);
        assert_eq!(history[0].changed_at, 42);
    });
}

// ===========================================================================
// 8. RegimeMultiplierProvider Trait Tests
// ===========================================================================

#[test]
fn trait_regime_multiplier_returns_current() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(5));
        let multiplier = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &1u64,
            ActionType::TaskCompletion,
        );
        assert_eq!(multiplier, 200);
    });
}

#[test]
fn trait_regime_multiplier_ignores_agent_id() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(80));
        let m1 = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &1u64,
            ActionType::Uptime,
        );
        let m2 = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &999u64,
            ActionType::Uptime,
        );
        assert_eq!(m1, m2, "All agents get the same multiplier in v1");
        assert_eq!(m1, 50); // Greed
    });
}

#[test]
fn trait_regime_multiplier_ignores_action_type() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(10)); // Fear
        let agent = 1u64;

        let m_task = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &agent,
            ActionType::TaskCompletion,
        );
        let m_uptime = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &agent,
            ActionType::Uptime,
        );
        let m_accuracy = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &agent,
            ActionType::Accuracy,
        );
        let m_peer = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &agent,
            ActionType::PeerReview,
        );
        let m_other = <ReputationRegime as RegimeMultiplierProvider<u64>>::regime_multiplier(
            &agent,
            ActionType::Other,
        );

        assert_eq!(m_task, 200);
        assert_eq!(m_uptime, 200);
        assert_eq!(m_accuracy, 200);
        assert_eq!(m_peer, 200);
        assert_eq!(m_other, 200);
    });
}

#[test]
fn trait_current_regime_returns_correct() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(5));
        assert_eq!(
            <ReputationRegime as RegimeMultiplierProvider<u64>>::current_regime(),
            Regime::Fear
        );
    });
}

#[test]
fn trait_current_fear_greed_returns_value() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(42));
        assert_eq!(
            <ReputationRegime as RegimeMultiplierProvider<u64>>::current_fear_greed(),
            42
        );
    });
}

// ===========================================================================
// 9. LastUpdated Tests
// ===========================================================================

#[test]
fn last_updated_changes_on_update() {
    new_test_ext().execute_with(|| {
        System::set_block_number(10);
        assert_ok!(set_fear_greed(10));
        assert_eq!(ReputationRegime::last_updated(), 10);

        System::set_block_number(20);
        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::last_updated(), 20);
    });
}

// ===========================================================================
// 10. Additional edge case tests
// ===========================================================================

#[test]
fn same_value_submitted_twice_records_both_in_history() {
    new_test_ext().execute_with(|| {
        assert_ok!(set_fear_greed(10));
        assert_ok!(set_fear_greed(10)); // Same value again

        // Both are recorded in history (each call is an update).
        let history = ReputationRegime::regime_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].fear_greed_value, 10);
        assert_eq!(history[1].fear_greed_value, 10);

        // Regime unchanged — FearGreedUpdated event for second call.
        assert_eq!(ReputationRegime::current_regime_value(), Regime::Fear);
    });
}

#[test]
fn all_three_regimes_multipliers_are_correct() {
    new_test_ext().execute_with(|| {
        // Fear
        assert_ok!(set_fear_greed(10));
        assert_eq!(ReputationRegime::current_multiplier_bps(), 200);

        // Neutral
        assert_ok!(set_fear_greed(50));
        assert_eq!(ReputationRegime::current_multiplier_bps(), 100);

        // Greed
        assert_ok!(set_fear_greed(90));
        assert_eq!(ReputationRegime::current_multiplier_bps(), 50);
    });
}

#[test]
fn regime_history_entry_fear_greed_value_matches_storage() {
    new_test_ext().execute_with(|| {
        for v in [0u8, 24, 25, 50, 75, 76, 100] {
            assert_ok!(set_fear_greed(v));
        }
        let history = ReputationRegime::regime_history();
        assert_eq!(history.len(), 7);
        let values: Vec<u8> = history.iter().map(|e| e.fear_greed_value).collect();
        assert_eq!(values, vec![0, 24, 25, 50, 75, 76, 100]);
    });
}

#[test]
fn multiple_regime_transitions_emit_correct_events() {
    new_test_ext().execute_with(|| {
        // Neutral → Fear
        assert_ok!(set_fear_greed(10));
        // Fear → Greed (direct, skipping Neutral)
        assert_ok!(set_fear_greed(90));

        let events = System::events();
        let regime_updated_count = events
            .iter()
            .filter(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::ReputationRegime(Event::RegimeUpdated { .. })
                )
            })
            .count();

        assert_eq!(
            regime_updated_count, 2,
            "Expected exactly 2 RegimeUpdated events"
        );
    });
}
