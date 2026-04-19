import { describe, it, expect, beforeEach } from 'vitest';
import { SchemaValidator, SchemaValidationError } from '../src/validation/schema-validator';
import { canonicalize, semanticHash, verifyHash } from '../src/validation/hash';

describe('SchemaValidator', () => {
  let validator: SchemaValidator;

  beforeEach(() => {
    validator = new SchemaValidator();
  });

  const eventSchema = {
    type: 'object',
    properties: {
      title: { type: 'string' },
      start: { type: 'string', format: 'date-time' },
      end: { type: 'string', format: 'date-time' },
    },
    required: ['title', 'start', 'end'],
  };

  describe('register', () => {
    it('should register schema', () => {
      validator.register('org.calendar.Event.v1', eventSchema);
      expect(validator.hasSchema('org.calendar.Event.v1')).toBe(true);
    });

    it('should register schema from JSON string', () => {
      validator.register('org.calendar.Event.v1', JSON.stringify(eventSchema));
      expect(validator.hasSchema('org.calendar.Event.v1')).toBe(true);
    });
  });

  describe('validate', () => {
    beforeEach(() => {
      validator.register('org.calendar.Event.v1', eventSchema);
    });

    it('should validate correct payload', () => {
      const result = validator.validate('org.calendar.Event.v1', {
        title: 'Meeting',
        start: '2024-01-15T10:00:00Z',
        end: '2024-01-15T11:00:00Z',
      });
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should reject payload missing required fields', () => {
      const result = validator.validate('org.calendar.Event.v1', {
        title: 'Meeting',
      });
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should reject payload with wrong type', () => {
      const result = validator.validate('org.calendar.Event.v1', {
        title: 123, // Should be string
        start: '2024-01-15T10:00:00Z',
        end: '2024-01-15T11:00:00Z',
      });
      expect(result.valid).toBe(false);
    });

    it('should throw for unknown SType', () => {
      expect(() => validator.validate('unknown.Type.v1', {})).toThrow();
    });
  });

  describe('validateOrThrow', () => {
    beforeEach(() => {
      validator.register('org.calendar.Event.v1', eventSchema);
    });

    it('should not throw for valid payload', () => {
      expect(() => {
        validator.validateOrThrow('org.calendar.Event.v1', {
          title: 'Meeting',
          start: '2024-01-15T10:00:00Z',
          end: '2024-01-15T11:00:00Z',
        });
      }).not.toThrow();
    });

    it('should throw SchemaValidationError for invalid payload', () => {
      expect(() => {
        validator.validateOrThrow('org.calendar.Event.v1', { title: 'Meeting' });
      }).toThrow(SchemaValidationError);
    });
  });

  describe('registeredStypes', () => {
    it('should return all registered stypes', () => {
      validator.register('org.calendar.Event.v1', eventSchema);
      validator.register('org.communication.Message.v1', { type: 'object' });

      const stypes = validator.registeredStypes();
      expect(stypes).toContain('org.calendar.Event.v1');
      expect(stypes).toContain('org.communication.Message.v1');
    });
  });
});

describe('Semantic Hash', () => {
  describe('canonicalize', () => {
    it('should sort object keys', () => {
      const a = canonicalize({ b: 2, a: 1 });
      const b = canonicalize({ a: 1, b: 2 });
      expect(a).toBe(b);
    });

    it('should handle nested objects', () => {
      const result = canonicalize({ z: { b: 2, a: 1 }, a: 1 });
      expect(result).toBe('{"a":1,"z":{"a":1,"b":2}}');
    });

    it('should handle arrays', () => {
      const result = canonicalize({ items: [{ b: 2, a: 1 }] });
      expect(result).toBe('{"items":[{"a":1,"b":2}]}');
    });
  });

  describe('semanticHash', () => {
    it('should produce consistent hash', () => {
      const payload = { title: 'Meeting', count: 5 };
      const hash1 = semanticHash(payload);
      const hash2 = semanticHash(payload);
      expect(hash1).toBe(hash2);
    });

    it('should produce same hash for different key orders', () => {
      const hash1 = semanticHash({ b: 2, a: 1 });
      const hash2 = semanticHash({ a: 1, b: 2 });
      expect(hash1).toBe(hash2);
    });

    it('should have b3: prefix', () => {
      const hash = semanticHash({ test: true });
      expect(hash.startsWith('b3:')).toBe(true);
    });
  });

  describe('verifyHash', () => {
    it('should return true for matching hash', () => {
      const payload = { title: 'Test' };
      const hash = semanticHash(payload);
      expect(verifyHash(payload, hash)).toBe(true);
    });

    it('should return false for mismatched hash', () => {
      const payload = { title: 'Test' };
      expect(verifyHash(payload, 'b3:invalid')).toBe(false);
    });
  });
});
