/**
 * ClawChain Testnet Faucet — Entry Point
 *
 * Loads config, connects to the chain, initialises the database,
 * creates the Express app, and starts listening.
 */

import { loadConfig } from './config.js';
import { initDb } from './db.js';
import { connectChain } from './chain.js';
import { createApp } from './server.js';

async function main(): Promise<void> {
  const config = loadConfig();

  console.log(`[${new Date().toISOString()}] Starting ClawChain Faucet...`);
  console.log(`[${new Date().toISOString()}] Connecting to chain at ${config.rpcUrl}`);

  // Initialise SQLite database
  const db = initDb(config.dbPath);
  console.log(`[${new Date().toISOString()}] Database initialised at ${config.dbPath}`);

  // Connect to Substrate chain
  let api;
  try {
    api = await connectChain(config.rpcUrl);
    const chain = await api.rpc.system.chain();
    const version = await api.rpc.system.version();
    console.log(`[${new Date().toISOString()}] Connected to ${chain} (${version})`);
  } catch (err) {
    console.error(`[${new Date().toISOString()}] Failed to connect to chain:`, err);
    process.exit(1);
  }

  // Create Express app
  const app = await createApp(config, db, api);

  // Start listening
  const server = app.listen(config.port, () => {
    console.log(`[${new Date().toISOString()}] Faucet listening on port ${config.port}`);
  });

  // Graceful shutdown
  const shutdown = async (signal: string): Promise<void> => {
    console.log(`[${new Date().toISOString()}] ${signal} received — shutting down...`);
    server.close(async () => {
      await api.disconnect();
      db.close();
      console.log(`[${new Date().toISOString()}] Shutdown complete.`);
      process.exit(0);
    });
  };

  process.on('SIGTERM', () => void shutdown('SIGTERM'));
  process.on('SIGINT', () => void shutdown('SIGINT'));
}

main().catch((err) => {
  console.error(`[${new Date().toISOString()}] Fatal error:`, err);
  process.exit(1);
});
