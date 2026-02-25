/**
 * Integration tests for POST /faucet using supertest and a mocked chain.
 */

import { describe, it, expect, beforeAll, afterAll, vi, beforeEach } from 'vitest';
import supertest from 'supertest';
import { initDb, recordDrip } from '../src/db.js';
import { createApp } from '../src/server.js';
import type { Config } from '../src/config.js';
import type { Database } from 'better-sqlite3';
import type { ApiPromise } from '@polkadot/api';
import { tmpdir } from 'os';
import { join } from 'path';
import { unlinkSync, existsSync } from 'fs';
import type { Express } from 'express';

// ── Mock chain.ts ──────────────────────────────────────────────────────────
vi.mock('../src/chain.js', () => ({
  connectChain: vi.fn(),
  transferClaw: vi.fn().mockResolvedValue('0xdeadbeef1234567890abcdef'),
  getFaucetBalance: vi.fn().mockResolvedValue('999000000000000000'),
}));

import { transferClaw, getFaucetBalance } from '../src/chain.js';

// ── Substrate dev account SS58 addresses (generic prefix 42) ──────────────
// These are all valid and well-known test accounts
const ALICE   = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const BOB     = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';
const CHARLIE = '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y';
const DAVE    = '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy';
const EVE     = '5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw';
const FERDIE  = '5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL';

// ── Helpers ────────────────────────────────────────────────────────────────
function tmpDbPath(): string {
  return join(tmpdir(), `faucet-api-${Date.now()}-${Math.random().toString(36).slice(2)}.db`);
}

function cleanupDb(dbPath: string, db: Database): void {
  db.close();
  for (const suffix of ['', '-wal', '-shm']) {
    const p = dbPath + suffix;
    if (existsSync(p)) unlinkSync(p);
  }
}

function makeConfig(override: Partial<Config> = {}): Config {
  return {
    port: 3001,
    rpcUrl: 'ws://localhost:9944',
    faucetSeed: '//Alice',
    githubClientId: '',
    githubClientSecret: '',
    sessionSecret: 'test-secret-very-long',
    dbPath: ':memory:',
    dripAmount: BigInt(100) * BigInt(10 ** 12),
    boostAmount: BigInt(1000) * BigInt(10 ** 12),
    cooldownMs: 24 * 60 * 60 * 1000,
    ipRateLimit: 100, // generous default so it doesn't interfere with address tests
    ...override,
  };
}

// Mock ApiPromise — no real chain needed
const mockApi = {
  isReady: Promise.resolve(),
  disconnect: vi.fn(),
} as unknown as ApiPromise;

// ── POST /faucet suite ─────────────────────────────────────────────────────
describe('POST /faucet', () => {
  let app: Express;
  let db: Database;
  let dbPath: string;

  beforeAll(async () => {
    vi.mocked(transferClaw).mockResolvedValue('0xdeadbeef1234567890abcdef');
    vi.mocked(getFaucetBalance).mockResolvedValue('999000000000000000');

    dbPath = tmpDbPath();
    db = initDb(dbPath);
    const config = makeConfig({ dbPath });
    app = await createApp(config, db, mockApi);
  });

  afterAll(() => {
    cleanupDb(dbPath, db);
  });

  it('returns 400 when address is missing', async () => {
    const res = await supertest(app).post('/faucet').send({});
    expect(res.status).toBe(400);
    expect(res.body.error).toBeTruthy();
  });

  it('returns 400 when address is invalid', async () => {
    const res = await supertest(app).post('/faucet').send({ address: 'not-an-ss58-address' });
    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/invalid ss58/i);
  });

  it('returns 400 when address is empty string', async () => {
    const res = await supertest(app).post('/faucet').send({ address: '' });
    expect(res.status).toBe(400);
  });

  it('returns 200 with correct shape for valid address', async () => {
    const res = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '5.5.5.5')
      .send({ address: ALICE });

    expect(res.status).toBe(200);
    expect(res.body.tx_hash).toBeTruthy();
    expect(res.body.amount).toBeTruthy();
    expect(res.body.next_drip_at).toBeTruthy();
  });

  it('returns 100 CLAW (no GitHub session)', async () => {
    const res = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '6.6.6.6')
      .send({ address: BOB });

    expect(res.status).toBe(200);
    expect(res.body.amount).toContain('100');
    expect(res.body.amount).not.toContain('1000');
  });

  it('returns 429 when address is on cooldown', async () => {
    // First request succeeds (Charlie hasn't been used yet)
    const first = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '7.7.7.7')
      .send({ address: CHARLIE });
    expect(first.status).toBe(200);

    // Second request for same address within cooldown → 429
    const second = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '7.7.7.8')
      .send({ address: CHARLIE });
    expect(second.status).toBe(429);
    expect(second.body.error).toMatch(/rate limited/i);
    expect(second.body.next_drip_at).toBeTruthy();
  });

  it('returns 503 when chain transfer throws unavailable error', async () => {
    vi.mocked(transferClaw).mockRejectedValueOnce(new Error('WebSocket connect failed'));

    // Use DAVE — fresh address, no cooldown
    const res = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '9.9.9.9')
      .send({ address: DAVE });

    expect(res.status).toBe(503);
    expect(res.body.error).toMatch(/chain unavailable/i);

    // Reset mock
    vi.mocked(transferClaw).mockResolvedValue('0xdeadbeef1234567890abcdef');
  });

  it('returns 500 when chain transfer fails for unknown reason', async () => {
    vi.mocked(transferClaw).mockRejectedValueOnce(new Error('Unknown transfer failure'));

    // Use EVE — fresh address, no cooldown
    const res = await supertest(app)
      .post('/faucet')
      .set('X-Forwarded-For', '10.10.10.10')
      .send({ address: EVE });

    expect(res.status).toBe(500);
    expect(res.body.error).toMatch(/transfer failed/i);

    vi.mocked(transferClaw).mockResolvedValue('0xdeadbeef1234567890abcdef');
  });
});

