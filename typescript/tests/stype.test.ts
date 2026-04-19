import { describe, it, expect } from 'vitest';
import { SType, STypeParseError } from '../src/types/stype';

describe('SType', () => {
  describe('parse', () => {
    it('should parse valid SType string', () => {
      const stype = SType.parse('org.calendar.Event.v1');
      expect(stype.namespace).toBe('org');
      expect(stype.domain).toBe('calendar');
      expect(stype.name).toBe('Event');
      expect(stype.majorVersion).toBe(1);
    });

    it('should parse complex namespace', () => {
      const stype = SType.parse('com.acme.finance.InvestmentRecommendation.v2');
      expect(stype.namespace).toBe('com.acme');
      expect(stype.domain).toBe('finance');
      expect(stype.name).toBe('InvestmentRecommendation');
      expect(stype.majorVersion).toBe(2);
    });

    it('should throw on invalid format', () => {
      expect(() => SType.parse('invalid')).toThrow(STypeParseError);
    });

    it('should throw on invalid version', () => {
      expect(() => SType.parse('org.calendar.Event.vX')).toThrow(STypeParseError);
    });

    it('should throw on lowercase name', () => {
      expect(() => SType.parse('org.calendar.event.v1')).toThrow(STypeParseError);
    });
  });

  describe('id', () => {
    it('should return correct id', () => {
      const stype = SType.parse('org.calendar.Event.v1');
      expect(stype.id()).toBe('org.calendar.Event.v1');
    });
  });

  describe('urn', () => {
    it('should return correct URN', () => {
      const stype = SType.parse('org.calendar.Event.v1');
      expect(stype.urn()).toBe('urn:stype:org.calendar.Event.v1');
    });
  });

  describe('registryPath', () => {
    it('should return correct registry path', () => {
      const stype = SType.parse('org.calendar.Event.v1');
      expect(stype.registryPath()).toBe('stypes/org/calendar/Event/v1');
    });
  });

  describe('create', () => {
    it('should create SType from components', () => {
      const stype = SType.create('org', 'calendar', 'Event', 1);
      expect(stype.id()).toBe('org.calendar.Event.v1');
    });
  });

  describe('toString', () => {
    it('should return id as string', () => {
      const stype = SType.parse('org.calendar.Event.v1');
      expect(String(stype)).toBe('org.calendar.Event.v1');
    });
  });
});
