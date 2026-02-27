/**
 * OpenClaw Plugin — Entry Point
 *
 * Usage:
 *   CLAWCHAIN_RPC_URL=ws://localhost:9944 \
 *   CLAWCHAIN_KEYPAIR_PATH=./keypair.json \
 *   node dist/index.js [command]
 *
 * Commands:
 *   (none)              — Initialize plugin (register DID) and exit
 *   clawchain_status    — Print agent status as JSON
 */

import { loadConfig } from './config';
import { OpenClawPlugin } from './plugin';

async function main(): Promise<void> {
  const config = loadConfig();
  const plugin = new OpenClawPlugin(config);

  const command = process.argv[2];

  try {
    await plugin.initialize();

    if (command) {
      const result = await plugin.handleCommand(command);
      process.stdout.write(JSON.stringify(result, null, 2) + '\n');
      if (!result.success) {
        process.exitCode = 1;
      }
    } else {
      console.log('[ClawChain] Plugin initialized successfully.');
    }
  } finally {
    await plugin.shutdown();
  }
}

main().catch((err: Error) => {
  console.error('[ClawChain] Fatal error:', err.message);
  process.exit(1);
});

export { OpenClawPlugin } from './plugin';
export { ClawChainRpcClient } from './rpc/client';
export { DIDRegistrar } from './did/registrar';
export { StatusChecker } from './status/checker';
export { loadConfig } from './config';
export type { PluginConfig } from './config';
export type { CommandResult } from './plugin';
export type { AgentStatus } from './status/checker';
export type { DIDRegistrationResult, RegistrationStatus } from './did/registrar';