// ── IP Rate limit (isolated suite with its own db + strict app) ───────────
describe('POST /faucet IP rate limit', () => {
  let strictApp: Express;
  let db: Database;
  let dbPath: string;

  beforeAll(async () => {
    vi.mocked(transferClaw).mockResolvedValue('0xdeadbeef1234567890abcdef');
    vi.mocked(getFaucetBalance).mockResolvedValue('999000000000000000');

    dbPath = tmpDbPath();
    db = initDb(dbPath);
    // Create app with ipRateLimit = 1 so the second request from same IP is blocked
    const config = makeConfig({ dbPath, ipRateLimit: 1 });
    strictApp = await createApp(config, db, mockApi);
  });

  afterAll(() => {
    cleanupDb(dbPath, db);
  });

  it('returns 429 when IP rate limit exceeded', async () => {
    // Pre-populate: 1 drip already from this IP (hits limit immediately)
    recordDrip(db, ALICE, '8.8.8.8', '100000000000000', '0xprepopulated');

    // Now any request from 8.8.8.8 should be blocked (count=1 >= limit=1)
    const res = await supertest(strictApp)
      .post('/faucet')
      .set('X-Forwarded-For', '8.8.8.8')
      .send({ address: FERDIE });

    expect(res.status).toBe(429);
    expect(res.body.error).toMatch(/too many requests/i);
    expect(res.body.retry_after).toBeGreaterThan(0);
  });

  it('allows requests from different IPs', async () => {
    // 9.9.9.9 has no drips → should pass
    const res = await supertest(strictApp)
      .post('/faucet')
      .set('X-Forwarded-For', '9.9.9.9')
      .send({ address: CHARLIE });

    // Either 200 (success) or 429 (address already on cooldown) — NOT an IP block
    // We just verify it's not blocked by IP rate limit with "too many requests"
    expect(res.status).not.toBe(429);
    // If it is 429, it should be address cooldown, not IP
    if (res.status === 429) {
      expect(res.body.error).not.toMatch(/too many requests/i);
    }
  });
});

// ── GET /status ────────────────────────────────────────────────────────────
describe('GET /status', () => {
  let app: Express;
  let db: Database;
  let dbPath: string;

  beforeAll(async () => {
    vi.mocked(getFaucetBalance).mockResolvedValue('500000000000000');

    dbPath = tmpDbPath();
    db = initDb(dbPath);
    const config = makeConfig({ dbPath });
    app = await createApp(config, db, mockApi);
  });

  afterAll(() => {
    cleanupDb(dbPath, db);
  });

  it('returns 200 with correct shape', async () => {
    const res = await supertest(app).get('/status');
    expect(res.status).toBe(200);
    expect(res.body.balance).toBeTruthy();
    expect(typeof res.body.total_drips).toBe('number');
    expect(typeof res.body.total_amount).toBe('string');
    expect(typeof res.body.unique_addresses).toBe('number');
  });

  it('returns 503 when chain is unavailable', async () => {
    vi.mocked(getFaucetBalance).mockRejectedValueOnce(new Error('Chain down'));
    const res = await supertest(app).get('/status');
    expect(res.status).toBe(503);
    vi.mocked(getFaucetBalance).mockResolvedValue('500000000000000');
  });
});

// ── GET /auth/me ───────────────────────────────────────────────────────────
describe('GET /auth/me', () => {
  let app: Express;
  let db: Database;
  let dbPath: string;

  beforeAll(async () => {
    dbPath = tmpDbPath();
    db = initDb(dbPath);
    const config = makeConfig({ dbPath });
    app = await createApp(config, db, mockApi);
  });

  afterAll(() => {
    cleanupDb(dbPath, db);
  });

  it('returns authenticated: false when no session', async () => {
    const res = await supertest(app).get('/auth/me');
    expect(res.status).toBe(200);
    expect(res.body.authenticated).toBe(false);
  });
});
