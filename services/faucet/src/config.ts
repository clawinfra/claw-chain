/**
 * Config loader — reads .env / process.env and validates required fields.
 */

import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

export interface Config {
  port: number;
  rpcUrl: string;
  faucetSeed: string;
  githubClientId: string;
  githubClientSecret: string;
  sessionSecret: string;
  dbPath: string;
  /** 100 CLAW in planck (12 decimal places) */
  dripAmount: bigint;
  /** 1000 CLAW in planck */
  boostAmount: bigint;
  /** 24 hours in ms */
  cooldownMs: number;
  /** Max requests per hour per IP */
  ipRateLimit: number;
}

function loadDotEnv(): void {
  try {
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = dirname(__filename);
    const envPath = join(__dirname, '..', '.env');
    const lines = readFileSync(envPath, 'utf-8').split('\n');
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const eqIdx = trimmed.indexOf('=');
      if (eqIdx === -1) continue;
      const k = trimmed.slice(0, eqIdx).trim();
      const v = trimmed.slice(eqIdx + 1).trim();
      if (k && !(k in process.env)) {
        process.env[k] = v;
      }
    }
  } catch {
    // .env not found or not readable — fall through to process.env
  }
}

function requireEnv(key: string): string {
  const value = process.env[key];
  if (!value) {
    throw new Error(`Missing required environment variable: ${key}`);
  }
  return value;
}

function optionalEnv(key: string, fallback: string): string {
  return process.env[key] ?? fallback;
}

export function loadConfig(): Config {
  loadDotEnv();

  return {
    port: parseInt(optionalEnv('PORT', '3000'), 10),
    rpcUrl: requireEnv('RPC_URL'),
    faucetSeed: requireEnv('FAUCET_SEED'),
    githubClientId: optionalEnv('GITHUB_CLIENT_ID', ''),
    githubClientSecret: optionalEnv('GITHUB_CLIENT_SECRET', ''),
    sessionSecret: optionalEnv('SESSION_SECRET', 'change-me'),
    dbPath: optionalEnv('DB_PATH', './faucet.db'),
    // 100 CLAW × 10^12 planck per CLAW
    dripAmount: BigInt(100) * BigInt(10 ** 12),
    // 1000 CLAW × 10^12 planck per CLAW
    boostAmount: BigInt(1000) * BigInt(10 ** 12),
    cooldownMs: 24 * 60 * 60 * 1000,
    ipRateLimit: 10,
  };
}
