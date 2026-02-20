/**
 * @clawinfra/clawchain-sdk
 *
 * TypeScript/JavaScript SDK for EvoClaw agents to interact with ClawChain L1.
 *
 * Quick-start:
 * ```ts
 * import { ClawChainClient, AgentRegistry, TaskMarket } from "@clawinfra/clawchain-sdk";
 *
 * const client = new ClawChainClient("wss://testnet.clawchain.win");
 * await client.connect();
 *
 * const registry = new AgentRegistry(client);
 * const market   = new TaskMarket(client);
 *
 * // Query the chain
 * const block = await client.getBlockNumber();
 * const balance = await client.getBalance("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
 *
 * // Register an agent
 * const txHash = await registry.registerAgent(
 *   signer,
 *   "did:claw:agent:mybot",
 *   { name: "MyBot", type: "task-executor" }
 * );
 *
 * await client.disconnect();
 * ```
 *
 * @module @clawinfra/clawchain-sdk
 */

// Core client
export { ClawChainClient, TESTNET_WS_URL } from "./client.js";

// Pallet wrappers
export { AgentRegistry } from "./agents.js";
export { TaskMarket } from "./tasks.js";

// Types
export type {
  // Config
  ClawChainConfig,

  // Agent types
  AgentInfo,
  AgentStatus,
  RegisterAgentArgs,

  // Task types
  TaskInfo,
  TaskStatus,
  CreateTaskArgs,
  BidOnTaskArgs,

  // Chain types
  BlockSummary,
} from "./types.js";
