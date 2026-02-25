/**
 * Shorten a hex hash to a readable form: 0x1234...5678
 */
export function shortenHash(hash: string, prefixLen = 6, suffixLen = 4): string {
  if (!hash || hash.length <= prefixLen + suffixLen + 2) return hash;
  const prefix = hash.startsWith('0x') ? hash.slice(0, prefixLen + 2) : hash.slice(0, prefixLen);
  const suffix = hash.slice(-suffixLen);
  return `${prefix}...${suffix}`;
}

/**
 * Format a Unix timestamp (ms or s) to a human-readable relative/absolute string.
 * Returns ISO string if relative is false.
 */
export function formatTimestamp(ts: number, relative = true): string {
  if (!ts || ts <= 0) return 'Unknown';
  // Substrate timestamps are in milliseconds
  const ms = ts > 1e12 ? ts : ts * 1000;
  const date = new Date(ms);
  if (!relative) return date.toISOString();

  const now = Date.now();
  const diffMs = now - ms;
  if (diffMs < 0) return date.toISOString();
  const diffSec = Math.floor(diffMs / 1000);
  if (diffSec < 60) return `${diffSec}s ago`;
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

/**
 * Format an SS58 address: shorten to first/last chars.
 */
export function formatAddress(address: string | null | undefined, chars = 6): string {
  if (!address) return 'Unknown';
  if (address.length <= chars * 2 + 3) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

/**
 * Format a Substrate balance (in planck) to human-readable CLAW with 4 decimal places.
 * Assumes 18 decimal places (like most Substrate chains).
 */
export function formatBalance(
  planck: string | bigint | number,
  decimals = 18,
  symbol = 'CLAW',
): string {
  try {
    const raw = BigInt(planck.toString());
    const divisor = BigInt(10 ** decimals);
    const whole = raw / divisor;
    const remainder = raw % divisor;
    const fracStr = remainder.toString().padStart(decimals, '0').slice(0, 4);
    return `${whole.toString()}.${fracStr} ${symbol}`;
  } catch {
    return `0.0000 ${symbol}`;
  }
}
