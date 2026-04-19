/**
 * Quality of Meaning (QoM) types and profiles
 */

export interface QomMetrics {
  /** Schema Fidelity - 1.0 if valid, 0.0 if invalid */
  schemaFidelity: number;
  /** Instruction Compliance - assertion pass rate */
  instructionCompliance?: number;
  /** Groundedness - claim support score */
  groundedness?: number;
  /** Determinism under Jitter - consistency score */
  determinismJitter?: number;
  /** Ontology Adherence - semantic constraint compliance */
  ontologyAdherence?: number;
  /** Tool Outcome Correctness - business logic validation */
  toolOutcomeCorrectness?: number;
}

export interface MetricThreshold {
  min?: number;
  max?: number;
}

export interface QomProfileConfig {
  name: string;
  description?: string;
  metrics: {
    schemaFidelity?: MetricThreshold;
    instructionCompliance?: MetricThreshold;
    groundedness?: MetricThreshold;
    determinismJitter?: MetricThreshold;
    ontologyAdherence?: MetricThreshold;
    toolOutcomeCorrectness?: MetricThreshold;
  };
}

export interface QomEvaluation {
  meetsProfile: boolean;
  profile: string;
  metrics: QomMetrics;
  failures: MetricFailure[];
}

export interface MetricFailure {
  metric: string;
  actual: number;
  threshold: number;
  direction: 'min' | 'max';
}

export class QomProfile {
  readonly name: string;
  readonly description?: string;
  private readonly thresholds: Map<string, MetricThreshold>;

  constructor(config: QomProfileConfig) {
    this.name = config.name;
    this.description = config.description;
    this.thresholds = new Map(Object.entries(config.metrics));
  }

  /**
   * Create basic profile (Schema Fidelity only)
   */
  static basic(): QomProfile {
    return new QomProfile({
      name: 'qom-basic',
      description: 'Basic QoM profile requiring schema fidelity',
      metrics: {
        schemaFidelity: { min: 1.0 },
      },
    });
  }

  /**
   * Create strict profile (SF + IC)
   */
  static strictArgcheck(): QomProfile {
    return new QomProfile({
      name: 'qom-strict-argcheck',
      description: 'Strict QoM profile with schema fidelity and instruction compliance',
      metrics: {
        schemaFidelity: { min: 1.0 },
        instructionCompliance: { min: 0.95 },
      },
    });
  }

  /**
   * Create outcome-focused profile
   */
  static outcome(): QomProfile {
    return new QomProfile({
      name: 'qom-outcome',
      description: 'QoM profile focused on tool outcome correctness',
      metrics: {
        schemaFidelity: { min: 1.0 },
        toolOutcomeCorrectness: { min: 0.9 },
      },
    });
  }

  /**
   * Evaluate metrics against this profile
   */
  evaluate(metrics: QomMetrics): QomEvaluation {
    const failures: MetricFailure[] = [];

    for (const [metric, threshold] of this.thresholds) {
      const value = metrics[metric as keyof QomMetrics];

      if (value === undefined) {
        // Metric not provided - skip if not required
        continue;
      }

      if (threshold.min !== undefined && value < threshold.min) {
        failures.push({
          metric,
          actual: value,
          threshold: threshold.min,
          direction: 'min',
        });
      }

      if (threshold.max !== undefined && value > threshold.max) {
        failures.push({
          metric,
          actual: value,
          threshold: threshold.max,
          direction: 'max',
        });
      }
    }

    return {
      meetsProfile: failures.length === 0,
      profile: this.name,
      metrics,
      failures,
    };
  }
}
