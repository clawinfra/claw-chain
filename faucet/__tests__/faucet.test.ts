/**
 * Integration tests for the faucet API routes.
 * Uses supertest + jest mocks — no real chain required.
 */

import request from 'supertest';
import express from 'express';
import cors from 'cors';
import type { ApiPromise } from '@polkadot/api';

// Mock the chain module before importing routes
jest.mock('../src/services/chain', () => ({
  connectChain: jest.fn(),
  transferClaw: jest.fn().mockResolvedValue('0xdeadbeef1234567890abcdef'),
  getFaucetBalance: jest.fn().mockResolvedValue('1000000000000000000'),
  getFaucetAddress: jest.fn().mockReturnValue('5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY'),
}));

import { transferClaw, getFaucetBalance } from '../src/services/chain';
import { createFaucetRouter } from '../src/routes/faucet';
import { RateLimiter } from '../src/services/rateLimit';
import type { Config } from '../src/config';

// Substrate dev accounts (prefix 42)
const ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';
const CHARLIE = '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y';
const DAVE = '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy';
const EVE = '5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw';

const mockApi = {
  isReady: Promise.resolve(),
  disconnect: jest.fn(),
} as unknown as ApiPromise;

function makeConfig(override: Partial<Config> = {}): Config {
  return {
    port: 3001,
    rpcUrl: 'ws://localhost:9944',
    faucetSeed: '//Alice',
    dripAmountClaw: 1000,
    dripAmountPlanck: BigInt(1000) * BigInt(10 ** 12),
    cooldownMs: 24 * 60 * 60 * 1000,
    ipRateLimit: 100,
    logLevel: 'silent',
    ...override,
  };
}

function buildApp(config: Config, rateLimiter: RateLimiter): express.Application {
  const app = express();
  app.set('trust proxy', 1);
  app.use(cors());
  app.use(express.json());
  app.use('/', createFaucetRouter({ api: mockApi, config, rateLimiter }));
  return app;
}

// ── POST /drip ─────────────────────────────────────────────────────────────
describe('POST /drip', () => {
  let app: express.Application;
  let rateLimiter: RateLimiter;

  beforeEach(() => {
    jest.mocked(transferClaw).mockResolvedValue('0xdeadbeef1234567890abcdef');
    jest.mocked(getFaucetBalance).mockResolvedValue('1000000000000000000');
    rateLimiter = new RateLimiter({ cooldownMs: 24 * 60 * 60 * 1000, ipRateLimit: 100 });
    app = buildApp(makeConfig(), rateLimiter);
  });

  afterEach(() => {
    rateLimiter.destroy();
  });

  it('returns 400 when address is missing', async () => {
    const res = await request(app).post('/drip').send({});
    expect(res.status).toBe(400);
    expect(res.body.error).toBeTruthy();
  });

  it('returns 400 when address is an invalid SS58', async () => {
    const res = await request(app).post('/drip').send({ address: 'not-an-address' });
    expect(res.status).toBe(400);
    expect(res.body.error).toMatch(/invalid ss58/i);
  });

  it('returns 400 when address is empty string', async () => {
    const res = await request(app).post('/drip').send({ address: '' });
    expect(res.status).toBe(400);
  });

  it('returns 400 when address is whitespace only', async () => {
    const res = await request(app).post('/drip').send({ address: '   ' });
    expect(res.status).toBe(400);
  });

  it('returns 200 with correct shape for valid address', async () => {
    const res = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '5.5.5.5')
      .send({ address: ALICE });

    expect(res.status).toBe(200);
    expect(res.body.tx_hash).toBeTruthy();
    expect(res.body.amount).toBe('1000 CLAW');
    expect(res.body.next_drip_at).toBeTruthy();
    expect(new Date(res.body.next_drip_at).getTime()).toBeGreaterThan(Date.now());
  });

  it('returns 429 when address is on cooldown', async () => {
    // First drip succeeds
    const first = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '6.6.6.6')
      .send({ address: BOB });
    expect(first.status).toBe(200);

    // Second drip for same address within cooldown → 429
    const second = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '7.7.7.7')
      .send({ address: BOB });
    expect(second.status).toBe(429);
    expect(second.body.error).toMatch(/cooldown/i);
    expect(second.body.next_drip_at).toBeTruthy();
  });

  it('returns 429 when IP rate limit is exceeded', async () => {
    const strictRateLimiter = new RateLimiter({ ipRateLimit: 1, cooldownMs: 24 * 60 * 60 * 1000 });
    const strictApp = buildApp(makeConfig({ ipRateLimit: 1 }), strictRateLimiter);

    // First request from this IP
    const first = await request(strictApp)
      .post('/drip')
      .set('X-Forwarded-For', '9.9.9.9')
      .send({ address: CHARLIE });
    expect(first.status).toBe(200);

    // Second request from same IP → IP rate limited
    const second = await request(strictApp)
      .post('/drip')
      .set('X-Forwarded-For', '9.9.9.9')
      .send({ address: DAVE });
    expect(second.status).toBe(429);
    expect(second.body.error).toMatch(/too many requests/i);
    expect(second.body.retry_after).toBeGreaterThan(0);

    strictRateLimiter.destroy();
  });

  it('returns 503 when chain transfer throws connect error', async () => {
    jest.mocked(transferClaw).mockRejectedValueOnce(new Error('WebSocket connect failed'));

    const res = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '11.11.11.11')
      .send({ address: DAVE });

    expect(res.status).toBe(503);
    expect(res.body.error).toMatch(/chain unavailable/i);
  });

  it('returns 503 when faucet is depleted', async () => {
    jest.mocked(transferClaw).mockRejectedValueOnce(new Error('Faucet insufficient balance'));

    const res = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '12.12.12.12')
      .send({ address: EVE });

    expect(res.status).toBe(503);
    expect(res.body.error).toMatch(/depleted/i);
  });

  it('returns 500 for unknown transfer failures', async () => {
    jest.mocked(transferClaw).mockRejectedValueOnce(new Error('Unknown weird error XYZ'));

    const res = await request(app)
      .post('/drip')
      .set('X-Forwarded-For', '13.13.13.13')
      .send({ address: CHARLIE });

    expect(res.status).toBe(500);
    expect(res.body.error).toMatch(/transfer failed/i);
  });
});

