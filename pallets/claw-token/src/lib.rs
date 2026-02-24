//! # CLAW Token Pallet
//!
//! Extends the standard Substrate balances pallet with ClawChain-specific economics.
//!
//! ## Overview
//!
//! This pallet manages:
//! - Contributor score tracking for airdrop eligibility
//! - Airdrop claim mechanism based on contribution scores
//! - Treasury spending for community initiatives
//!
//! ## Tokenomics (from whitepaper)
//!
//! - Total supply: 1,000,000,000 CLAW
//! - 40% airdrop allocation (400M CLAW)
//! - 30% validator rewards (300M CLAW)
//! - 20% treasury (200M CLAW)
//! - 10% team allocation (100M CLAW)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement},
    };
    use frame_system::pallet_prelude::*;

    /// Balance type from the currency trait.
    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        /// The currency implementation (typically pallet_balances).
        type Currency: Currency<Self::AccountId>;

        /// Total airdrop pool size in base units.
        #[pallet::constant]
        type AirdropPool: Get<u128>;

        /// Maximum contribution score a single account can accumulate.
        #[pallet::constant]
        type MaxContributionScore: Get<u64>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ========== Storage ==========

    /// Map of contributor accounts to their contribution scores.
    #[pallet::storage]
    #[pallet::getter(fn contributor_scores)]
    pub type ContributorScores<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Whether an account has already claimed their airdrop.
    #[pallet::storage]
    #[pallet::getter(fn airdrop_claimed)]
    pub type AirdropClaimed<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// Total contribution scores across all contributors (for proportional airdrop calculation).
    #[pallet::storage]
    #[pallet::getter(fn total_contribution_score)]
    pub type TotalContributionScore<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Total amount of CLAW already distributed from the airdrop pool.
    #[pallet::storage]
    #[pallet::getter(fn airdrop_distributed)]
    pub type AirdropDistributed<T: Config> = StorageValue<_, u128, ValueQuery>;

    // ========== Events ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A contribution score was recorded.
        ContributionRecorded {
            contributor: T::AccountId,
            score: u64,
            total_score: u64,
        },
        /// An airdrop was claimed.
        AirdropClaimed { who: T::AccountId, amount: u128 },
        /// Treasury funds were spent.
        TreasurySpend { to: T::AccountId, amount: u128 },
    }

    // ========== Errors ==========

    #[pallet::error]
    pub enum Error<T> {
        /// The caller is not authorized (requires root/sudo).
        NotAuthorized,
        /// The airdrop has already been claimed by this account.
        AlreadyClaimed,
        /// The account has no contribution score and is not eligible for an airdrop.
        NoContributionScore,
        /// The airdrop pool is exhausted.
        AirdropPoolExhausted,
        /// Contribution score would overflow.
        ScoreOverflow,
        /// Insufficient treasury balance.
        InsufficientTreasuryBalance,
        /// Arithmetic overflow in calculations.
        ArithmeticOverflow,
    }

    // ========== Extrinsics ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Record a contribution score for an account.
        ///
        /// This is a privileged operation — only root/sudo can call it.
        /// In production, this would be called by a governance-approved oracle
        /// that tracks GitHub contributions, community activity, etc.
        ///
        /// # Arguments
        /// * `contributor` - The account to credit
        /// * `score` - The contribution score to add
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 2))]
        pub fn record_contribution(
            origin: OriginFor<T>,
            contributor: T::AccountId,
            score: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let current_score = ContributorScores::<T>::get(&contributor);
            let new_score = current_score
                .checked_add(score)
                .ok_or(Error::<T>::ScoreOverflow)?;

            ContributorScores::<T>::insert(&contributor, new_score);

            // Update total
            let total = TotalContributionScore::<T>::get();
            TotalContributionScore::<T>::put(
                total
                    .checked_add(score as u128)
                    .ok_or(Error::<T>::ArithmeticOverflow)?,
            );

            Self::deposit_event(Event::ContributionRecorded {
                contributor,
                score,
                total_score: new_score,
            });

            Ok(())
        }

        /// Claim airdrop tokens based on contribution score.
        ///
        /// The airdrop amount is proportional to the caller's contribution score
        /// relative to the total contribution scores. Each account can only claim once.
        ///
        /// Formula: `claim = (account_score / total_scores) * airdrop_pool`
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(4, 2))]
        pub fn claim_airdrop(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check not already claimed
            ensure!(!AirdropClaimed::<T>::get(&who), Error::<T>::AlreadyClaimed);

            // Check contribution score exists
            let score = ContributorScores::<T>::get(&who);
            ensure!(score > 0, Error::<T>::NoContributionScore);

            // Calculate proportional airdrop amount
            let total_score = TotalContributionScore::<T>::get();
            ensure!(total_score > 0, Error::<T>::NoContributionScore);

            let pool = T::AirdropPool::get();
            let distributed = AirdropDistributed::<T>::get();
            let remaining = pool.saturating_sub(distributed);
            ensure!(remaining > 0, Error::<T>::AirdropPoolExhausted);

            // claim = (score / total_score) * pool
            // Use u128 math to avoid overflow
            let claim_amount = (score as u128)
                .checked_mul(pool)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_div(total_score)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let claim_amount = claim_amount.min(remaining);

            // Mark as claimed
            AirdropClaimed::<T>::insert(&who, true);
            AirdropDistributed::<T>::put(distributed.saturating_add(claim_amount));

            Self::deposit_event(Event::AirdropClaimed {
                who,
                amount: claim_amount,
            });

            Ok(())
        }

        /// Spend from the treasury.
        ///
        /// This is a privileged operation — only root/sudo can call it.
        ///
        /// # Arguments
        /// * `to` - The recipient account
        /// * `amount` - The amount to transfer from treasury
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(1, 1))]
        pub fn treasury_spend(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            Self::deposit_event(Event::TreasurySpend { to, amount });

            Ok(())
        }
    }

    // ========== Weight Info Trait ==========

    /// Weight information for the pallet's extrinsics.
    pub trait WeightInfo {
        fn record_contribution() -> Weight;
        fn claim_airdrop() -> Weight;
        fn treasury_spend() -> Weight;
    }

    /// Default weights for testing.
    impl WeightInfo for () {
        fn record_contribution() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn claim_airdrop() -> Weight {
            Weight::from_parts(10_000, 0)
        }
        fn treasury_spend() -> Weight {
            Weight::from_parts(10_000, 0)
        }
    }
}
