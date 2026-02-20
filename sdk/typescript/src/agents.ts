/**
 * @clawinfra/clawchain-sdk — AgentRegistry
 *
 * High-level wrapper for the `pallet-agent-registry` extrinsics and storage.
 *
 * Extrinsics (writes — require a signer):
 *   - registerAgent(signer, did, metadata) → tx hash
 *   - updateReputation(signer, agentId, delta) → tx hash
 *
 * Queries (reads — free):
 *   - getAgent(agentId) → AgentInfo | null
 *   - listAgents() → AgentInfo[]
 */

import type { KeyringPair } from "@polkadot/keyring/types";
import type { ClawChainClient } from "./client.js";
import type { AgentInfo, AgentStatus } from "./types.js";

/** Raw on-chain codec type returned by storage query (approximate shape) */
interface RawAgentInfo {
  owner: { toString(): string };
  did: { toHuman?(): unknown; toString(): string };
  metadata: { toHuman?(): unknown; toString(): string };
  reputation: { toNumber(): number };
  registeredAt: { toNumber(): number };
  lastActive: { toNumber(): number };
  status: { type: string };
}

/**
 * AgentRegistry — interact with the ClawChain `pallet-agent-registry`.
 *
 * ```ts
 * const client = new ClawChainClient("wss://testnet.clawchain.win");
 * await client.connect();
 *
 * const registry = new AgentRegistry(client);
 * const txHash = await registry.registerAgent(
 *   keyring.getPair(myAddress),
 *   "did:claw:agent:mybot",
 *   { name: "MyBot", type: "task-executor" }
 * );
 * console.log("Registered! tx:", txHash);
 * ```
 */
export class AgentRegistry {
  private readonly client: ClawChainClient;

  constructor(client: ClawChainClient) {
    this.client = client;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Extrinsics (signed writes)
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Register a new AI agent on-chain.
   *
   * Submits `agentRegistry.registerAgent(did, metadata)` and waits for
   * inclusion in a block.
   *
   * @param signer    KeyringPair with funds to pay tx fees
   * @param did       Decentralized Identifier, e.g. "did:claw:agent:abc123"
   * @param metadata  Arbitrary JSON object (max 1 KB serialised)
   * @returns Hex-encoded extrinsic hash
   */
  async registerAgent(
    signer: KeyringPair,
    did: string,
    metadata: Record<string, unknown>
  ): Promise<string> {
    const api = this.client.api;
    const metadataStr = JSON.stringify(metadata);

    return new Promise((resolve, reject) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (api.tx as any).agentRegistry
        .registerAgent(
          Array.from(Buffer.from(did, "utf8")),
          Array.from(Buffer.from(metadataStr, "utf8"))
        )
        .signAndSend(signer, ({ status, dispatchError }: { status: { isInBlock: boolean; asInBlock: { toHex(): string } }; dispatchError?: { isModule: boolean; asModule: unknown } }) => {
          if (dispatchError) {
            if (dispatchError.isModule) {
              const decoded = api.registry.findMetaError(
                dispatchError.asModule as Parameters<typeof api.registry.findMetaError>[0]
              );
              reject(
                new Error(
                  `Transaction failed: ${decoded.section}.${decoded.name} — ${decoded.docs.join(" ")}`
                )
              );
            } else {
              reject(new Error(`Transaction failed: ${dispatchError.toString()}`));
            }
          }
          if (status.isInBlock) {
            resolve(status.asInBlock.toHex());
          }
        })
        .catch(reject);
    });
  }

  /**
   * Update an agent's on-chain reputation score.
   *
   * Calls `agentRegistry.updateReputation(agentId, delta)`.
   * Note: on mainnet this will be governance/sudo-gated; on testnet it may
   * be open for testing purposes.
   *
   * @param signer   KeyringPair (must be root or governance-approved)
   * @param agentId  Numeric agent ID
   * @param delta    Signed reputation change (positive or negative)
   * @returns Hex-encoded extrinsic hash
   */
  async updateReputation(
    signer: KeyringPair,
    agentId: number,
    delta: number
  ): Promise<string> {
    const api = this.client.api;

    return new Promise((resolve, reject) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      (api.tx as any).agentRegistry
        .updateReputation(agentId, delta)
        .signAndSend(signer, ({ status, dispatchError }: { status: { isInBlock: boolean; asInBlock: { toHex(): string } }; dispatchError?: { isModule: boolean; asModule: unknown } }) => {
          if (dispatchError) {
            if (dispatchError.isModule) {
              const decoded = api.registry.findMetaError(
                dispatchError.asModule as Parameters<typeof api.registry.findMetaError>[0]
              );
              reject(
                new Error(
                  `Transaction failed: ${decoded.section}.${decoded.name} — ${decoded.docs.join(" ")}`
                )
              );
            } else {
              reject(new Error(`Transaction failed: ${dispatchError.toString()}`));
            }
          }
          if (status.isInBlock) {
            resolve(status.asInBlock.toHex());
          }
        })
        .catch(reject);
    });
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Queries (free reads)
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Fetch a single agent by its numeric ID.
   *
   * @param agentId  On-chain agent ID
   * @returns AgentInfo if found, null otherwise
   */
  async getAgent(agentId: number): Promise<AgentInfo | null> {
    const api = this.client.api;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rawOption = await (api.query as any).agentRegistry.agents(agentId);

    if (rawOption.isNone) return null;

    const raw: RawAgentInfo = rawOption.unwrap();
    return this._decodeAgentInfo(agentId, raw);
  }

  /**
   * List all registered agents on-chain.
   *
   * Reads from the `Agents` StorageMap by iterating all entries.
   * For very large sets, consider paginating via the raw API.
   *
   * @returns Array of AgentInfo (may be empty)
   */
  async listAgents(): Promise<AgentInfo[]> {
    const api = this.client.api;
    // entries() returns [StorageKey, Value][]
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const entries: [{ args: [{ toNumber(): number }] }, RawAgentInfo][] =
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await (api.query as any).agentRegistry.agents.entries();

    return entries.map(([key, raw]) => {
      const agentId = key.args[0].toNumber();
      return this._decodeAgentInfo(agentId, raw);
    });
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Internal helpers
  // ────────────────────────────────────────────────────────────────────────────

  private _decodeAgentInfo(agentId: number, raw: RawAgentInfo): AgentInfo {
    // DID is stored as BoundedVec<u8> — convert bytes back to UTF-8
    const didHuman = raw.did.toHuman ? raw.did.toHuman() : raw.did.toString();
    const did = Array.isArray(didHuman)
      ? Buffer.from(didHuman as number[]).toString("utf8")
      : String(didHuman);

    // Metadata is stored as BoundedVec<u8> — convert bytes back to JSON
    const metaHuman = raw.metadata.toHuman
      ? raw.metadata.toHuman()
      : raw.metadata.toString();
    let metadata: Record<string, unknown> = {};
    try {
      const metaStr = Array.isArray(metaHuman)
        ? Buffer.from(metaHuman as number[]).toString("utf8")
        : String(metaHuman);
      metadata = JSON.parse(metaStr) as Record<string, unknown>;
    } catch {
      // non-JSON metadata — store raw
      metadata = { raw: metaHuman };
    }

    const status = raw.status.type as AgentStatus;

    return {
      agentId,
      owner: raw.owner.toString(),
      did,
      metadata,
      reputation: raw.reputation.toNumber(),
      registeredAt: raw.registeredAt.toNumber(),
      lastActive: raw.lastActive.toNumber(),
      status,
    };
  }
}
