/**
 * IP-based rate limiting middleware.
 * Checks the SQLite drips table: max N requests per hour per IP.
 */

import type { Request, Response, NextFunction, RequestHandler } from 'express';
import type { Database } from 'better-sqlite3';
import type { Config } from '../config.js';
import { getIpRequestCount } from '../db.js';

interface Deps {
  db: Database;
  config: Config;
}

/**
 * Extract the real client IP from the request.
 * Trusts only the first value of X-Forwarded-For (set by reverse proxy).
 */
export function extractIp(req: Request): string {
  const forwarded = req.headers['x-forwarded-for'];
  if (forwarded) {
    const first = (Array.isArray(forwarded) ? forwarded[0] : forwarded).split(',')[0];
    return first.trim();
  }
  return req.socket.remoteAddress ?? '127.0.0.1';
}

export function ipRateLimitMiddleware(deps: Deps): RequestHandler {
  return (req: Request, res: Response, next: NextFunction): void => {
    const ip = extractIp(req);
    const windowMs = 60 * 60 * 1000; // 1 hour
    const count = getIpRequestCount(deps.db, ip, windowMs);

    if (count >= deps.config.ipRateLimit) {
      const retryAfter = Math.ceil(windowMs / 1000);
      res.setHeader('Retry-After', retryAfter);
      res.status(429).json({
        error: 'Too many requests',
        retry_after: retryAfter,
      });
      return;
    }

    next();
  };
}
