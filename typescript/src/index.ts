/**
 * MPL SDK - Meaning Protocol Layer for AI agents
 *
 * Simple usage (recommended):
 * ```typescript
 * import { Client, Mode } from 'mpl-sdk';
 *
 * const client = new Client('http://localhost:9443');
 * const result = await client.call('calendar.create', { title: 'Meeting' });
 * ```
 *
 * Advanced usage:
 * ```typescript
 * import { Session, SessionConfig, QomProfile } from 'mpl-sdk';
 * // ... full control over validation, QoM, etc.
 * ```
 *
 * @packageDocumentation
 */

// ===== Simple API (use these first) =====
export { Client, Mode, ClientOptions, CallResult, typed } from './client';

// Errors (commonly needed)
export { MplError, SchemaFidelityError } from './errors';

// ===== Advanced API =====

// Types
export { SType, STypeParseError, STypeComponents } from './types/stype';
export {
  MplEnvelope,
  MplEnvelopeOptions,
  Provenance,
  QomReport,
} from './types/envelope';
export {
  QomMetrics,
  QomProfile,
  QomProfileConfig,
  QomEvaluation,
  MetricFailure,
  MetricThreshold,
} from './types/qom';

// Validation
export {
  SchemaValidator,
  SchemaValidationError,
  ValidationResult,
  ValidationError,
} from './validation/schema-validator';
export { canonicalize, semanticHash, verifyHash } from './validation/hash';

// Session
export {
  Session,
  SessionConfig,
  NegotiatedCapabilities,
  SendOptions,
} from './session/session';

// More Errors
export {
  QomBreachError,
  UnknownStypeError,
  NegotiationError,
  ConnectionError,
  HashMismatchError,
  PolicyDeniedError,
} from './errors';

// Version
export const VERSION = '0.1.0';
