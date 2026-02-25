/**
 * Unit tests for src/middleware/rateLimit.ts
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { ipRateLimitMiddleware, extractIp } from '../src/middleware/rateLimit.js';
import { initDb, recordDrip } from '../src/db.js';
import type { Config } from '../src/config.js';
import type { Database } from 'better-sqlite3';
import type { Request, Response } from 'express';
import { tmpdir } from 'os';
import { join } from 'path';
import { unlinkSync, existsSync } from 'fs';

function tmpDbPath(): string {
  return join(tmpdir(), `faucet-rl-test-${Date.now()}-${Math.random().toString(36).slice(2)}.db`);
}

function cleanup(dbPath: string, db: Database): void {
  db.close();
  for (const suffix of ['', '-wal', '-shm']) {
    const p = dbPath + suffix;
    if (existsSync(p)) unlinkSync(p);
  }
}

function makeConfig(ipRateLimit = 10): Config {
  return {
    port: 3000,
    rpcUrl: 'ws://localhost:9944',
    faucetSeed: '//Alice',
    githubClientId: '',
    githubClientSecret: '',
    sessionSecret: 'test-secret',
    dbPath: ':memory:',
    dripAmount: BigInt(100) * BigInt(10 ** 12),
    boostAmount: BigInt(1000) * BigInt(10 ** 12),
    cooldownMs: 24 * 60 * 60 * 1000,
    ipRateLimit,
  };
}

function mockReq(ip: string, forwarded?: string): Partial<Request> {
  return {
    headers: forwarded ? { 'x-forwarded-for': forwarded } : {},
    socket: { remoteAddress: ip } as never,
  };
}

function mockRes(): { status: number | null; body: unknown; setHeader: (k: string, v: string) => void } & Partial<Response> {
  const res = {
    status: null as number | null,
    body: null as unknown,
    setHeader: (_k: string, _v: string) => {},
    json: function (b: unknown) {
      this.body = b;
      return this;
    },
    status: function (code: number) {
      this.status = code;
      return this;
    },
  };
  // Fix: status as method
  const obj: { statusCode: number | null; body: unknown; headersSent?: boolean } & Partial<Response> = {
    statusCode: null,
    body: null,
  };
  let _status: number | null = null;
  const r = {
    _status,
    body: null as unknown,
    setHeader: (_k: string, _v: string) => {},
    status(code: number) {
      this._status = code;
      return this;
    },
    json(b: unknown) {
      this.body = b;
      return this;
    },
    getStatus() { return this._status; },
  } as unknown;
  return r as ReturnType<typeof mockRes>;
}

describe('extractIp', () => {
  it('returns remoteAddress when no X-Forwarded-For header', () => {
    const req = mockReq('192.168.1.1') as Request;
    expect(extractIp(req)).toBe('192.168.1.1');
  });

  it('returns first IP from X-Forwarded-For', () => {
    const req = mockReq('192.168.1.1', '10.0.0.1, 10.0.0.2') as Request;
    expect(extractIp(req)).toBe('10.0.0.1');
  });

  it('handles single X-Forwarded-For value', () => {
    const req = mockReq('192.168.1.1', '203.0.113.5') as Request;
    expect(extractIp(req)).toBe('203.0.113.5');
  });

  it('trims whitespace from X-Forwarded-For', () => {
    const req = mockReq('192.168.1.1', '  10.0.0.1  , 10.0.0.2') as Request;
    expect(extractIp(req)).toBe('10.0.0.1');
  });

  it('falls back to 127.0.0.1 when no address available', () => {
    const req = { headers: {}, socket: { remoteAddress: undefined } } as unknown as Request;
    expect(extractIp(req)).toBe('127.0.0.1');
  });
});

describe('ipRateLimitMiddleware', () => {
  let db: Database;
  let dbPath: string;

  beforeEach(() => {
    dbPath = tmpDbPath();
    db = initDb(dbPath);
  });

  afterEach(() => {
    cleanup(dbPath, db);
  });

  it('calls next() when under the rate limit', () => {
    const config = makeConfig(10);
    const middleware = ipRateLimitMiddleware({ db, config });
    const ip = '1.2.3.4';
    // Add 9 requests (under limit of 10)
    for (let i = 0; i < 9; i++) {
      recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', ip, '100', `0x${i}`);
    }

    const req = mockReq(ip) as Request;
    const res = { setHeader: () => {}, status: () => ({ json: () => {} }) } as unknown as Response;
    let nextCalled = false;
    const next = () => { nextCalled = true; };
    middleware(req, res, next);
    expect(nextCalled).toBe(true);
  });

  it('returns 429 when at the rate limit', () => {
    const config = makeConfig(3);
    const middleware = ipRateLimitMiddleware({ db, config });
    const ip = '1.2.3.5';
    for (let i = 0; i < 3; i++) {
      recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', ip, '100', `0x${i}`);
    }

    let statusCode = 0;
    let responseBody: unknown = null;
    const res = {
      setHeader: () => {},
      status(code: number) { statusCode = code; return this; },
      json(body: unknown) { responseBody = body; return this; },
    } as unknown as Response;

    let nextCalled = false;
    const req = mockReq(ip) as Request;
    middleware(req, res, () => { nextCalled = true; });
    expect(nextCalled).toBe(false);
    expect(statusCode).toBe(429);
    expect((responseBody as { error: string }).error).toBe('Too many requests');
  });

  it('allows requests from a different IP', () => {
    const config = makeConfig(2);
    const middleware = ipRateLimitMiddleware({ db, config });
    const blockedIp = '1.2.3.6';
    const allowedIp = '9.9.9.9';

    // Fill up the blocked IP
    for (let i = 0; i < 2; i++) {
      recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', blockedIp, '100', `0x${i}`);
    }

    // Different IP should still pass
    const req = mockReq(allowedIp) as Request;
    const res = { setHeader: () => {}, status: () => ({ json: () => {} }) } as unknown as Response;
    let nextCalled = false;
    middleware(req, res, () => { nextCalled = true; });
    expect(nextCalled).toBe(true);
  });

  it('does not count old requests outside the 1-hour window', () => {
    const config = makeConfig(2);
    const middleware = ipRateLimitMiddleware({ db, config });
    const ip = '1.2.3.7';

    // Insert 2 old records (outside window)
    db.prepare(
      `INSERT INTO drips (address, ip, amount, tx_hash, created_at)
       VALUES (?, ?, ?, ?, datetime('now', '-2 hours'))`
    ).run('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', ip, '100', '0xold1');
    db.prepare(
      `INSERT INTO drips (address, ip, amount, tx_hash, created_at)
       VALUES (?, ?, ?, ?, datetime('now', '-2 hours'))`
    ).run('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', ip, '100', '0xold2');

    // Should still allow (old records don't count)
    const req = mockReq(ip) as Request;
    const res = { setHeader: () => {}, status: () => ({ json: () => {} }) } as unknown as Response;
    let nextCalled = false;
    middleware(req, res, () => { nextCalled = true; });
    expect(nextCalled).toBe(true);
  });

  it('sets Retry-After header when rate limited', () => {
    const config = makeConfig(1);
    const middleware = ipRateLimitMiddleware({ db, config });
    const ip = '1.2.3.8';
    recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', ip, '100', '0x1');

    let retryAfterValue = '';
    const res = {
      setHeader(key: string, value: string) { if (key === 'Retry-After') retryAfterValue = value; },
      status() { return this; },
      json() { return this; },
    } as unknown as Response;

    middleware(mockReq(ip) as Request, res, () => {});
    expect(retryAfterValue).toBeTruthy();
    expect(parseInt(retryAfterValue, 10)).toBeGreaterThan(0);
  });
});
