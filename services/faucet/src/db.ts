/**
 * SQLite database setup and query helpers using better-sqlite3.
 * All operations are synchronous for simplicity and performance.
 */

import Database from 'better-sqlite3';

export { Database };

export function initDb(dbPath: string): Database.Database {
  const db = new Database(dbPath);

  // Enable WAL mode for better concurrency
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');

  db.exec(`
    CREATE TABLE IF NOT EXISTS drips (
      id         INTEGER PRIMARY KEY AUTOINCREMENT,
      address    TEXT NOT NULL,
      ip         TEXT NOT NULL,
      amount     TEXT NOT NULL,
      tx_hash    TEXT NOT NULL,
      github_user TEXT,
      created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_drips_address
      ON drips(address);

    CREATE INDEX IF NOT EXISTS idx_drips_ip_created
      ON drips(ip, created_at);
  `);

  return db;
}

export interface DripRecord {
  created_at: string;
  amount: string;
}

/**
 * Record a successful drip to the database.
 */
export function recordDrip(
  db: Database.Database,
  address: string,
  ip: string,
  amount: string,
  txHash: string,
  githubUser?: string,
): void {
  const stmt = db.prepare(`
    INSERT INTO drips (address, ip, amount, tx_hash, github_user)
    VALUES (?, ?, ?, ?, ?)
  `);
  stmt.run(address, ip, amount, txHash, githubUser ?? null);
}

/**
 * Get the most recent drip for an address, or undefined if none.
 */
export function getLastDrip(
  db: Database.Database,
  address: string,
): DripRecord | undefined {
  const stmt = db.prepare(`
    SELECT created_at, amount
    FROM drips
    WHERE address = ?
    ORDER BY id DESC
    LIMIT 1
  `);
  return stmt.get(address) as DripRecord | undefined;
}

/**
 * Count how many requests an IP has made within the given time window (ms).
 */
export function getIpRequestCount(
  db: Database.Database,
  ip: string,
  windowMs: number,
): number {
  const cutoff = new Date(Date.now() - windowMs).toISOString().replace('T', ' ').slice(0, 19);
  const stmt = db.prepare(`
    SELECT COUNT(*) as cnt
    FROM drips
    WHERE ip = ?
      AND created_at >= ?
  `);
  const row = stmt.get(ip, cutoff) as { cnt: number };
  return row.cnt;
}

export interface FaucetStats {
  total_drips: number;
  total_amount: string;
  unique_addresses: number;
}

/**
 * Aggregate statistics for the /status endpoint.
 */
export function getStats(db: Database.Database): FaucetStats {
  const stmt = db.prepare(`
    SELECT
      COUNT(*)                   AS total_drips,
      COALESCE(SUM(CAST(amount AS INTEGER)), 0) AS total_amount_planck,
      COUNT(DISTINCT address)    AS unique_addresses
    FROM drips
  `);
  const row = stmt.get() as {
    total_drips: number;
    total_amount_planck: number;
    unique_addresses: number;
  };
  return {
    total_drips: row.total_drips,
    total_amount: row.total_amount_planck.toString(),
    unique_addresses: row.unique_addresses,
  };
}
