//! # pallet-gas-quota
//!
//! Implements ADR-002: Stake-based free transaction quota for ClawChain agents.
//!
//! ## Overview
//!
//! Agents receive a daily free transaction quota proportional to their staked $CLAW:
//!
//! | Stake        | Free TX/day | Excess fee       |
//! |-------------|-------------|-----------------|
//! | 0 $CLAW     | 10          | 0.001 $CLAW/tx  |
//! | 100 $CLAW   | 100         | 0.0005 $CLAW/tx |
//! | 1,000 $CLAW | 1,000       | 0.0001 $CLAW/tx |
//! | 10,000+     | Unlimited   | Free            |
//!
//! Reputation multipliers: High rep → 1.5×, Verified contributor → 2×
//!
//! ## Rationale
//!
//! Pure zero-gas is trivially spammable. This approach gives agents 0-gas UX
//! in practice while making spam economically costly. See ADR-002 for full rationale.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, Get, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{CheckedAdd, Saturating, Zero},
        Perbill,
    };

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    // =========================================================================
    // Configuration Trait
    // =========================================================================

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency used for staking and fees.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// Number of blocks per day (used for quota reset).
        /// At 6s block time: 14,400 blocks/day.
        #[pallet::constant]
        type BlocksPerDay: Get<BlockNumberFor<Self>>;

        /// Minimum free TX quota regardless of stake (10 tx/day).
        #[pallet::constant]
        type MinFreeQuota: Get<u32>;

        /// Stake amount (in planck units) per 1 free TX above the minimum.
        /// e.g. 100 $CLAW staked → 100 free tx/day → rate = 1 $CLAW per tx.
        #[pallet::constant]
        type StakePerFreeTx: Get<BalanceOf<Self>>;

        /// Stake threshold for unlimited free transactions.
        #[pallet::constant]
        type UnlimitedStakeThreshold: Get<BalanceOf<Self>>;

        /// Base fee per transaction when over quota (in planck units).
        #[pallet::constant]
        type BaseFeePerTx: Get<BalanceOf<Self>>;

        /// Fee discount applied to base fee as stake increases (per tx above quota).
        /// Higher stake → lower excess fee.
        #[pallet::constant]
        type FeeDiscountPerKStake: Get<Perbill>;
    }

    // =========================================================================
    // Storage
    // =========================================================================

    /// Per-agent quota tracking.
    #[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
    pub struct AgentQuota<Balance, BlockNumber> {
        /// Current staked balance (snapshot, updated on stake changes).
        pub stake: Balance,
        /// Number of free TXs used in the current day.
        pub daily_used: u32,
        /// Block number when the current day started (for reset tracking).
        pub day_start_block: BlockNumber,
        /// Reputation tier: 0=normal, 1=high, 2=verified_contributor.
        pub reputation_tier: u8,
    }

    #[pallet::storage]
    #[pallet::getter(fn agent_quota)]
    pub type AgentQuotas<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        AgentQuota<BalanceOf<T>, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Total excess fees collected (goes to treasury).
    #[pallet::storage]
    #[pallet::getter(fn total_fees_collected)]
    pub type TotalFeesCollected<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // =========================================================================
    // Events
    // =========================================================================

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An agent's daily quota was reset. [agent, new_free_quota]
        QuotaReset {
            agent: T::AccountId,
            free_quota: u32,
        },
        /// A transaction used quota. [agent, remaining_quota]
        QuotaUsed {
            agent: T::AccountId,
            remaining: u32,
        },
        /// An over-quota fee was charged. [agent, fee_amount]
        FeeCharged {
            agent: T::AccountId,
            amount: BalanceOf<T>,
        },
        /// An agent's stake was updated. [agent, new_stake]
        StakeUpdated {
            agent: T::AccountId,
            stake: BalanceOf<T>,
        },
        /// Reputation tier updated. [agent, tier]
        ReputationTierUpdated {
            agent: T::AccountId,
            tier: u8,
        },
    }

    // =========================================================================
    // Errors
    // =========================================================================

    #[pallet::error]
    pub enum Error<T> {
        /// Agent has insufficient balance to pay excess fee.
        InsufficientBalance,
        /// Quota record not found (not yet initialized).
        QuotaNotInitialized,
    }

    // =========================================================================
    // Pallet Struct & Hooks
    // =========================================================================

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // =========================================================================
    // Extrinsics
    // =========================================================================

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Initialize or update an agent's quota record.
        /// Called on first TX or when stake changes.
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn initialize_quota(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_quota_initialized(&who);
            Ok(())
        }

        /// Update stake snapshot for an agent (called by staking hooks).
        /// Only callable by the agent themselves or root.
        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_stake(
            origin: OriginFor<T>,
            agent: T::AccountId,
            new_stake: BalanceOf<T>,
        ) -> DispatchResult {
            // Allow agent to update own stake, or root for any
            let caller = ensure_signed(origin)?;
            ensure!(caller == agent, frame_support::error::BadOrigin);

            AgentQuotas::<T>::mutate(&agent, |maybe_quota| {
                if let Some(quota) = maybe_quota {
                    quota.stake = new_stake;
                }
            });

            Self::deposit_event(Event::StakeUpdated {
                agent,
                stake: new_stake,
            });
            Ok(())
        }

        /// Update reputation tier for an agent (called by reputation pallet).
        #[pallet::call_index(2)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_reputation_tier(
            origin: OriginFor<T>,
            agent: T::AccountId,
            tier: u8,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(tier <= 2, Error::<T>::QuotaNotInitialized); // reuse error for now

            AgentQuotas::<T>::mutate(&agent, |maybe_quota| {
                if let Some(quota) = maybe_quota {
                    quota.reputation_tier = tier;
                }
            });

            Self::deposit_event(Event::ReputationTierUpdated { agent, tier });
            Ok(())
        }
    }

    // =========================================================================
    // Public API (called by SignedExtension / other pallets)
    // =========================================================================

    impl<T: Config> Pallet<T> {
        /// Calculate the free daily quota for a given stake and reputation tier.
        pub fn calculate_free_quota(stake: BalanceOf<T>, reputation_tier: u8) -> u32 {
            // Unlimited above threshold
            if stake >= T::UnlimitedStakeThreshold::get() {
                return u32::MAX;
            }

            // Base quota: stake / StakePerFreeTx, minimum MinFreeQuota
            let stake_per_tx = T::StakePerFreeTx::get();
            let base_quota = if stake_per_tx.is_zero() {
                T::MinFreeQuota::get()
            } else {
                let stake_quota = (stake / stake_per_tx)
                    .try_into()
                    .unwrap_or(u32::MAX);
                stake_quota.max(T::MinFreeQuota::get())
            };

            // Apply reputation multiplier
            match reputation_tier {
                0 => base_quota,                               // 1× (normal)
                1 => base_quota.saturating_mul(3) / 2,        // 1.5× (high rep)
                2 => base_quota.saturating_mul(2),            // 2× (verified contributor)
                _ => base_quota,
            }
        }

        /// Check and consume one transaction from an agent's daily quota.
        /// Returns Ok(()) if within quota (free TX), or charges a fee if over quota.
        /// Returns Err if fee payment fails.
        pub fn consume_quota(who: &T::AccountId) -> DispatchResult {
            let current_block = <frame_system::Pallet<T>>::block_number();
            let blocks_per_day = T::BlocksPerDay::get();

            Self::ensure_quota_initialized(who);

            AgentQuotas::<T>::try_mutate(who, |maybe_quota| -> DispatchResult {
                let quota = maybe_quota.as_mut().ok_or(Error::<T>::QuotaNotInitialized)?;

                // Reset daily counter if a new day has started
                let blocks_since_day_start = current_block.saturating_sub(quota.day_start_block);
                if blocks_since_day_start >= blocks_per_day {
                    let free_quota = Self::calculate_free_quota(quota.stake, quota.reputation_tier);
                    quota.daily_used = 0;
                    quota.day_start_block = current_block;
                    Self::deposit_event(Event::QuotaReset {
                        agent: who.clone(),
                        free_quota,
                    });
                }

                let free_quota = Self::calculate_free_quota(quota.stake, quota.reputation_tier);

                if free_quota == u32::MAX || quota.daily_used < free_quota {
                    // Within free quota
                    quota.daily_used = quota.daily_used.saturating_add(1);
                    let remaining = if free_quota == u32::MAX {
                        u32::MAX
                    } else {
                        free_quota.saturating_sub(quota.daily_used)
                    };
                    Self::deposit_event(Event::QuotaUsed {
                        agent: who.clone(),
                        remaining,
                    });
                } else {
                    // Over quota — charge fee
                    let fee = Self::calculate_excess_fee(quota.stake);
                    T::Currency::withdraw(
                        who,
                        fee,
                        frame_support::traits::WithdrawReasons::FEE,
                        frame_support::traits::ExistenceRequirement::KeepAlive,
                    )
                    .map_err(|_| Error::<T>::InsufficientBalance)?;

                    TotalFeesCollected::<T>::mutate(|total| {
                        *total = total.saturating_add(fee);
                    });

                    quota.daily_used = quota.daily_used.saturating_add(1);
                    Self::deposit_event(Event::FeeCharged {
                        agent: who.clone(),
                        amount: fee,
                    });
                }

                Ok(())
            })
        }

        /// Calculate excess fee based on stake level.
        /// Higher stake → lower per-tx fee over quota.
        fn calculate_excess_fee(stake: BalanceOf<T>) -> BalanceOf<T> {
            let base_fee = T::BaseFeePerTx::get();
            // Simple: reduce fee by 50% for every 10× stake above MinFreeQuota threshold
            // Full formula in ADR-002; simplified here for v1
            let discount = T::FeeDiscountPerKStake::get();
            let stake_k = stake / (T::StakePerFreeTx::get().saturating_mul(1000u32.into()));
            let total_discount = discount.saturating_pow(
                stake_k.try_into().unwrap_or(0u32),
            );
            total_discount.mul_floor(base_fee).max(base_fee / 10u32.into()) // floor at 10% of base
        }

        /// Ensure an agent has a quota record, initializing if missing.
        fn ensure_quota_initialized(who: &T::AccountId) {
            if !AgentQuotas::<T>::contains_key(who) {
                let stake = T::Currency::reserved_balance(who);
                let current_block = <frame_system::Pallet<T>>::block_number();
                AgentQuotas::<T>::insert(
                    who,
                    AgentQuota {
                        stake,
                        daily_used: 0,
                        day_start_block: current_block,
                        reputation_tier: 0,
                    },
                );
            }
        }
    }
}
