//! IBC-lite traits and interfaces.

use frame_support::pallet_prelude::*;

// =========================================================
// Agent Registry Interface
// =========================================================

/// Interface to agent-registry for cross-chain agent identity validation.
pub trait AgentRegistryInterface<AccountId> {
    /// Check if an agent exists.
    fn agent_exists(agent_id: u64) -> bool;

    /// Get the owner of an agent.
    fn agent_owner(agent_id: u64) -> Option<AccountId>;

    /// Check if an agent is active.
    fn is_agent_active(agent_id: u64) -> bool;
}

// =========================================================
// Mock Implementation for Testing
// =========================================================

#[cfg(test)]
mod mock {
    use super::*;
    use sp_runtime::RuntimeDebug;

    /// Mock agent registry for testing.
    #[derive(RuntimeDebug)]
    pub struct MockAgentRegistry;

    impl AgentRegistryInterface<u64> for MockAgentRegistry {
        fn agent_exists(agent_id: u64) -> bool {
            // Simple mock: agents 1-10 exist
            agent_id > 0 && agent_id <= 10
        }

        fn agent_owner(agent_id: u64) -> Option<u64> {
            if Self::agent_exists(agent_id) {
                Some(agent_id) // Owner ID = agent ID for simplicity
            } else {
                None
            }
        }

        fn is_agent_active(agent_id: u64) -> bool {
            Self::agent_exists(agent_id)
        }
    }
}
