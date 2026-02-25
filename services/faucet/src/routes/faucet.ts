/**
 * POST /faucet — main drip endpoint.
 *
 * Request body: { address: string }
 * Session:      req.session.githubUser may be set
 *
 * Response 200: { tx_hash, amount, next_drip_at }
 * Response 400: { error }
 * Response 429: { error, next_drip_at }
 * Response 500: { error }
 * Response 503: { error }
 */

import { Router, Request, Response } from 'express';
import { decodeAddress } from '@polkadot/util-crypto';
import type { ApiPromise } from '@polkadot/api';
import type { Database } from 'better-sqlite3';
import type { Config } from '../config.js';
import { getLastDrip, recordDrip } from '../db.js';
import { transferClaw } from '../chain.js';
import { extractIp } from '../middleware/rateLimit.js';

interface Deps {
  db: Database;
  api: ApiPromise;
  config: Config;
}

function isValidSS58(address: string): boolean {
  try {
    decodeAddress(address);
    return true;
  } catch {
    return false;
  }
}

export function faucetRouter(deps: Deps): Router {
  const router = Router();

  router.post('/', async (req: Request, res: Response): Promise<void> => {
    const { address } = req.body as { address?: string };

    // ── 1. Validate address ──────────────────────────────────────────────────
    if (!address || typeof address !== 'string' || address.trim() === '') {
      res.status(400).json({ error: 'Address is required' });
      return;
    }

    const trimmedAddress = address.trim();
    if (!isValidSS58(trimmedAddress)) {
      res.status(400).json({ error: 'Invalid SS58 address' });
      return;
    }

    // ── 2. Address cooldown check ────────────────────────────────────────────
    const lastDrip = getLastDrip(deps.db, trimmedAddress);
    if (lastDrip) {
      const lastAt = new Date(lastDrip.created_at + 'Z').getTime();
      const elapsed = Date.now() - lastAt;
      if (elapsed < deps.config.cooldownMs) {
        const nextDripAt = new Date(lastAt + deps.config.cooldownMs).toISOString();
        res.status(429).json({
          error: 'Address rate limited',
          next_drip_at: nextDripAt,
        });
        return;
      }
    }

    // ── 3. Determine drip amount ─────────────────────────────────────────────
    // @ts-expect-error: session module augmentation
    const githubUser = req.session?.githubUser as { username: string; id: number } | undefined;
    const isGitHubAuthed = Boolean(githubUser);
    const amount = isGitHubAuthed ? deps.config.boostAmount : deps.config.dripAmount;
    const amountLabel = isGitHubAuthed ? '1000' : '100';

    const ip = extractIp(req);

    // ── 4. Transfer CLAW ─────────────────────────────────────────────────────
    let txHash: string;
    try {
      txHash = await transferClaw(deps.api, deps.config.faucetSeed, trimmedAddress, amount);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.toLowerCase().includes('depleted') || msg.toLowerCase().includes('insufficient')) {
        res.status(503).json({ error: 'Faucet depleted' });
        return;
      }
      if (msg.toLowerCase().includes('unavailable') || msg.toLowerCase().includes('connect')) {
        res.status(503).json({ error: 'Chain unavailable' });
        return;
      }
      console.error(`[${new Date().toISOString()}] Transfer error:`, msg);
      res.status(500).json({ error: 'Transfer failed' });
      return;
    }

    // ── 5. Record drip ───────────────────────────────────────────────────────
    recordDrip(deps.db, trimmedAddress, ip, amount.toString(), txHash, githubUser?.username);

    const nextDripAt = new Date(Date.now() + deps.config.cooldownMs).toISOString();

    res.status(200).json({
      tx_hash: txHash,
      amount: `${amountLabel} CLAW`,
      next_drip_at: nextDripAt,
    });
  });

  return router;
}
