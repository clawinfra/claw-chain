/**
 * Unit tests for src/services/rateLimit.ts
 */

import { RateLimiter } from '../src/services/rateLimit';

const ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
const BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';

describe('RateLimiter — address cooldown', () => {
  it('allows a fresh address (not seen before)', () => {
    const limiter = new RateLimiter({ cooldownMs: 1000 });
    const result = limiter.checkAddressCooldown(ALICE);
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });

  it('blocks an address within cooldown window', () => {
    const limiter = new RateLimiter({ cooldownMs: 24 * 60 * 60 * 1000 });
    limiter.recordDrip(ALICE, '1.2.3.4', '0xabc');

    const result = limiter.checkAddressCooldown(ALICE);
    expect(result.blocked).toBe(true);
    if (result.blocked) {
      expect(result.nextDripAt).toBeTruthy();
      // nextDripAt should be in the future
      expect(new Date(result.nextDripAt).getTime()).toBeGreaterThan(Date.now());
    }
    limiter.destroy();
  });

  it('allows an address after cooldown expires', () => {
    // 1ms cooldown — expires immediately
    const limiter = new RateLimiter({ cooldownMs: 1 });

    // Manually inject a stale record
    const pastTime = new Date(Date.now() - 100).toISOString();
    (limiter as unknown as { store: { addressDrips: Map<string, { timestamp: string; txHash: string }> } })
      .store.addressDrips.set(ALICE, { timestamp: pastTime, txHash: '0xold' });

    const result = limiter.checkAddressCooldown(ALICE);
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });

  it('does not block a different address', () => {
    const limiter = new RateLimiter({ cooldownMs: 24 * 60 * 60 * 1000 });
    limiter.recordDrip(ALICE, '1.2.3.4', '0xabc');

    const result = limiter.checkAddressCooldown(BOB);
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });

  it('returns nextDripAt approximately cooldown ms in the future', () => {
    const cooldownMs = 24 * 60 * 60 * 1000;
    const limiter = new RateLimiter({ cooldownMs });
    limiter.recordDrip(ALICE, '1.2.3.4', '0xabc');

    const before = Date.now();
    const result = limiter.checkAddressCooldown(ALICE);
    expect(result.blocked).toBe(true);
    if (result.blocked) {
      const nextAt = new Date(result.nextDripAt).getTime();
      expect(nextAt).toBeGreaterThanOrEqual(before + cooldownMs - 100);
      expect(nextAt).toBeLessThanOrEqual(before + cooldownMs + 100);
    }
    limiter.destroy();
  });
});

describe('RateLimiter — IP rate limit', () => {
  it('allows requests under the limit', () => {
    const limiter = new RateLimiter({ ipRateLimit: 5 });
    // 4 requests from same IP
    for (let i = 0; i < 4; i++) {
      limiter.recordDrip(`addr${i}`, '5.5.5.5', `0x${i}`);
    }
    const result = limiter.checkIpRateLimit('5.5.5.5');
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });

  it('blocks at the rate limit boundary', () => {
    const limiter = new RateLimiter({ ipRateLimit: 3 });
    for (let i = 0; i < 3; i++) {
      limiter.recordDrip(`addr${i}`, '6.6.6.6', `0x${i}`);
    }
    const result = limiter.checkIpRateLimit('6.6.6.6');
    expect(result.blocked).toBe(true);
    if (result.blocked) {
      expect(result.retryAfter).toBeGreaterThan(0);
    }
    limiter.destroy();
  });

  it('allows requests from a different IP', () => {
    const limiter = new RateLimiter({ ipRateLimit: 1 });
    limiter.recordDrip(ALICE, '7.7.7.7', '0xabc');

    // 8.8.8.8 is fresh
    const result = limiter.checkIpRateLimit('8.8.8.8');
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });

  it('returns retryAfter approximately 3600 seconds', () => {
    const limiter = new RateLimiter({ ipRateLimit: 1 });
    limiter.recordDrip(ALICE, '9.9.9.9', '0xabc');
    const result = limiter.checkIpRateLimit('9.9.9.9');
    expect(result.blocked).toBe(true);
    if (result.blocked) {
      // Should be ~3600 seconds (1 hour window)
      expect(result.retryAfter).toBeGreaterThanOrEqual(3599);
      expect(result.retryAfter).toBeLessThanOrEqual(3601);
    }
    limiter.destroy();
  });

  it('does not count requests outside the 1-hour window', () => {
    const limiter = new RateLimiter({ ipRateLimit: 2 });
    const ip = '10.0.0.1';

    // Manually inject stale requests (> 1 hour ago)
    const staleTime = Date.now() - 2 * 60 * 60 * 1000;
    (limiter as unknown as { store: { ipRequests: Map<string, number[]> } })
      .store.ipRequests.set(ip, [staleTime, staleTime]);

    // Should not be blocked (stale requests don't count)
    const result = limiter.checkIpRateLimit(ip);
    expect(result.blocked).toBe(false);
    limiter.destroy();
  });
});

describe('RateLimiter — recordDrip', () => {
  it('records drip and makes it retrievable via getLastDrip', () => {
    const limiter = new RateLimiter();
    limiter.recordDrip(ALICE, '1.2.3.4', '0xhash123');
    const record = limiter.getLastDrip(ALICE);
    expect(record).toBeDefined();
    expect(record!.txHash).toBe('0xhash123');
    expect(record!.timestamp).toBeTruthy();
    limiter.destroy();
  });

  it('overrides previous drip for same address', () => {
    const limiter = new RateLimiter();
    limiter.recordDrip(ALICE, '1.2.3.4', '0xfirst');
    // Manually expire the cooldown
    const pastTime = new Date(Date.now() - 25 * 60 * 60 * 1000).toISOString();
    (limiter as unknown as { store: { addressDrips: Map<string, { timestamp: string; txHash: string }> } })
      .store.addressDrips.set(ALICE, { timestamp: pastTime, txHash: '0xfirst' });

    limiter.recordDrip(ALICE, '1.2.3.4', '0xsecond');
    const record = limiter.getLastDrip(ALICE);
    expect(record!.txHash).toBe('0xsecond');
    limiter.destroy();
  });

  it('increments getTotalDrips', () => {
    const limiter = new RateLimiter();
    expect(limiter.getTotalDrips()).toBe(0);
    limiter.recordDrip(ALICE, '1.2.3.4', '0xabc');
    expect(limiter.getTotalDrips()).toBe(1);
    limiter.recordDrip(BOB, '1.2.3.4', '0xdef');
    expect(limiter.getTotalDrips()).toBe(2);
    limiter.destroy();
  });
});

describe('RateLimiter — destroy', () => {
  it('clears the cleanup interval without throwing', () => {
    const limiter = new RateLimiter();
    expect(() => limiter.destroy()).not.toThrow();
    // Calling destroy twice is safe
    expect(() => limiter.destroy()).not.toThrow();
  });
});
