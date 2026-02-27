/**
 * Configuration loader
 * Reads ClawChain plugin settings from environment variables.
 */

export interface PluginConfig {
  rpcUrl: string;
  keypairPath: string;
  connectTimeoutMs: number;
}

export class ConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ConfigError';
  }
}

/**
 * Load and validate plugin configuration from environment variables.
 *
 * Required:
 *   CLAWCHAIN_RPC_URL     - WebSocket RPC endpoint (e.g. ws://localhost:9944)
 *   CLAWCHAIN_KEYPAIR_PATH - Path to keypair file (mnemonic or JSON)
 *
 * Optional:
 *   CLAWCHAIN_CONNECT_TIMEOUT_MS - Connection timeout in ms (default: 30000)
 */
export function loadConfig(): PluginConfig {
  const rpcUrl = process.env['CLAWCHAIN_RPC_URL'];
  if (!rpcUrl) {
    throw new ConfigError('Missing required environment variable: CLAWCHAIN_RPC_URL');
  }
  if (!rpcUrl.startsWith('ws://') && !rpcUrl.startsWith('wss://')) {
    throw new ConfigError(`CLAWCHAIN_RPC_URL must start with ws:// or wss://, got: ${rpcUrl}`);
  }

  const keypairPath = process.env['CLAWCHAIN_KEYPAIR_PATH'];
  if (!keypairPath) {
    throw new ConfigError('Missing required environment variable: CLAWCHAIN_KEYPAIR_PATH');
  }

  const timeoutRaw = process.env['CLAWCHAIN_CONNECT_TIMEOUT_MS'];
  const connectTimeoutMs = timeoutRaw ? parseInt(timeoutRaw, 10) : 30_000;
  if (isNaN(connectTimeoutMs) || connectTimeoutMs <= 0) {
    throw new ConfigError(`CLAWCHAIN_CONNECT_TIMEOUT_MS must be a positive integer, got: ${timeoutRaw}`);
  }

  return { rpcUrl, keypairPath, connectTimeoutMs };
}
