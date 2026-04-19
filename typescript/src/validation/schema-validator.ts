/**
 * Schema Validator - validates payloads against JSON Schema
 */

import Ajv, { ValidateFunction, ErrorObject } from 'ajv';
import addFormats from 'ajv-formats';

export interface ValidationError {
  path: string;
  message: string;
  keyword?: string;
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export class SchemaValidator {
  private ajv: Ajv;
  private validators: Map<string, ValidateFunction>;

  constructor() {
    this.ajv = new Ajv({
      allErrors: true,
      verbose: true,
      strict: false,
    });
    addFormats(this.ajv);
    this.validators = new Map();
  }

  /**
   * Register a schema for an SType
   */
  register(stype: string, schema: object | string): void {
    const schemaObj = typeof schema === 'string' ? JSON.parse(schema) : schema;

    // Add $id if not present
    if (!schemaObj.$id) {
      schemaObj.$id = `urn:stype:${stype}`;
    }

    const validate = this.ajv.compile(schemaObj);
    this.validators.set(stype, validate);
  }

  /**
   * Check if a schema is registered for the given SType
   */
  hasSchema(stype: string): boolean {
    return this.validators.has(stype);
  }

  /**
   * Validate a payload against an SType schema
   */
  validate(stype: string, payload: unknown): ValidationResult {
    const validate = this.validators.get(stype);
    if (!validate) {
      throw new Error(`No schema registered for SType: ${stype}`);
    }

    const valid = validate(payload);
    const errors = this.formatErrors(validate.errors);

    return { valid: !!valid, errors };
  }

  /**
   * Validate and throw if invalid
   */
  validateOrThrow(stype: string, payload: unknown): void {
    const result = this.validate(stype, payload);
    if (!result.valid) {
      throw new SchemaValidationError(stype, result.errors);
    }
  }

  /**
   * Get all registered STypes
   */
  registeredStypes(): string[] {
    return Array.from(this.validators.keys());
  }

  /**
   * Format AJV errors into our error format
   */
  private formatErrors(errors: ErrorObject[] | null | undefined): ValidationError[] {
    if (!errors) return [];

    return errors.map((err) => ({
      path: err.instancePath || '/',
      message: err.message || 'Validation failed',
      keyword: err.keyword,
    }));
  }
}

export class SchemaValidationError extends Error {
  readonly stype: string;
  readonly validationErrors: ValidationError[];

  constructor(stype: string, errors: ValidationError[]) {
    const messages = errors.map((e) => `${e.path}: ${e.message}`).join(', ');
    super(`Schema validation failed for ${stype}: ${messages}`);
    this.name = 'SchemaValidationError';
    this.stype = stype;
    this.validationErrors = errors;
  }
}
