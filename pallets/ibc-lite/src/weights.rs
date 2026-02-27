//! IBC-lite weight stubs.
//!
//! These are placeholder weights. Real weights should be determined through
//! benchmarking using the `runtime-benchmarks` feature.

use frame_support::weights::Weight;

// =========================================================
// Weight Info Trait
// =========================================================

pub trait WeightInfo {
    // Channel management
    fn open_channel() -> Weight;
    fn close_channel_init() -> Weight;
    fn close_channel_confirm() -> Weight;

    // Packet operations
    fn send_packet() -> Weight;
    fn receive_packet() -> Weight;
    fn acknowledge_packet() -> Weight;
    fn timeout_packet() -> Weight;

    // Relayer management
    fn add_relayer() -> Weight;
    fn remove_relayer() -> Weight;

    // Cross-chain agents
    fn register_cross_chain_agent() -> Weight;
}

// =========================================================
// Default Stub Implementation
// =========================================================

// We use `()` as a default implementation for WeightInfo.
// This is just a placeholder and should be replaced with real benchmarked weights.
impl WeightInfo for () {
    // Channel management
    fn open_channel() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn close_channel_init() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn close_channel_confirm() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    // Packet operations
    fn send_packet() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn receive_packet() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn acknowledge_packet() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn timeout_packet() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    // Relayer management
    fn add_relayer() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn remove_relayer() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    // Cross-chain agents
    fn register_cross_chain_agent() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}
