/**
 * MPL SDK - Meaning Protocol Layer for AI agents
 *
 * @packageDocumentation
 */

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

// Errors
export {
  MplError,
  SchemaFidelityError,
  QomBreachError,
  UnknownStypeError,
  NegotiationError,
  ConnectionError,
  HashMismatchError,
  PolicyDeniedError,
} from './errors';

// Version
export const VERSION = '0.1.0';
