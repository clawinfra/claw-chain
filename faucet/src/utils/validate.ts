/**
 * SS58 address validation utilities.
 *
 * Uses @polkadot/util-crypto to decode and validate SS58 addresses.
 * Accepts any valid SS58 prefix (generic substrate prefix 42 by default).
 */

import { decodeAddress, encodeAddress } from '@polkadot/util-crypto';

/**
 * Returns true if the given string is a valid SS58-encoded substrate address.
 * Accepts any prefix (0–16383).
 */
export function isValidSS58(address: string): boolean {
  if (!address || typeof address !== 'string') return false;
  const trimmed = address.trim();
  if (trimmed.length === 0) return false;
  try {
    decodeAddress(trimmed);
    return true;
  } catch {
    return false;
  }
}

/**
 * Normalise an SS58 address to the generic substrate prefix (42).
 * Returns null if the address is invalid.
 */
export function normaliseAddress(address: string): string | null {
  if (!isValidSS58(address)) return null;
  try {
    const decoded = decodeAddress(address.trim());
    return encodeAddress(decoded, 42);
  } catch {
    return null;
  }
}

/**
 * Validate and return an error message, or null if valid.
 */
export function validateAddress(address: unknown): string | null {
  if (address === undefined || address === null) {
    return 'address is required';
  }
  if (typeof address !== 'string') {
    return 'address must be a string';
  }
  if (address.trim().length === 0) {
    return 'address must not be empty';
  }
  if (!isValidSS58(address)) {
    return 'invalid SS58 address';
  }
  return null;
}
