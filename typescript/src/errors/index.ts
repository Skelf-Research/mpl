/**
 * MPL Error Types
 */

export class MplError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'MplError';
    this.code = code;
  }

  toJSON(): Record<string, unknown> {
    return {
      error: this.code,
      message: this.message,
    };
  }
}

export class SchemaFidelityError extends MplError {
  readonly stype: string;
  readonly validationErrors: Array<{ path: string; message: string }>;

  constructor(
    stype: string,
    validationErrors: Array<{ path: string; message: string }>
  ) {
    super(
      'E-SCHEMA-FIDELITY',
      `Payload does not match schema for ${stype}`
    );
    this.name = 'SchemaFidelityError';
    this.stype = stype;
    this.validationErrors = validationErrors;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      stype: this.stype,
      validation_errors: this.validationErrors,
    };
  }
}

export class QomBreachError extends MplError {
  readonly profile: string;
  readonly metrics: Record<string, number>;
  readonly failures: Array<{ metric: string; actual: number; threshold: number }>;

  constructor(
    profile: string,
    metrics: Record<string, number>,
    failures: Array<{ metric: string; actual: number; threshold: number }>
  ) {
    const failedMetrics = failures.map((f) => f.metric).join(', ');
    super('E-QOM-BREACH', `QoM profile ${profile} not met: ${failedMetrics}`);
    this.name = 'QomBreachError';
    this.profile = profile;
    this.metrics = metrics;
    this.failures = failures;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      profile: this.profile,
      metrics: this.metrics,
      failures: this.failures,
    };
  }
}

export class UnknownStypeError extends MplError {
  readonly stype: string;

  constructor(stype: string) {
    super('E-UNKNOWN-STYPE', `Unknown SType: ${stype}`);
    this.name = 'UnknownStypeError';
    this.stype = stype;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      stype: this.stype,
    };
  }
}

export class NegotiationError extends MplError {
  readonly clientStypes: string[];
  readonly serverStypes: string[];
  readonly reason?: string;

  constructor(
    clientStypes: string[],
    serverStypes: string[],
    reason?: string
  ) {
    super('E-NEGOTIATION-FAILED', reason ?? 'AI-ALPN negotiation failed');
    this.name = 'NegotiationError';
    this.clientStypes = clientStypes;
    this.serverStypes = serverStypes;
    this.reason = reason;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      client_stypes: this.clientStypes,
      server_stypes: this.serverStypes,
    };
  }
}

export class ConnectionError extends MplError {
  readonly endpoint: string;
  readonly cause?: string;

  constructor(endpoint: string, cause?: string) {
    super('E-CONNECTION', `Failed to connect to ${endpoint}`);
    this.name = 'ConnectionError';
    this.endpoint = endpoint;
    this.cause = cause;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      endpoint: this.endpoint,
      cause: this.cause,
    };
  }
}

export class HashMismatchError extends MplError {
  readonly expected: string;
  readonly actual: string;

  constructor(expected: string, actual: string) {
    super('E-HASH-MISMATCH', 'Semantic hash mismatch');
    this.name = 'HashMismatchError';
    this.expected = expected;
    this.actual = actual;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      expected: this.expected,
      actual: this.actual,
    };
  }
}

export class PolicyDeniedError extends MplError {
  readonly policy: string;
  readonly reason: string;
  readonly remediation?: string;

  constructor(policy: string, reason: string, remediation?: string) {
    super('E-POLICY-DENIED', `Policy ${policy} denied: ${reason}`);
    this.name = 'PolicyDeniedError';
    this.policy = policy;
    this.reason = reason;
    this.remediation = remediation;
  }

  toJSON(): Record<string, unknown> {
    return {
      ...super.toJSON(),
      policy: this.policy,
      reason: this.reason,
      remediation: this.remediation,
    };
  }
}
