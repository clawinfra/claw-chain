/**
 * @clawinfra/clawchain-sdk — ClawChainClient
 *
 * Low-level connection wrapper around @polkadot/api's ApiPromise.
 * Handles connect/disconnect lifecycle and exposes common chain queries.
 */

import { ApiPromise, WsProvider } from "@polkadot/api";
import type { BlockSummary, ClawChainConfig } from "./types.js";

/** Default ClawChain testnet WebSocket endpoint */
export const TESTNET_WS_URL = "wss://testnet.clawchain.win";

/**
 * ClawChainClient — the entry-point for all SDK interactions.
 *
 * ```ts
 * const client = new ClawChainClient("wss://testnet.clawchain.win");
 * await client.connect();
 *
 * const block = await client.getBlockNumber();
 * console.log("Current block:", block);
 *
 * await client.disconnect();
 * ```
 */
export class ClawChainClient {
  private readonly wsUrl: string;
  private _api: ApiPromise | null = null;

  constructor(wsUrl: string = TESTNET_WS_URL) {
    this.wsUrl = wsUrl;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Connect to the ClawChain node and wait until the API is ready.
   * Safe to call multiple times — subsequent calls return the existing connection.
   */
  async connect(): Promise<void> {
    if (this._api !== null) return; // already connected

    const provider = new WsProvider(this.wsUrl);
    this._api = await ApiPromise.create({ provider });
    await this._api.isReady;
  }

  /**
   * Disconnect from the chain and clean up the WebSocket connection.
   */
  async disconnect(): Promise<void> {
    if (this._api === null) return;
    await this._api.disconnect();
    this._api = null;
  }

  /**
   * Returns true when the client has an active connection.
   */
  get isConnected(): boolean {
    return this._api !== null && this._api.isConnected;
  }

  /**
   * Access the raw @polkadot/api `ApiPromise` for advanced usage.
   * Will throw if not yet connected.
   */
  get api(): ApiPromise {
    if (this._api === null) {
      throw new Error(
        "ClawChainClient is not connected. Call connect() first."
      );
    }
    return this._api;
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Chain queries
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Returns the current best block number.
   */
  async getBlockNumber(): Promise<number> {
    const header = await this.api.rpc.chain.getHeader();
    return header.number.toNumber();
  }

  /**
   * Returns a summary of the latest finalised block (or a specific block by hash).
   *
   * @param blockHash  Optional hex block hash. Defaults to the latest block.
   */
  async getBlock(blockHash?: string): Promise<BlockSummary> {
    const signedBlock = blockHash
      ? await this.api.rpc.chain.getBlock(blockHash)
      : await this.api.rpc.chain.getBlock();

    const { block } = signedBlock;
    const header = block.header;

    return {
      number: header.number.toNumber(),
      hash: header.hash.toHex(),
      parentHash: header.parentHash.toHex(),
      extrinsicCount: block.extrinsics.length,
    };
  }

  /**
   * Returns the free CLAW balance for `address` in Planck units.
   * 1 CLAW = 1_000_000_000_000 Planck (12 decimals, Substrate default).
   *
   * @param address  SS58-encoded account address
   */
  async getBalance(address: string): Promise<bigint> {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const result = await (this.api.query as any).system.account(address);
    // result.data.free is a u128 Balance
    const free: bigint = BigInt(result.data.free.toString());
    return free;
  }

  /**
   * Formats a Planck-unit balance into a human-readable CLAW string.
   *
   * @param planck  Balance in Planck units
   * @param decimals  Token decimals (default: 12 for ClawChain)
   */
  static formatBalance(planck: bigint, decimals = 12): string {
    const divisor = BigInt(10 ** decimals);
    const whole = planck / divisor;
    const fraction = planck % divisor;
    const fracStr = fraction.toString().padStart(decimals, "0").replace(/0+$/, "");
    return fracStr.length > 0 ? `${whole}.${fracStr} CLAW` : `${whole} CLAW`;
  }

  /**
   * Returns the chain name and spec version for diagnostic purposes.
   */
  async getChainInfo(): Promise<{ chainName: string; specVersion: number }> {
    const [chainName, runtimeVersion] = await Promise.all([
      this.api.rpc.system.chain(),
      this.api.rpc.state.getRuntimeVersion(),
    ]);
    return {
      chainName: chainName.toString(),
      specVersion: runtimeVersion.specVersion.toNumber(),
    };
  }

  // ────────────────────────────────────────────────────────────────────────────
  // Static factory helpers
  // ────────────────────────────────────────────────────────────────────────────

  /**
   * Create a client from a {@link ClawChainConfig} object and immediately connect.
   */
  static async fromConfig(config: ClawChainConfig): Promise<ClawChainClient> {
    const client = new ClawChainClient(config.wsUrl);
    await client.connect();
    return client;
  }
}
