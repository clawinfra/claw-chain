/**
 * In-memory rate limiter for the faucet.
 *
 * Tracks:
 *   - Per-address cooldown: one drip per address per COOLDOWN_HOURS
 *   - Per-IP hourly limit: max IP_RATE_LIMIT requests per IP per hour
 *
 * All state is in-process Maps — no database required.
 * State is periodically cleaned up to prevent unbounded growth.
 */

export interface DripRecord {
  /** ISO timestamp of the drip */
  timestamp: string;
  /** tx hash returned by the chain */
  txHash: string;
}

export interface RateLimitStore {
  /** address → last drip record */
  addressDrips: Map<string, DripRecord>;
  /** ip → list of request timestamps (ms) */
  ipRequests: Map<string, number[]>;
}

export interface RateLimitConfig {
  /** Duration in ms before an address can drip again (default: 24h) */
  cooldownMs: number;
  /** Max requests per IP per hour (default: 10) */
  ipRateLimit: number;
}

const DEFAULT_CONFIG: RateLimitConfig = {
  cooldownMs: 24 * 60 * 60 * 1000,
  ipRateLimit: 10,
};

export class RateLimiter {
  private readonly store: RateLimitStore;
  private readonly config: RateLimitConfig;
  private cleanupInterval: ReturnType<typeof setInterval> | null = null;

  constructor(config: Partial<RateLimitConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.store = {
      addressDrips: new Map(),
      ipRequests: new Map(),
    };
    // Clean up stale entries every 10 minutes
    this.cleanupInterval = setInterval(() => this.cleanup(), 10 * 60 * 1000);
    // Allow process to exit even if interval is active
    if (this.cleanupInterval.unref) {
      this.cleanupInterval.unref();
    }
  }

  /**
   * Check if an address is within the cooldown window.
   * Returns null if allowed, or the ISO next-drip-at timestamp if blocked.
   */
  checkAddressCooldown(address: string): { blocked: true; nextDripAt: string } | { blocked: false } {
    const record = this.store.addressDrips.get(address);
    if (!record) return { blocked: false };

    const lastAt = new Date(record.timestamp).getTime();
    const elapsed = Date.now() - lastAt;

    if (elapsed < this.config.cooldownMs) {
      const nextDripAt = new Date(lastAt + this.config.cooldownMs).toISOString();
      return { blocked: true, nextDripAt };
    }
    return { blocked: false };
  }

  /**
   * Check if an IP has exceeded the hourly rate limit.
   * Returns null if allowed, or retry_after seconds if blocked.
   */
  checkIpRateLimit(ip: string): { blocked: true; retryAfter: number } | { blocked: false } {
    const windowMs = 60 * 60 * 1000; // 1 hour
    const now = Date.now();
    const cutoff = now - windowMs;

    const requests = (this.store.ipRequests.get(ip) ?? []).filter((t) => t > cutoff);
    this.store.ipRequests.set(ip, requests);

    if (requests.length >= this.config.ipRateLimit) {
      const retryAfter = Math.ceil(windowMs / 1000);
      return { blocked: true, retryAfter };
    }
    return { blocked: false };
  }

  /**
   * Record a successful drip for an address and IP.
   */
  recordDrip(address: string, ip: string, txHash: string): void {
    const timestamp = new Date().toISOString();
    this.store.addressDrips.set(address, { timestamp, txHash });

    const requests = this.store.ipRequests.get(ip) ?? [];
    requests.push(Date.now());
    this.store.ipRequests.set(ip, requests);
  }

  /**
   * Get the last drip record for an address, or undefined if none.
   */
  getLastDrip(address: string): DripRecord | undefined {
    return this.store.addressDrips.get(address);
  }

  /**
   * Get total number of tracked addresses (for /status).
   */
  getTotalDrips(): number {
    return this.store.addressDrips.size;
  }

  /**
   * Remove stale IP request entries to prevent memory growth.
   */
  private cleanup(): void {
    const windowMs = 60 * 60 * 1000;
    const cutoff = Date.now() - windowMs;

    for (const [ip, requests] of this.store.ipRequests.entries()) {
      const fresh = requests.filter((t) => t > cutoff);
      if (fresh.length === 0) {
        this.store.ipRequests.delete(ip);
      } else {
        this.store.ipRequests.set(ip, fresh);
      }
    }
  }

  /**
   * Stop the cleanup interval (call on shutdown).
   */
  destroy(): void {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
  }
}
