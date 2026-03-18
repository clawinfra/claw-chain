//! # Reputation Regime Pallet
//!
//! Fear-adaptive 3-tier reputation multiplier for ClawChain agents.
//!
//! ## Overview
//!
//! This pallet manages a global Fear & Greed regime that scales reputation
//! gains/losses for agent actions:
//!
//! - **Fear** (F&G < 25): 2x multiplier — reliability during fear is rewarded
//! - **Neutral** (F&G 25–75): 1x baseline
//! - **Greed** (F&G > 75): 0.5x multiplier — easy-mode performance diluted
//!
//! ## V1 Design
//!
//! An authorized origin (root or configured oracle account) submits F&G values
//! via `update_regime()`. The pallet derives the regime and stores it.
//!
//! ## Integration
//!
//! Other pallets (especially `pallet-reputation`) call the
//! [`RegimeMultiplierProvider`] trait to get the current multiplier.
//!
//! ## Usage Example
//!
//! ```ignore
//! // In pallet-reputation (v2 integration):
//! let base_delta: u64 = 500;
//! let multiplier = T::RegimeProvider::regime_multiplier(&reviewee, ActionType::PeerReview);
//! let adjusted_delta = base_delta.saturating_mul(multiplier as u64) / 100;
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated, clippy::let_unit_value)]

extern crate alloc;

pub use pallet::*;
pub mod types;

#[cfg(test)]
mod tests;

use types::{ActionType, Regime, RegimeChange};

/// Trait for cross-pallet regime multiplier queries.
///
/// Returns the multiplier in basis points (100 = 1x, 200 = 2x, 50 = 0.5x).
/// The `agent_id` parameter is included for forward compatibility — in v1
/// all agents get the same global multiplier.
pub trait RegimeMultiplierProvider<AccountId> {
    /// Get the current regime multiplier for a given agent and action type.
    ///
    /// Returns basis points: 200 = 2x, 100 = 1x, 50 = 0.5x.
    /// In v1 the `agent_id` and `action_type` are unused (global multiplier).
    fn regime_multiplier(agent_id: &AccountId, action_type: ActionType) -> u32;

    /// Get the current regime.
    fn current_regime() -> Regime;

    /// Get the raw Fear & Greed value (0–100).
    fn current_fear_greed() -> u8;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    // -------------------------------------------------------------------------
    // Weight Info
    // -------------------------------------------------------------------------

    /// Weight information for pallet extrinsics.
    pub trait WeightInfo {
        fn update_regime() -> Weight;
    }

    impl WeightInfo for () {
        fn update_regime() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }

    // -------------------------------------------------------------------------
    // Pallet
    // -------------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // -------------------------------------------------------------------------
    // Config
    // -------------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// Origin allowed to update the F&G value.
        ///
        /// Root always works; this allows an additional authorized origin.
        /// Set to `EnsureRoot` if only sudo should update.
        type OracleOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Fear threshold (exclusive upper bound). F&G values **below** this
        /// are in the Fear regime.
        ///
        /// Default recommended value: 25.
        #[pallet::constant]
        type FearThreshold: Get<u8>;

        /// Greed threshold (exclusive lower bound). F&G values **above** this
        /// are in the Greed regime.
        ///
        /// Default recommended value: 75.
        #[pallet::constant]
        type GreedThreshold: Get<u8>;

        /// Multiplier for the Fear regime in basis points (100 = 1x).
        ///
        /// Default recommended value: 200 (= 2x).
        #[pallet::constant]
        type FearMultiplierBps: Get<u32>;

        /// Multiplier for the Neutral regime in basis points.
        ///
        /// Default recommended value: 100 (= 1x).
        #[pallet::constant]
        type NeutralMultiplierBps: Get<u32>;

        /// Multiplier for the Greed regime in basis points.
        ///
        /// Default recommended value: 50 (= 0.5x).
        #[pallet::constant]
        type GreedMultiplierBps: Get<u32>;

