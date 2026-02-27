// Auto-generated weights placeholder for pallet-anon-messaging.
// Replace with actual benchmarked values before production deployment.

use frame_support::weights::Weight;

/// Weight functions for `pallet_anon_messaging`.
pub trait WeightInfo {
    fn register_public_key() -> Weight;
    fn send_message() -> Weight;
    fn read_message() -> Weight;
    fn delete_message() -> Weight;
    fn set_auto_response() -> Weight;
    fn claim_reply_escrow() -> Weight;
    fn on_initialize(n: u32) -> Weight;
}

/// Placeholder weights — all operations cost a flat 10_000 ref_time.
pub struct SubstrateWeight;

impl WeightInfo for SubstrateWeight {
    fn register_public_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn send_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn read_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn delete_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_auto_response() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn claim_reply_escrow() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn on_initialize(_n: u32) -> Weight {
        Weight::from_parts(10_000, 0)
    }
}

impl WeightInfo for () {
    fn register_public_key() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn send_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn read_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn delete_message() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn set_auto_response() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn claim_reply_escrow() -> Weight {
        Weight::from_parts(10_000, 0)
    }
    fn on_initialize(_n: u32) -> Weight {
        Weight::from_parts(10_000, 0)
    }
}
