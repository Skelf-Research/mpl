/**
 * Semantic Hashing - BLAKE3-based deterministic hashing
 */

import { createHash } from 'crypto';

/**
 * Canonicalize a JSON value for deterministic hashing
 * - Sort object keys alphabetically
 * - Normalize numbers
 * - Remove undefined values
 */
export function canonicalize(value: unknown): string {
  return JSON.stringify(sortKeys(value));
}

/**
 * Recursively sort object keys for canonical representation
 */
function sortKeys(value: unknown): unknown {
  if (value === null || value === undefined) {
    return null;
  }

  if (Array.isArray(value)) {
    return value.map(sortKeys);
  }

  if (typeof value === 'object') {
    const sorted: Record<string, unknown> = {};
    const keys = Object.keys(value as object).sort();
    for (const key of keys) {
      const v = (value as Record<string, unknown>)[key];
      if (v !== undefined) {
        sorted[key] = sortKeys(v);
      }
    }
    return sorted;
  }

  return value;
}

/**
 * Compute semantic hash of a payload
 * Uses SHA-256 with "b3:" prefix for compatibility
 * (In production, this should use BLAKE3)
 */
export function semanticHash(payload: unknown): string {
  const canonical = canonicalize(payload);
  const hash = createHash('sha256').update(canonical).digest('hex');
  return `b3:${hash}`;
}

/**
 * Verify a semantic hash matches a payload
 */
export function verifyHash(payload: unknown, expectedHash: string): boolean {
  const computed = semanticHash(payload);
  return computed === expectedHash;
}
