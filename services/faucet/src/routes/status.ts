/**
 * GET /status — faucet health and stats endpoint.
 *
 * Response 200: { balance, total_drips, total_amount, unique_addresses }
 * Response 503: { error }
 */

import { Router, Request, Response } from 'express';
import type { ApiPromise } from '@polkadot/api';
import type { Database } from 'better-sqlite3';
import type { Config } from '../config.js';
import { getStats } from '../db.js';
import { getFaucetBalance } from '../chain.js';

interface Deps {
  db: Database;
  api: ApiPromise;
  config: Config;
}

export function statusRouter(deps: Deps): Router {
  const router = Router();

  router.get('/', async (_req: Request, res: Response): Promise<void> => {
    try {
      const [balance, stats] = await Promise.all([
        getFaucetBalance(deps.api, deps.config.faucetSeed),
        Promise.resolve(getStats(deps.db)),
      ]);

      res.status(200).json({
        balance,
        total_drips: stats.total_drips,
        total_amount: stats.total_amount,
        unique_addresses: stats.unique_addresses,
      });
    } catch (err) {
      console.error(`[${new Date().toISOString()}] Status error:`, err);
      res.status(503).json({ error: 'Chain unavailable' });
    }
  });

  return router;
}
