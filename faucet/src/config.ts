/**
 * Config loader — reads environment variables (from .env or process.env)
 * and validates required fields.
 */

import { readFileSync } from 'fs';
import { join } from 'path';

export interface Config {
  /** HTTP port to listen on */
  port: number;
  /** WebSocket RPC URL of the ClawChain node */
  rpcUrl: string;
  /** Faucet account seed phrase or dev URI */
  faucetSeed: string;
  /** Amount in whole CLAW tokens per drip */
  dripAmountClaw: number;
  /** Amount in planck (dripAmountClaw × 10^12) */
  dripAmountPlanck: bigint;
  /** Cooldown between drips per address in ms */
  cooldownMs: number;
  /** Max drip requests per IP per hour */
  ipRateLimit: number;
  /** Log level */
  logLevel: string;
}

function loadDotEnv(): void {
  try {
    const envPath = join(process.cwd(), '.env');
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
    // .env not present — fall through to process.env
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

/** CLAW has 12 decimal places (same as Polkadot DOT) */
const PLANCK_PER_CLAW = BigInt(10 ** 12);

export function loadConfig(): Config {
  loadDotEnv();

  const dripAmountClaw = parseInt(optionalEnv('DRIP_AMOUNT_CLAW', '1000'), 10);
  const cooldownHours = parseFloat(optionalEnv('COOLDOWN_HOURS', '24'));

  return {
    port: parseInt(optionalEnv('PORT', '3001'), 10),
    rpcUrl: requireEnv('RPC_URL'),
    faucetSeed: requireEnv('FAUCET_SEED'),
    dripAmountClaw,
    dripAmountPlanck: BigInt(dripAmountClaw) * PLANCK_PER_CLAW,
    cooldownMs: Math.round(cooldownHours * 60 * 60 * 1000),
    ipRateLimit: parseInt(optionalEnv('IP_RATE_LIMIT', '10'), 10),
    logLevel: optionalEnv('LOG_LEVEL', 'info'),
  };
}