// ── GET /status ────────────────────────────────────────────────────────────
describe('GET /status', () => {
  let app: express.Application;
  let rateLimiter: RateLimiter;

  beforeEach(() => {
    jest.mocked(getFaucetBalance).mockResolvedValue('500000000000000000');
    rateLimiter = new RateLimiter();
    app = buildApp(makeConfig(), rateLimiter);
  });

  afterEach(() => {
    rateLimiter.destroy();
  });

  it('returns 200 with correct shape', async () => {
    const res = await request(app).get('/status');
    expect(res.status).toBe(200);
    expect(res.body.status).toBe('ok');
    expect(res.body.faucet_address).toBeTruthy();
    expect(res.body.balance_planck).toBeTruthy();
    expect(res.body.balance_claw).toBeTruthy();
    expect(typeof res.body.drip_amount_claw).toBe('number');
    expect(res.body.drip_amount_claw).toBe(1000);
    expect(typeof res.body.total_drips).toBe('number');
  });

  it('returns 503 when chain is unavailable', async () => {
    jest.mocked(getFaucetBalance).mockRejectedValueOnce(new Error('Chain down'));
    const res = await request(app).get('/status');
    expect(res.status).toBe(503);
    expect(res.body.error).toMatch(/chain unavailable/i);
  });
});

// ── GET /balance ───────────────────────────────────────────────────────────
describe('GET /balance', () => {
  let app: express.Application;
  let rateLimiter: RateLimiter;

  beforeEach(() => {
    jest.mocked(getFaucetBalance).mockResolvedValue('999000000000000000');
    rateLimiter = new RateLimiter();
    app = buildApp(makeConfig(), rateLimiter);
  });

  afterEach(() => {
    rateLimiter.destroy();
  });

  it('returns 200 with balance fields', async () => {
    const res = await request(app).get('/balance');
    expect(res.status).toBe(200);
    expect(res.body.address).toBeTruthy();
    expect(res.body.balance_planck).toBeTruthy();
    expect(res.body.balance_claw).toBeTruthy();
  });

  it('returns 503 when chain is unavailable', async () => {
    jest.mocked(getFaucetBalance).mockRejectedValueOnce(new Error('RPC down'));
    const res = await request(app).get('/balance');
    expect(res.status).toBe(503);
    expect(res.body.error).toMatch(/chain unavailable/i);
  });
});
