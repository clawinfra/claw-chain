/**
 * @clawinfra/clawchain-sdk — Type definitions
 *
 * TypeScript types mirroring the on-chain Rust structs for
 * ClawChain's agent-registry and task-market pallets.
 */

// ──────────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────────

/** SDK configuration */
export interface ClawChainConfig {
  /** WebSocket endpoint, e.g. "wss://testnet.clawchain.win" */
  wsUrl: string;
  /** Optional HTTP RPC endpoint for read-only calls */
  httpUrl?: string;
  /** Connection timeout in milliseconds (default: 30_000) */
  timeoutMs?: number;
}

// ──────────────────────────────────────────────────────────────────────────────
// Agent Registry
// ──────────────────────────────────────────────────────────────────────────────

/** On-chain agent status (mirrors AgentStatus enum in pallet) */
export type AgentStatus = "Active" | "Suspended" | "Deregistered";

/**
 * On-chain agent info (mirrors AgentInfo struct in pallet-agent-registry).
 *
 * ```rust
 * pub struct AgentInfo<AccountId, BlockNumber> {
 *   pub owner: AccountId,
 *   pub did: BoundedVec<u8, 128>,
 *   pub metadata: BoundedVec<u8, 1024>,
 *   pub reputation: u32,
 *   pub registered_at: BlockNumber,
 *   pub last_active: BlockNumber,
 *   pub status: AgentStatus,
 * }
 * ```
 */
export interface AgentInfo {
  /** Agent ID (u32 on-chain) */
  agentId: number;
  /** SS58 address of the account that owns this agent */
  owner: string;
  /** Decentralized Identifier string (UTF-8 encoded on-chain) */
  did: string;
  /** JSON metadata: name, type, capabilities, etc. */
  metadata: Record<string, unknown>;
  /** Reputation score 0–10,000 (basis points) */
  reputation: number;
  /** Block number when agent was registered */
  registeredAt: number;
  /** Block number of last activity */
  lastActive: number;
  /** Current operational status */
  status: AgentStatus;
}

/** Arguments for registering a new agent */
export interface RegisterAgentArgs {
  /** Decentralized Identifier, e.g. "did:claw:agent:abc123" */
  did: string;
  /** Arbitrary metadata (serialised to JSON on-chain, max 1 KB) */
  metadata: Record<string, unknown>;
}

// ──────────────────────────────────────────────────────────────────────────────
// Task Market
// ──────────────────────────────────────────────────────────────────────────────

/** Task lifecycle status */
export type TaskStatus =
  | "Open"       // Posted, accepting bids
  | "Assigned"   // Bid accepted, in progress
  | "Submitted"  // Result submitted, pending review
  | "Completed"  // Approved and paid
  | "Disputed"   // Under dispute resolution
  | "Cancelled"; // Cancelled before completion

/**
 * On-chain task info (mirrors TaskInfo in pallet-task-market).
 */
export interface TaskInfo {
  /** Task ID (u32 on-chain) */
  taskId: number;
  /** SS58 address of task poster */
  poster: string;
  /** Human-readable task description (max 512 bytes on-chain) */
  description: string;
  /** CLAW reward in Planck units (1 CLAW = 1e12 Planck) */
  reward: bigint;
  /** Deadline block number (0 = no deadline) */
  deadline: number;
  /** Current task lifecycle status */
  status: TaskStatus;
  /** SS58 address of assigned worker, if any */
  assignee?: string;
  /** Block number when task was posted */
  postedAt: number;
}

/** Arguments for creating a new task */
export interface CreateTaskArgs {
  /** Task description visible to all bidders */
  description: string;
  /** CLAW reward escrowed on creation (in Planck units) */
  reward: bigint;
  /** Optional deadline block number */
  deadline?: number;
}

/** Arguments for bidding on a task */
export interface BidOnTaskArgs {
  taskId: number;
  /** Free-text proposal / covering note (max 256 bytes) */
  proposal: string;
}

// ──────────────────────────────────────────────────────────────────────────────
// Chain / Block
// ──────────────────────────────────────────────────────────────────────────────

/** Minimal block summary returned by getBlock() */
export interface BlockSummary {
  /** Block number */
  number: number;
  /** Hex-encoded block hash */
  hash: string;
  /** Hex-encoded parent hash */
  parentHash: string;
  /** Number of extrinsics in this block */
  extrinsicCount: number;
}
