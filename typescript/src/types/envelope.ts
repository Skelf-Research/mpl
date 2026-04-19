/**
 * MPL Envelope - wrapper for typed payloads
 */

import { randomUUID } from 'crypto';

export interface MplEnvelopeOptions {
  id?: string;
  stype: string;
  payload: Record<string, unknown>;
  argsStype?: string;
  profile?: string;
  semHash?: string;
  features?: string[];
  provenance?: Provenance;
}

export interface Provenance {
  intent?: string;
  inputsRef?: string[];
  parentId?: string;
  timestamp?: string;
}

export interface QomReport {
  schemaFidelity: number;
  instructionCompliance?: number;
  groundedness?: number;
  determinismJitter?: number;
  toolOutcomeCorrectness?: number;
  meetsProfile: boolean;
  profile: string;
  failures?: MetricFailure[];
}

export interface MetricFailure {
  metric: string;
  actual: number;
  threshold: number;
}

export class MplEnvelope {
  readonly id: string;
  stype: string;
  payload: Record<string, unknown>;
  argsStype?: string;
  profile?: string;
  semHash?: string;
  features: string[];
  provenance?: Provenance;
  qomReport?: QomReport;

  constructor(options: MplEnvelopeOptions) {
    this.id = options.id ?? randomUUID();
    this.stype = options.stype;
    this.payload = options.payload;
    this.argsStype = options.argsStype;
    this.profile = options.profile;
    this.semHash = options.semHash;
    this.features = options.features ?? [];
    this.provenance = options.provenance;
  }

  /**
   * Create envelope from JSON string
   */
  static fromJSON(json: string): MplEnvelope {
    const data = JSON.parse(json);
    return new MplEnvelope({
      id: data.id,
      stype: data.stype,
      payload: data.payload,
      argsStype: data.args_stype ?? data.argsStype,
      profile: data.profile,
      semHash: data.sem_hash ?? data.semHash,
      features: data.features,
      provenance: data.provenance,
    });
  }

  /**
   * Convert to JSON string
   */
  toJSON(): string {
    return JSON.stringify({
      id: this.id,
      stype: this.stype,
      payload: this.payload,
      args_stype: this.argsStype,
      profile: this.profile,
      sem_hash: this.semHash,
      features: this.features,
      provenance: this.provenance,
      qom_report: this.qomReport,
    }, null, 2);
  }

  /**
   * Convert to plain object
   */
  toObject(): Record<string, unknown> {
    return {
      id: this.id,
      stype: this.stype,
      payload: this.payload,
      args_stype: this.argsStype,
      profile: this.profile,
      sem_hash: this.semHash,
      features: this.features,
      provenance: this.provenance,
      qom_report: this.qomReport,
    };
  }
}
