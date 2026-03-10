/**
 * Faucet API routes:
 *
 *   POST /drip    — request CLAW tokens
 *   GET  /status  — faucet health and stats
 *   GET  /balance — faucet account balance
 */

import { Router, Request, Response } from 'express';
import type { ApiPromise } from '@polkadot/api';
import type { Config } from '../config';
import type { RateLimiter } from '../services/rateLimit';
import { validateAddress } from '../utils/validate';
import { transferClaw, getFaucetBalance, getFaucetAddress } from '../services/chain';
import logger from '../utils/logger';

interface Deps {
  api: ApiPromise;
  config: Config;
  rateLimiter: RateLimiter;
}

/**
 * Extract the real client IP from the request.
 * Trusts only the first value of X-Forwarded-For.
 */
function extractIp(req: Request): string {
  const forwarded = req.headers['x-forwarded-for'];
  if (forwarded) {
    const first = (Array.isArray(forwarded) ? forwarded[0] : forwarded).split(',')[0];
    return first?.trim() ?? '127.0.0.1';
  }
  return req.socket.remoteAddress ?? '127.0.0.1';
}

export function createFaucetRouter(deps: Deps): Router {
  const router = Router();
  const { api, config, rateLimiter } = deps;

  // ── POST /drip ─────────────────────────────────────────────────────────────
  router.post('/drip', async (req: Request, res: Response): Promise<void> => {
    const { address } = req.body as { address?: unknown };

    // 1. Validate address
    const addrError = validateAddress(address);
    if (addrError) {
      res.status(400).json({ error: addrError });
      return;
    }

    const trimmedAddress = (address as string).trim();
    const ip = extractIp(req);

    // 2. IP rate limit check
    const ipCheck = rateLimiter.checkIpRateLimit(ip);
    if (ipCheck.blocked) {
      res.setHeader('Retry-After', String(ipCheck.retryAfter));
      res.status(429).json({
        error: 'Too many requests from this IP',
        retry_after: ipCheck.retryAfter,
      });
      return;
    }

    // 3. Address cooldown check
    const cooldownCheck = rateLimiter.checkAddressCooldown(trimmedAddress);
    if (cooldownCheck.blocked) {
      res.status(429).json({
        error: 'Address is on cooldown',
        next_drip_at: cooldownCheck.nextDripAt,
      });
      return;
    }

    // 4. Transfer CLAW
    let txHash: string;
    try {
      txHash = await transferClaw(api, config.faucetSeed, trimmedAddress, config.dripAmountPlanck);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      logger.error({ err, address: trimmedAddress }, 'Transfer failed');

      if (
        msg.toLowerCase().includes('depleted') ||
        msg.toLowerCase().includes('insufficient') ||
        msg.toLowerCase().includes('keepalive')
      ) {
        res.status(503).json({ error: 'Faucet depleted' });
        return;
      }
      if (
        msg.toLowerCase().includes('connect') ||
        msg.toLowerCase().includes('unavailable') ||
        msg.toLowerCase().includes('timeout')
      ) {
        res.status(503).json({ error: 'Chain unavailable' });
        return;
      }
      res.status(500).json({ error: 'Transfer failed' });
      return;
    }

    // 5. Record drip
    rateLimiter.recordDrip(trimmedAddress, ip, txHash);
    const nextDripAt = new Date(Date.now() + config.cooldownMs).toISOString();

    logger.info({ address: trimmedAddress, amount: config.dripAmountClaw, txHash }, 'Drip sent');

    res.status(200).json({
      tx_hash: txHash,
      amount: `${config.dripAmountClaw} CLAW`,
      next_drip_at: nextDripAt,
    });
  });

  // ── GET /status ────────────────────────────────────────────────────────────
  router.get('/status', async (_req: Request, res: Response): Promise<void> => {
    try {
      const balance = await getFaucetBalance(api, config.faucetSeed);
      const faucetAddress = getFaucetAddress(config.faucetSeed);

      res.status(200).json({
        status: 'ok',
        faucet_address: faucetAddress,
        balance_planck: balance,
        balance_claw: (BigInt(balance) / BigInt(10 ** 12)).toString(),
        drip_amount_claw: config.dripAmountClaw,
        cooldown_hours: config.cooldownMs / (60 * 60 * 1000),
        total_drips: rateLimiter.getTotalDrips(),
      });
    } catch (err) {
      logger.error({ err }, 'Status check failed');
      res.status(503).json({ error: 'Chain unavailable' });
    }
  });

  // ── GET /balance ───────────────────────────────────────────────────────────
  router.get('/balance', async (_req: Request, res: Response): Promise<void> => {
    try {
      const balance = await getFaucetBalance(api, config.faucetSeed);
      const address = getFaucetAddress(config.faucetSeed);

      res.status(200).json({
        address,
        balance_planck: balance,
        balance_claw: (BigInt(balance) / BigInt(10 ** 12)).toString(),
      });
    } catch (err) {
      logger.error({ err }, 'Balance check failed');
      res.status(503).json({ error: 'Chain unavailable' });
    }
  });

  return router;
}
