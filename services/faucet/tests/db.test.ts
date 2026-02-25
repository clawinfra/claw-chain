/**
 * Unit tests for src/db.ts
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { initDb, recordDrip, getLastDrip, getIpRequestCount, getStats } from '../src/db.js';
import type { Database } from 'better-sqlite3';
import { tmpdir } from 'os';
import { join } from 'path';
import { unlinkSync, existsSync } from 'fs';

function tmpDbPath(): string {
  return join(tmpdir(), `faucet-test-${Date.now()}-${Math.random().toString(36).slice(2)}.db`);
}

function cleanup(dbPath: string, db: Database): void {
  db.close();
  if (existsSync(dbPath)) unlinkSync(dbPath);
  const walPath = dbPath + '-wal';
  const shmPath = dbPath + '-shm';
  if (existsSync(walPath)) unlinkSync(walPath);
  if (existsSync(shmPath)) unlinkSync(shmPath);
}

describe('initDb', () => {
  it('creates the drips table on a fresh database', () => {
    const path = tmpDbPath();
    const db = initDb(path);
    try {
      const tables = db.prepare(
        `SELECT name FROM sqlite_master WHERE type='table' AND name='drips'`
      ).all();
      expect(tables).toHaveLength(1);
    } finally {
      cleanup(path, db);
    }
  });

  it('creates indexes on a fresh database', () => {
    const path = tmpDbPath();
    const db = initDb(path);
    try {
      const indexes = db.prepare(
        `SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='drips'`
      ).all() as { name: string }[];
      const names = indexes.map(i => i.name);
      expect(names).toContain('idx_drips_address');
      expect(names).toContain('idx_drips_ip_created');
    } finally {
      cleanup(path, db);
    }
  });

  it('is idempotent — calling twice does not throw', () => {
    const path = tmpDbPath();
    const db = initDb(path);
    db.close();
    const db2 = initDb(path);
    try {
      const tables = db2.prepare(
        `SELECT name FROM sqlite_master WHERE type='table' AND name='drips'`
      ).all();
      expect(tables).toHaveLength(1);
    } finally {
      cleanup(path, db2);
    }
  });
});

describe('recordDrip + getLastDrip', () => {
  let db: Database;
  let dbPath: string;

  beforeEach(() => {
    dbPath = tmpDbPath();
    db = initDb(dbPath);
  });

  afterEach(() => {
    cleanup(dbPath, db);
  });

  it('returns undefined when no drip exists for address', () => {
    const result = getLastDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
    expect(result).toBeUndefined();
  });

  it('records a drip and retrieves it', () => {
    const address = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    recordDrip(db, address, '127.0.0.1', '100000000000000', '0xabc123', 'testuser');
    const result = getLastDrip(db, address);
    expect(result).toBeDefined();
    expect(result!.amount).toBe('100000000000000');
    expect(result!.created_at).toBeTruthy();
  });

  it('returns the most recent drip when multiple exist', () => {
    const address = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    recordDrip(db, address, '127.0.0.1', '100000000000000', '0xfirst');
    recordDrip(db, address, '127.0.0.2', '1000000000000000', '0xsecond');
    const result = getLastDrip(db, address);
    expect(result!.amount).toBe('1000000000000000');
  });

  it('does not return drips for a different address', () => {
    recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', '127.0.0.1', '100', '0xabc');
    const result = getLastDrip(db, '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty');
    expect(result).toBeUndefined();
  });
});

describe('getIpRequestCount', () => {
  let db: Database;
  let dbPath: string;

  beforeEach(() => {
    dbPath = tmpDbPath();
    db = initDb(dbPath);
  });

  afterEach(() => {
    cleanup(dbPath, db);
  });

  it('returns 0 when no requests exist for IP', () => {
    const count = getIpRequestCount(db, '1.2.3.4', 3_600_000);
    expect(count).toBe(0);
  });

  it('counts requests within the window', () => {
    const ip = '10.0.0.1';
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    recordDrip(db, addr, ip, '100', '0xaaa');
    recordDrip(db, addr, ip, '100', '0xbbb');
    const count = getIpRequestCount(db, ip, 3_600_000);
    expect(count).toBe(2);
  });

  it('does not count requests outside the window', () => {
    const ip = '10.0.0.2';
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    // Insert an old record by manipulating created_at directly
    db.prepare(
      `INSERT INTO drips (address, ip, amount, tx_hash, created_at)
       VALUES (?, ?, ?, ?, datetime('now', '-2 hours'))`
    ).run(addr, ip, '100', '0xold');
    // Window is 1 hour
    const count = getIpRequestCount(db, ip, 3_600_000);
    expect(count).toBe(0);
  });

  it('counts only within window when mixed old and new', () => {
    const ip = '10.0.0.3';
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    // Old record (outside window)
    db.prepare(
      `INSERT INTO drips (address, ip, amount, tx_hash, created_at)
       VALUES (?, ?, ?, ?, datetime('now', '-2 hours'))`
    ).run(addr, ip, '100', '0xold');
    // Recent record (within window)
    recordDrip(db, addr, ip, '100', '0xnew');
    const count = getIpRequestCount(db, ip, 3_600_000);
    expect(count).toBe(1);
  });
});

describe('getStats', () => {
  let db: Database;
  let dbPath: string;

  beforeEach(() => {
    dbPath = tmpDbPath();
    db = initDb(dbPath);
  });

  afterEach(() => {
    cleanup(dbPath, db);
  });

  it('returns zeros for an empty database', () => {
    const stats = getStats(db);
    expect(stats.total_drips).toBe(0);
    expect(stats.total_amount).toBe('0');
    expect(stats.unique_addresses).toBe(0);
  });

  it('counts total drips correctly', () => {
    const addr1 = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    const addr2 = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';
    recordDrip(db, addr1, '1.1.1.1', '100000000000000', '0x1');
    recordDrip(db, addr2, '2.2.2.2', '100000000000000', '0x2');
    const stats = getStats(db);
    expect(stats.total_drips).toBe(2);
  });

  it('counts unique addresses correctly', () => {
    const addr = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
    recordDrip(db, addr, '1.1.1.1', '100000000000000', '0x1');
    recordDrip(db, addr, '2.2.2.2', '100000000000000', '0x2'); // same address, different IP
    const stats = getStats(db);
    expect(stats.unique_addresses).toBe(1);
    expect(stats.total_drips).toBe(2);
  });

  it('sums total_amount correctly', () => {
    recordDrip(db, '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', '1.1.1.1', '100000000000000', '0x1');
    recordDrip(db, '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', '2.2.2.2', '1000000000000000', '0x2');
    const stats = getStats(db);
    expect(BigInt(stats.total_amount)).toBe(BigInt('100000000000000') + BigInt('1000000000000000'));
  });
});
