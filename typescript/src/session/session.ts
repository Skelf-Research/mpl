/**
 * MPL Session - manages connection and typed communication
 */

import WebSocket from 'ws';
import { MplEnvelope, MplEnvelopeOptions } from '../types/envelope';
import { QomProfile, QomMetrics } from '../types/qom';
import { SchemaValidator } from '../validation/schema-validator';
import { semanticHash } from '../validation/hash';
import {
  ConnectionError,
  NegotiationError,
  SchemaFidelityError,
} from '../errors';

export interface SessionConfig {
  /** Server endpoint URL (ws:// or http://) */
  endpoint: string;
  /** List of STypes this session supports */
  stypes?: string[];
  /** QoM profile to enforce */
  qomProfile?: string;
  /** Path to local registry */
  registryPath?: string;
  /** Request timeout in ms */
  timeoutMs?: number;
  /** Auto-validate payloads */
  autoValidate?: boolean;
  /** Auto-compute semantic hashes */
  autoHash?: boolean;
}

export interface NegotiatedCapabilities {
  commonStypes: string[];
  selectedProfile?: string;
  serverExtensions: Record<string, unknown>;
}

export interface SendOptions {
  validate?: boolean;
  computeHash?: boolean;
}

type MessageHandler = (envelope: MplEnvelope) => void | Promise<void>;

export class Session {
  private config: SessionConfig;
  private ws?: WebSocket;
  private validator: SchemaValidator;
  private qomProfile?: QomProfile;
  private negotiated?: NegotiatedCapabilities;
  private connected: boolean = false;
  private messageHandlers: Map<string, MessageHandler> = new Map();
  private pendingRequests: Map<string, {
    resolve: (value: MplEnvelope) => void;
    reject: (error: Error) => void;
  }> = new Map();

  constructor(config: SessionConfig) {
    this.config = {
      timeoutMs: 30000,
      autoValidate: true,
      autoHash: true,
      stypes: [],
      ...config,
    };
    this.validator = new SchemaValidator();
  }

  /**
   * Connect to the server and perform AI-ALPN handshake
   */
  async connect(): Promise<NegotiatedCapabilities> {
    const { endpoint } = this.config;

    try {
      if (endpoint.startsWith('ws://') || endpoint.startsWith('wss://')) {
        await this.connectWebSocket();
      } else {
        throw new ConnectionError(endpoint, 'Only WebSocket connections supported');
      }

      // Perform AI-ALPN handshake
      this.negotiated = await this.handshake();
      this.connected = true;

      // Load QoM profile
      if (this.config.qomProfile) {
        this.qomProfile = this.loadQomProfile(this.config.qomProfile);
      }

      return this.negotiated;
    } catch (error) {
      throw new ConnectionError(
        endpoint,
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  /**
   * Establish WebSocket connection
   */
  private connectWebSocket(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.config.endpoint);

      this.ws.on('open', () => resolve());
      this.ws.on('error', (err) => reject(err));
      this.ws.on('message', (data) => this.handleMessage(data.toString()));
      this.ws.on('close', () => {
        this.connected = false;
      });
    });
  }

  /**
   * Perform AI-ALPN handshake
   */
  private async handshake(): Promise<NegotiatedCapabilities> {
    const clientHello = {
      type: 'ai-alpn-hello',
      version: '1.0',
      stypes: this.config.stypes,
      qom_profiles: this.config.qomProfile ? [this.config.qomProfile] : [],
    };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Handshake timeout'));
      }, this.config.timeoutMs);

      const handler = (data: string) => {
        clearTimeout(timeout);
        const response = JSON.parse(data);

        if (response.type === 'ai-alpn-error') {
          reject(
            new NegotiationError(
              this.config.stypes ?? [],
              response.server_stypes ?? [],
              response.message
            )
          );
          return;
        }

        resolve({
          commonStypes: response.common_stypes ?? [],
          selectedProfile: response.selected_profile,
          serverExtensions: response.extensions ?? {},
        });
      };

      // Temporarily override message handler for handshake
      if (this.ws) {
        this.ws.once('message', (data) => handler(data.toString()));
        this.ws.send(JSON.stringify(clientHello));
      }
    });
  }

  /**
   * Load a QoM profile
   */
  private loadQomProfile(profileName: string): QomProfile {
    switch (profileName) {
      case 'qom-strict-argcheck':
        return QomProfile.strictArgcheck();
      case 'qom-outcome':
        return QomProfile.outcome();
      default:
        return QomProfile.basic();
    }
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(data: string): void {
    try {
      const envelope = MplEnvelope.fromJSON(data);
      const handler = this.messageHandlers.get(envelope.stype);

      if (handler) {
        handler(envelope);
      }

      // Check for pending request response
      const pending = this.pendingRequests.get(envelope.id);
      if (pending) {
        this.pendingRequests.delete(envelope.id);
        pending.resolve(envelope);
      }
    } catch (error) {
      console.error('Failed to handle message:', error);
    }
  }

  /**
   * Register a message handler for an SType
   */
  onMessage(stype: string, handler: MessageHandler): void {
    this.messageHandlers.set(stype, handler);
  }

  /**
   * Send a typed payload
   */
  async send(
    stype: string,
    payload: Record<string, unknown>,
    options: SendOptions = {}
  ): Promise<MplEnvelope> {
    if (!this.connected) {
      throw new ConnectionError(this.config.endpoint, 'Not connected');
    }

    const shouldValidate = options.validate ?? this.config.autoValidate;
    const shouldHash = options.computeHash ?? this.config.autoHash;

    // Validate payload
    if (shouldValidate && this.validator.hasSchema(stype)) {
      const result = this.validator.validate(stype, payload);
      if (!result.valid) {
        throw new SchemaFidelityError(stype, result.errors);
      }
    }

    // Compute semantic hash
    const semHash = shouldHash ? semanticHash(payload) : undefined;

    // Create envelope
    const envelope = new MplEnvelope({
      stype,
      payload,
      profile: this.config.qomProfile,
      semHash,
    });

    // Send and wait for response
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(envelope.id);
        reject(new Error('Request timeout'));
      }, this.config.timeoutMs);

      this.pendingRequests.set(envelope.id, {
        resolve: (response) => {
          clearTimeout(timeout);
          resolve(response);
        },
        reject: (error) => {
          clearTimeout(timeout);
          reject(error);
        },
      });

      this.ws?.send(envelope.toJSON());
    });
  }

  /**
   * Register a schema for validation
   */
  registerSchema(stype: string, schema: object | string): void {
    this.validator.register(stype, schema);
  }

  /**
   * Close the session
   */
  async close(): Promise<void> {
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
    this.connected = false;
    this.negotiated = undefined;
  }

  /**
   * Check if connected
   */
  get isConnected(): boolean {
    return this.connected;
  }

  /**
   * Get negotiated capabilities
   */
  get capabilities(): NegotiatedCapabilities | undefined {
    return this.negotiated;
  }
}