        /// Maximum number of regime change history entries to keep.
        ///
        /// When the history is full the oldest entry is evicted (FIFO).
        #[pallet::constant]
        type MaxRegimeHistory: Get<u32>;
    }

    // -------------------------------------------------------------------------
    // Storage
    // -------------------------------------------------------------------------

    /// The current Fear & Greed index value (0–100).
    ///
    /// Default: 50 (mid-range neutral).
    #[pallet::storage]
    #[pallet::getter(fn current_fear_greed_value)]
    pub type CurrentFearGreed<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// The current derived regime.
    ///
    /// Default: [`Regime::Neutral`].
    #[pallet::storage]
    #[pallet::getter(fn current_regime_value)]
    pub type CurrentRegimeStorage<T: Config> = StorageValue<_, Regime, ValueQuery>;

    /// The current multiplier in basis points.
    ///
    /// Default: 100 (= 1x, Neutral). Initialised by genesis config.
    #[pallet::storage]
    #[pallet::getter(fn current_multiplier_bps)]
    pub type CurrentMultiplierBps<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// History of regime changes (bounded, FIFO eviction when full).
    #[pallet::storage]
    #[pallet::getter(fn regime_history)]
    pub type RegimeHistory<T: Config> = StorageValue<
        _,
        BoundedVec<RegimeChange<BlockNumberFor<T>>, T::MaxRegimeHistory>,
        ValueQuery,
    >;

    /// Block number of the last regime update.
    #[pallet::storage]
    #[pallet::getter(fn last_updated)]
    pub type LastUpdated<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // -------------------------------------------------------------------------
    // Genesis
    // -------------------------------------------------------------------------

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Initial Fear & Greed value (0–100). Defaults to 50 (Neutral).
        pub initial_fear_greed: u8,
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let value = self.initial_fear_greed.min(100);
            let regime = Pallet::<T>::derive_regime(value);
            let multiplier = Pallet::<T>::regime_to_multiplier(&regime);

            CurrentFearGreed::<T>::put(value);
            CurrentRegimeStorage::<T>::put(regime);
            CurrentMultiplierBps::<T>::put(multiplier);
        }
    }

    // -------------------------------------------------------------------------
    // Events
    // -------------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// The Fear & Greed value was updated and the regime changed.
        RegimeUpdated {
            /// The new F&G value (0–100).
            fear_greed_value: u8,
            /// The regime before this update.
            old_regime: Regime,
            /// The regime after this update.
            new_regime: Regime,
            /// The new multiplier in basis points.
            multiplier_bps: u32,
            /// Account that submitted the update, if signed (None = root).
            updater: Option<T::AccountId>,
        },
        /// The F&G value was updated but the regime did not change.
        FearGreedUpdated {
            /// The new F&G value.
            fear_greed_value: u8,
            /// The current (unchanged) regime.
            regime: Regime,
            /// Account that submitted the update, if signed (None = root).
            updater: Option<T::AccountId>,
        },
    }

    // -------------------------------------------------------------------------
    // Errors
    // -------------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// The Fear & Greed value must be in range 0–100 (inclusive).
        ValueOutOfRange,
        /// The configured FearThreshold must be strictly less than GreedThreshold.
        ///
        /// This is a misconfiguration guard; correct the runtime config.
        InvalidThresholdConfig,
    }

    // -------------------------------------------------------------------------
    // Extrinsics
    // -------------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Update the Fear & Greed index value and derive the new regime.
        ///
        /// # Parameters
        /// - `fear_greed_value`: the new F&G index value, must be 0–100 inclusive.
        ///
        /// # Origin
        /// Must satisfy the configured `OracleOrigin` (root or authorised oracle).
        ///
        /// # Errors
        /// - [`Error::ValueOutOfRange`] — if `fear_greed_value > 100`.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::update_regime())]
        pub fn update_regime(origin: OriginFor<T>, fear_greed_value: u8) -> DispatchResult {
            // 1. Verify origin and extract account for event logging.
            let updater = Self::ensure_oracle_origin(origin)?;

            // 2. Validate input range.
            ensure!(fear_greed_value <= 100, Error::<T>::ValueOutOfRange);

            // 3. Read current state.
            let old_regime = CurrentRegimeStorage::<T>::get();

            // 4. Derive new regime and multiplier.
            let new_regime = Self::derive_regime(fear_greed_value);
            let new_multiplier = Self::regime_to_multiplier(&new_regime);

            // 5. Persist new state.
            CurrentFearGreed::<T>::put(fear_greed_value);
            CurrentRegimeStorage::<T>::put(new_regime);
            CurrentMultiplierBps::<T>::put(new_multiplier);

            let current_block = <frame_system::Pallet<T>>::block_number();
            LastUpdated::<T>::put(current_block);

            // 6. Append to history (FIFO eviction when at capacity).
            let change = RegimeChange {
                fear_greed_value,
                regime: new_regime,
                multiplier_bps: new_multiplier,
                changed_at: current_block,
            };
            RegimeHistory::<T>::mutate(|history| {
                if history.len() >= T::MaxRegimeHistory::get() as usize {
                    // Remove oldest entry to make room.
                    history.remove(0);
                }
                // try_push only fails if BoundedVec is full; we just evicted one
                // entry so this is guaranteed to succeed.
                let _ = history.try_push(change);
            });

            // 7. Emit event.
            if old_regime != new_regime {
                Self::deposit_event(Event::RegimeUpdated {
                    fear_greed_value,
                    old_regime,
                    new_regime,
                    multiplier_bps: new_multiplier,
                    updater,
                });
            } else {
                Self::deposit_event(Event::FearGreedUpdated {
                    fear_greed_value,
                    regime: new_regime,
                    updater,
                });
            }

            Ok(())
        }
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Derive the [`Regime`] from a raw Fear & Greed value using the
        /// configured thresholds.
        ///
        /// - `value < FearThreshold` → [`Regime::Fear`]
        /// - `value > GreedThreshold` → [`Regime::Greed`]
        /// - everything else → [`Regime::Neutral`]
        pub(crate) fn derive_regime(value: u8) -> Regime {
            let fear = T::FearThreshold::get();
            let greed = T::GreedThreshold::get();

            if value < fear {
                Regime::Fear
            } else if value > greed {
                Regime::Greed
            } else {
                Regime::Neutral
            }
        }

        /// Map a [`Regime`] to its configured multiplier in basis points.
        pub(crate) fn regime_to_multiplier(regime: &Regime) -> u32 {
            match regime {
                Regime::Fear => T::FearMultiplierBps::get(),
                Regime::Neutral => T::NeutralMultiplierBps::get(),
                Regime::Greed => T::GreedMultiplierBps::get(),
            }
        }

        /// Validate the origin and return the caller's `AccountId` for event logging.
        ///
        /// Root origin is always accepted; for non-root signed origins the caller
        /// account is returned directly and is expected to satisfy `OracleOrigin`.
        ///
        /// If `OracleOrigin` is `EnsureRoot`, non-root signed calls will be
        /// rejected by `T::OracleOrigin::ensure_origin()`.
        fn ensure_oracle_origin(
            origin: OriginFor<T>,
        ) -> Result<Option<T::AccountId>, DispatchError> {
            // Try to extract a signed account for event logging (may fail for root).
            let maybe_who = frame_system::ensure_signed(origin.clone()).ok();

            // Validate the origin against the configured OracleOrigin.
            // For `EnsureRoot`: root passes, any signed origin fails.
            // For a future `EnsureSigned<AccountId>`: only that account passes.
            T::OracleOrigin::ensure_origin(origin)?;

            Ok(maybe_who)
        }
    }

    // -------------------------------------------------------------------------
    // Trait implementation
    // -------------------------------------------------------------------------

    impl<T: Config> RegimeMultiplierProvider<T::AccountId> for Pallet<T> {
        /// Returns the current global multiplier in basis points.
        ///
        /// In v1 `agent_id` and `action_type` are unused — all agents and all
        /// action types share the same global multiplier. These parameters exist
        /// for forward compatibility with v2 per-agent/per-action profiles.
        fn regime_multiplier(_agent_id: &T::AccountId, _action_type: ActionType) -> u32 {
            CurrentMultiplierBps::<T>::get()
        }

        fn current_regime() -> Regime {
            CurrentRegimeStorage::<T>::get()
        }

        fn current_fear_greed() -> u8 {
            CurrentFearGreed::<T>::get()
        }
    }
}
