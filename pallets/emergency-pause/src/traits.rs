//! Traits for the Emergency Pause pallet.
//!
//! These traits allow other pallets to integrate with the emergency pause
//! circuit-breaker without creating tight coupling.

extern crate alloc;
use alloc::vec::Vec;

/// Trait for querying whether a pallet is currently paused.
///
/// Other pallets should implement a guard in their extrinsics by calling
/// `EmergencyPauseProvider::is_paused(b"pallet-name")` and returning an error
/// if the pallet is paused.
pub trait EmergencyPauseProvider {
    /// Returns `true` if the pallet with the given ID is currently paused.
    ///
    /// # Arguments
    /// * `pallet_id` - The ASCII identifier of the pallet (e.g. `b"pallet-task-market"`).
    fn is_paused(pallet_id: &[u8]) -> bool;

    /// Returns the list of all currently paused pallet IDs.
    fn paused_pallets() -> Vec<Vec<u8>>;
}

/// No-op implementation — used in test environments where the pause pallet
/// is not wired in.
impl EmergencyPauseProvider for () {
    fn is_paused(_pallet_id: &[u8]) -> bool {
        false
    }

    fn paused_pallets() -> Vec<Vec<u8>> {
        Vec::new()
    }
}

/// Trait for recording audit events related to the emergency pause lifecycle.
///
/// Implement this trait to plug in an off-chain indexing or on-chain audit log.
/// The default no-op implementation discards all events.
pub trait AuditTrailProvider {
    /// Called when a pallet is paused.
    fn on_paused(pallet_id: &[u8], triggered_by: &[u8], block: u64);
    /// Called when a pallet is unpaused.
    fn on_unpaused(pallet_id: &[u8], triggered_by: &[u8], block: u64);
    /// Called when an emergency pause is activated.
    fn on_emergency_pause(triggered_by: &[u8], block: u64);
}

/// No-op audit trail — used when no external log is needed.
impl AuditTrailProvider for () {
    fn on_paused(_pallet_id: &[u8], _triggered_by: &[u8], _block: u64) {}
    fn on_unpaused(_pallet_id: &[u8], _triggered_by: &[u8], _block: u64) {}
    fn on_emergency_pause(_triggered_by: &[u8], _block: u64) {}
}
