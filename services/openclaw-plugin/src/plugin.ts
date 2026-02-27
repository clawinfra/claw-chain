/**
 * OpenClaw Plugin for ClawChain
 * Orchestrates startup DID registration and the clawchain_status skill command.
 */

import { ClawChainRpcClient } from './rpc/client';
import { DIDRegistrar } from './did/registrar';
import { StatusChecker } from './status/checker';
import { PluginConfig } from './config';

export interface CommandResult {
  success: boolean;
  command: string;
  data?: unknown;
  error?: string;
}

export class OpenClawPlugin {
  private readonly rpcClient: ClawChainRpcClient;
  private registrar: DIDRegistrar | null = null;
  private statusChecker: StatusChecker | null = null;
  private initialized = false;

  constructor(private readonly config: PluginConfig) {
    this.rpcClient = new ClawChainRpcClient({
      rpcUrl: config.rpcUrl,
      connectTimeoutMs: config.connectTimeoutMs,
    });
  }

  /**
   * Initialize the plugin:
   * 1. Connect to ClawChain node
   * 2. Register agent DID on-chain
   */
  async initialize(): Promise<void> {
    if (this.initialized) return;

    const api = await this.rpcClient.connect();

    this.registrar = new DIDRegistrar(api, { keypairPath: this.config.keypairPath });
    this.statusChecker = new StatusChecker(api);

    const result = await this.registrar.registerDID();

    if (result.alreadyRegistered) {
      console.log(`[ClawChain] DID already registered: ${result.did}`);
    } else {
      console.log(`[ClawChain] DID registered: ${result.did} (tx: ${result.txHash}, block: ${result.blockHash})`);
    }

    this.initialized = true;
  }

  /**
   * Handle the clawchain_status skill command.
   * Returns JSON with DID status, gas quota, and reputation.
   */
  async handleStatusCommand(): Promise<CommandResult> {
    if (!this.initialized || !this.registrar || !this.statusChecker) {
      return {
        success: false,
        command: 'clawchain_status',
        error: 'Plugin not initialized. Call initialize() first.',
      };
    }

    try {
      const keypair = this.registrar.loadKeypair();
      const did = this.registrar.deriveDID();
      const accountId = keypair.address;

      const status = await this.statusChecker.getFullStatus(did, accountId);

      return {
        success: true,
        command: 'clawchain_status',
        data: status,
      };
    } catch (err) {
      return {
        success: false,
        command: 'clawchain_status',
        error: (err as Error).message,
      };
    }
  }

  /**
   * Dispatch a skill command by name.
   */
  async handleCommand(command: string, _args?: Record<string, unknown>): Promise<CommandResult> {
    switch (command) {
      case 'clawchain_status':
        return this.handleStatusCommand();

      default:
        return {
          success: false,
          command,
          error: `Unknown command: ${command}. Available: clawchain_status`,
        };
    }
  }

  async shutdown(): Promise<void> {
    await this.rpcClient.disconnect();
    this.initialized = false;
  }

  get isInitialized(): boolean {
    return this.initialized;
  }
}
