/**
 * ClawChain Testnet Faucet — Entry Point
 *
 * Loads config, connects to the chain, creates the Express app,
 * and starts listening.
 *
 * Endpoints:
 *   POST /drip    — request 1000 CLAW (24h cooldown per address)
 *   GET  /status  — faucet health and stats
 *   GET  /balance — faucet account balance
 */

import express from 'express';
import cors from 'cors';
import { loadConfig } from './config';
import { connectChain } from './services/chain';
import { RateLimiter } from './services/rateLimit';
import { createFaucetRouter } from './routes/faucet';
import logger from './utils/logger';

async function main(): Promise<void> {
  const config = loadConfig();

  logger.info({ port: config.port, rpcUrl: config.rpcUrl }, 'Starting ClawChain Faucet');

  // Connect to chain
  let api;
  try {
    api = await connectChain(config.rpcUrl);
    const chain = await api.rpc.system.chain();
    const version = await api.rpc.system.version();
    logger.info({ chain: chain.toString(), version: version.toString() }, 'Connected to chain');
  } catch (err) {
    logger.error({ err }, 'Failed to connect to chain');
    process.exit(1);
  }

  // In-memory rate limiter
  const rateLimiter = new RateLimiter({
    cooldownMs: config.cooldownMs,
    ipRateLimit: config.ipRateLimit,
  });

  // Build Express app
  const app = express();
  app.set('trust proxy', 1);
  app.use(cors());
  app.use(express.json());

  // Routes
  app.use('/', createFaucetRouter({ api, config, rateLimiter }));

  // Health probe (used by Docker HEALTHCHECK)
  app.get('/health', (_req, res) => {
    res.status(200).json({ status: 'ok' });
  });

  // Global error handler
  app.use((err: Error, _req: express.Request, res: express.Response, _next: express.NextFunction) => {
    logger.error({ err }, 'Unhandled error');
    res.status(500).json({ error: 'Internal server error' });
  });

  // Start server
  const server = app.listen(config.port, () => {
    logger.info({ port: config.port }, 'Faucet listening');
  });

  // Graceful shutdown
  const shutdown = async (signal: string): Promise<void> => {
    logger.info({ signal }, 'Shutdown initiated');
    rateLimiter.destroy();
    server.close(async () => {
      await api.disconnect();
      logger.info('Shutdown complete');
      process.exit(0);
    });
  };

  process.on('SIGTERM', () => void shutdown('SIGTERM'));
  process.on('SIGINT', () => void shutdown('SIGINT'));
}

main().catch((err) => {
  logger.error({ err }, 'Fatal error');
  process.exit(1);
});
