/**
 * ClawChain RPC Client
 * Manages the Substrate JSON-RPC connection via @polkadot/api
 */

import { ApiPromise, WsProvider } from '@polkadot/api';

export interface RpcClientConfig {
  rpcUrl: string;
  connectTimeoutMs?: number;
}

export class ClawChainRpcClient {
  private api: ApiPromise | null = null;
  private readonly config: Required<RpcClientConfig>;

  constructor(config: RpcClientConfig) {
    this.config = {
      rpcUrl: config.rpcUrl,
      connectTimeoutMs: config.connectTimeoutMs ?? 30_000,
    };
  }

  /**
   * Connect to the ClawChain node and return the API instance.
   * Safe to call multiple times — returns existing connection if already connected.
   */
  async connect(): Promise<ApiPromise> {
    if (this.api && this.api.isConnected) {
      return this.api;
    }

    const provider = new WsProvider(this.config.rpcUrl, false);

    const connectPromise = new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`RPC connection timed out after ${this.config.connectTimeoutMs}ms`));
      }, this.config.connectTimeoutMs);

      provider.on('connected', () => {
        clearTimeout(timeout);
        resolve();
      });

      provider.on('error', (err: Error) => {
        clearTimeout(timeout);
        reject(new Error(`RPC provider error: ${err.message}`));
      });
    });

    provider.connect();
    await connectPromise;

    this.api = await ApiPromise.create({ provider });
    await this.api.isReady;

    return this.api;
  }

  /**
   * Disconnect from the node. No-op if already disconnected.
   */
  async disconnect(): Promise<void> {
    if (this.api) {
      await this.api.disconnect();
      this.api = null;
    }
  }

  /**
   * Returns the underlying API instance. Throws if not connected.
   */
  getApi(): ApiPromise {
    if (!this.api || !this.api.isConnected) {
      throw new Error('Not connected to ClawChain node. Call connect() first.');
    }
    return this.api;
  }

  get isConnected(): boolean {
    return this.api?.isConnected ?? false;
  }
}
