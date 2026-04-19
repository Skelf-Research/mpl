import { describe, it, expect } from 'vitest';
import { QomProfile, QomMetrics } from '../src/types/qom';

describe('QomProfile', () => {
  describe('basic', () => {
    it('should create basic profile', () => {
      const profile = QomProfile.basic();
      expect(profile.name).toBe('qom-basic');
    });

    it('should pass with perfect schema fidelity', () => {
      const profile = QomProfile.basic();
      const result = profile.evaluate({ schemaFidelity: 1.0 });
      expect(result.meetsProfile).toBe(true);
      expect(result.failures).toHaveLength(0);
    });

    it('should fail with zero schema fidelity', () => {
      const profile = QomProfile.basic();
      const result = profile.evaluate({ schemaFidelity: 0.0 });
      expect(result.meetsProfile).toBe(false);
      expect(result.failures).toHaveLength(1);
      expect(result.failures[0].metric).toBe('schemaFidelity');
    });
  });

  describe('strictArgcheck', () => {
    it('should create strict argcheck profile', () => {
      const profile = QomProfile.strictArgcheck();
      expect(profile.name).toBe('qom-strict-argcheck');
    });

    it('should pass with both metrics meeting thresholds', () => {
      const profile = QomProfile.strictArgcheck();
      const result = profile.evaluate({
        schemaFidelity: 1.0,
        instructionCompliance: 0.98,
      });
      expect(result.meetsProfile).toBe(true);
    });

    it('should fail with low instruction compliance', () => {
      const profile = QomProfile.strictArgcheck();
      const result = profile.evaluate({
        schemaFidelity: 1.0,
        instructionCompliance: 0.8, // Below 0.95 threshold
      });
      expect(result.meetsProfile).toBe(false);
      expect(result.failures.some((f) => f.metric === 'instructionCompliance')).toBe(true);
    });
  });

  describe('outcome', () => {
    it('should create outcome profile', () => {
      const profile = QomProfile.outcome();
      expect(profile.name).toBe('qom-outcome');
    });

    it('should pass with both metrics meeting thresholds', () => {
      const profile = QomProfile.outcome();
      const result = profile.evaluate({
        schemaFidelity: 1.0,
        toolOutcomeCorrectness: 0.95,
      });
      expect(result.meetsProfile).toBe(true);
    });

    it('should fail with low tool outcome correctness', () => {
      const profile = QomProfile.outcome();
      const result = profile.evaluate({
        schemaFidelity: 1.0,
        toolOutcomeCorrectness: 0.5,
      });
      expect(result.meetsProfile).toBe(false);
    });
  });

  describe('evaluate', () => {
    it('should return correct failure details', () => {
      const profile = QomProfile.basic();
      const result = profile.evaluate({ schemaFidelity: 0.5 });

      expect(result.failures[0]).toEqual({
        metric: 'schemaFidelity',
        actual: 0.5,
        threshold: 1.0,
        direction: 'min',
      });
    });

    it('should include metrics in result', () => {
      const profile = QomProfile.basic();
      const metrics: QomMetrics = { schemaFidelity: 1.0 };
      const result = profile.evaluate(metrics);
      expect(result.metrics).toEqual(metrics);
    });
  });
});
