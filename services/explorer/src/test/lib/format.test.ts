import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { shortenHash, formatTimestamp, formatAddress, formatBalance } from '@/lib/format';

describe('shortenHash', () => {
  it('returns the hash unchanged if short enough', () => {
    const short = '0x1234';
    expect(shortenHash(short)).toBe(short);
  });

  it('shortens a full 32-byte hash', () => {
    const hash = '0x' + 'a'.repeat(64);
    const result = shortenHash(hash);
    expect(result).toContain('...');
    expect(result.startsWith('0xaaaa')).toBe(true);
    expect(result.endsWith('aaaa')).toBe(true);
  });

  it('handles empty string', () => {
    expect(shortenHash('')).toBe('');
  });

  it('respects custom prefix and suffix lengths', () => {
    const hash = '0x' + '1234567890abcdef'.repeat(4);
    const result = shortenHash(hash, 4, 4);
    expect(result).toContain('...');
  });
});

describe('formatTimestamp', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2024-01-01T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns Unknown for zero timestamp', () => {
    expect(formatTimestamp(0)).toBe('Unknown');
  });

  it('returns Unknown for negative timestamp', () => {
    expect(formatTimestamp(-1)).toBe('Unknown');
  });

  it('formats seconds ago correctly', () => {
    const now = Date.now();
    const ts = now - 30_000; // 30 seconds ago in ms
    expect(formatTimestamp(ts)).toBe('30s ago');
  });

  it('formats minutes ago correctly', () => {
    const now = Date.now();
    const ts = now - 5 * 60_000; // 5 minutes ago in ms
    expect(formatTimestamp(ts)).toBe('5m ago');
  });

  it('formats hours ago correctly', () => {
    const now = Date.now();
    const ts = now - 3 * 3600_000; // 3 hours ago in ms
    expect(formatTimestamp(ts)).toBe('3h ago');
  });

  it('formats days ago correctly', () => {
    const now = Date.now();
    const ts = now - 2 * 86400_000; // 2 days ago in ms
    expect(formatTimestamp(ts)).toBe('2d ago');
  });

  it('handles substrate-style timestamps (large ms values)', () => {
    const now = Date.now();
    const ts = now - 45_000; // 45 seconds ago
    expect(formatTimestamp(ts)).toBe('45s ago');
  });

  it('returns ISO string when relative=false', () => {
    const ts = new Date('2024-01-01T10:00:00Z').getTime();
    const result = formatTimestamp(ts, false);
    expect(result).toBe('2024-01-01T10:00:00.000Z');
  });

  it('handles small second-based timestamps (converts to ms)', () => {
    const now = Date.now();
    const tsSeconds = Math.floor((now - 10_000) / 1000); // 10 seconds ago in seconds
    // Small value → treated as seconds
    expect(formatTimestamp(tsSeconds)).toBe('10s ago');
  });
});

describe('formatAddress', () => {
  it('returns Unknown for null', () => {
    expect(formatAddress(null)).toBe('Unknown');
  });

  it('returns Unknown for undefined', () => {
    expect(formatAddress(undefined)).toBe('Unknown');
  });

  it('returns short address unchanged', () => {
    expect(formatAddress('5ABC')).toBe('5ABC');
  });

  it('shortens a long SS58 address', () => {
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    const result = formatAddress(addr);
    expect(result).toContain('...');
    expect(result.startsWith('5Grwva')).toBe(true);
  });

  it('respects custom chars parameter', () => {
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    const result = formatAddress(addr, 4);
    expect(result.startsWith('5Grw')).toBe(true);
    expect(result.endsWith('utQY')).toBe(true); // 4-char suffix
    expect(result).toContain('...');
  });
});

describe('formatBalance', () => {
  it('formats zero balance', () => {
    expect(formatBalance(0)).toBe('0.0000 CLAW');
  });

  it('formats a positive balance', () => {
    // 1 CLAW = 10^18 planck
    const oneClaw = BigInt('1000000000000000000'); // 10^18
    expect(formatBalance(oneClaw)).toBe('1.0000 CLAW');
  });

  it('formats fractional balance', () => {
    const halfClaw = BigInt('500000000000000000'); // 5 * 10^17
    expect(formatBalance(halfClaw)).toBe('0.5000 CLAW');
  });

  it('handles string input', () => {
    expect(formatBalance('0')).toBe('0.0000 CLAW');
  });

  it('handles invalid input gracefully', () => {
    expect(formatBalance('not-a-number')).toBe('0.0000 CLAW');
  });

  it('uses custom symbol', () => {
    expect(formatBalance(0, 18, 'DOT')).toBe('0.0000 DOT');
  });

  it('handles large balance', () => {
    const large = BigInt('1000000000000000000000'); // 1000 * 10^18
    expect(formatBalance(large)).toBe('1000.0000 CLAW');
  });
});
