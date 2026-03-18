//! Shared types for pallet-reputation-regime.

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;

/// The three market regimes based on Fear & Greed index.
///
/// The regime is derived from the raw F&G value:
/// - `value < FearThreshold` → `Fear`
/// - `value > GreedThreshold` → `Greed`
/// - everything in between → `Neutral`
#[derive(
    Clone,
    Copy,
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
pub enum Regime {
    /// F&G < 25: Fear regime — agents who perform well are rewarded 2x.
    Fear,
    /// F&G 25–75: Neutral regime — baseline 1x multiplier.
    #[default]
    Neutral,
    /// F&G > 75: Greed regime — reduced 0.5x multiplier.
    Greed,
}

/// Categories of reputation-affecting actions.
///
/// Different action types can theoretically have different multiplier
/// profiles per regime, but in v1 all actions share the same multiplier.
/// This enum exists for forward compatibility.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ActionType {
    /// Agent completed an assigned task successfully.
    TaskCompletion,
    /// Agent maintained uptime / heartbeat.
    Uptime,
    /// Agent provided an accurate result / attestation.
    Accuracy,
    /// Agent received a peer review.
    PeerReview,
    /// Catch-all for future action types.
    Other,
}

/// Snapshot of a regime change for history tracking.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegimeChange<BlockNumber> {
    /// The F&G value that triggered this change.
    pub fear_greed_value: u8,
    /// The resulting regime.
    pub regime: Regime,
    /// The multiplier in basis points (e.g. 200 = 2x, 100 = 1x, 50 = 0.5x).
    pub multiplier_bps: u32,
    /// Block number when this regime was set.
    pub changed_at: BlockNumber,
}
