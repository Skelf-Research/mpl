/**
 * Simplified MPL Client
 *
 * A minimal, user-friendly interface for MPL. Designed for the 80% use case
 * where you just want to call tools with type safety.
 *
 * @example
 * ```typescript
 * import { Client, Mode } from 'mpl-sdk';
 *
 * // Simple usage - just works
 * const client = new Client('http://localhost:9443');
 * const result = await client.call('calendar.create', { title: 'Meeting' });
 *
 * // With TypeScript generics for type safety
 * const result = await client.call<CalendarEvent>('calendar.create', { ... });
 * ```
 *
 * @packageDocumentation
 */

import { MplError } from './errors';

/**
 * Operating mode for the client.
 */
export enum Mode {
  /** Log validation errors but don't fail requests. */
  Development = 'development',
  /** Enforce validation and fail on errors. */
  Production = 'production',
}

/**
 * Options for creating a Client.
 */
export interface ClientOptions {
  /** Operating mode (development or production). Default: development */
  mode?: Mode;
  /** Request timeout in milliseconds. Default: 30000 */
  timeout?: number;
  /** Custom headers to include in all requests. */
  headers?: Record<string, string>;
}

/**
 * Result from a tool call.
 */
export interface CallResult<T = unknown> {
  /** The response payload. */
  data: T;
  /** SType of the response, if known. */
  stype?: string;
  /** Whether schema validation passed. */
  valid: boolean;
  /** Whether QoM evaluation passed. */
  qomPassed: boolean;
}

/**
 * Simple MPL client for calling typed tools.
 *
 * @example
 * ```typescript
 * // Basic usage
 * const client = new Client('http://localhost:9443');
 * const result = await client.call('calendar.create', {
 *   title: 'Meeting',
 *   start: '2024-01-15T10:00:00Z'
 * });
 * console.log(result.data);
 *
 * // With type safety
 * interface CalendarEvent {
 *   id: string;
 *   title: string;
 *   start: string;
 * }
 * const result = await client.call<CalendarEvent>('calendar.create', {...});
 * console.log(result.data.id); // TypeScript knows this is a string
 * ```
 */
export class Client {
  private readonly endpoint: string;
  private readonly mode: Mode;
  private readonly timeout: number;
  private readonly headers: Record<string, string>;

  /**
   * Create a new MPL client.
   *
   * @param endpoint - MPL proxy URL (e.g., "http://localhost:9443")
   * @param options - Optional configuration
   */
  constructor(endpoint: string, options: ClientOptions = {}) {
    this.endpoint = endpoint.replace(/\/$/, '');
    this.mode = options.mode ?? Mode.Development;
    this.timeout = options.timeout ?? 30000;
    this.headers = options.headers ?? {};
  }

  /**
   * Call a tool through the MPL proxy.
   *
   * @param tool - Tool name (e.g., "calendar.create")
   * @param args - Tool arguments
   * @param stype - Optional SType for the request
   * @returns CallResult with the response data
   *
   * @example
   * ```typescript
   * const result = await client.call('calendar.create', {
   *   title: 'Meeting',
   *   start: '2024-01-15T10:00:00Z'
   * });
   * ```
   */
  async call<T = unknown>(
    tool: string,
    args: Record<string, unknown>,
    stype?: string,
  ): Promise<CallResult<T>> {
    const requestBody = {
      jsonrpc: '2.0',
      id: 1,
      method: 'tools/call',
      params: {
        name: tool,
        arguments: args,
      },
    };

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...this.headers,
    };

    if (stype) {
      headers['X-MPL-SType'] = stype;
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.endpoint}/`, {
        method: 'POST',
        headers,
        body: JSON.stringify(requestBody),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      // Check QoM result header
      const qomResult = response.headers.get('X-MPL-QoM-Result') ?? 'pass';
      const qomPassed = qomResult.toLowerCase() === 'pass';

      const data = await response.json();

      // Check for JSON-RPC error
      if (data.error) {
        if (this.mode === Mode.Production) {
          throw new MplError(
            `Tool call failed: ${data.error.message ?? JSON.stringify(data.error)}`,
          );
        }
        return {
          data: data.error as T,
          valid: false,
          qomPassed,
        };
      }

      return {
        data: (data.result ?? data) as T,
        stype: response.headers.get('X-MPL-SType') ?? undefined,
        valid: true,
        qomPassed,
      };
    } catch (error) {
      if (error instanceof MplError) throw error;
      throw new MplError(`Request failed: ${error}`);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Send a typed payload directly (without JSON-RPC wrapper).
   *
   * Use this for non-tool payloads or direct MPL communication.
   *
   * @param stype - SType identifier (e.g., "org.calendar.Event.v1")
   * @param payload - The payload data
   * @returns CallResult with the response
   */
  async send<T = unknown>(
    stype: string,
    payload: Record<string, unknown>,
  ): Promise<CallResult<T>> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'X-MPL-SType': stype,
      ...this.headers,
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(`${this.endpoint}/mcp`, {
        method: 'POST',
        headers,
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      const qomResult = response.headers.get('X-MPL-QoM-Result') ?? 'pass';
      const qomPassed = qomResult.toLowerCase() === 'pass';

      const data = await response.json();

      return {
        data: data as T,
        stype,
        valid: response.ok,
        qomPassed,
      };
    } catch (error) {
      throw new MplError(`Request failed: ${error}`);
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Check proxy health status.
   */
  async health(): Promise<{ status: string; version: string }> {
    const response = await fetch(`${this.endpoint}/health`);
    return response.json();
  }

  /**
   * Get proxy capabilities (supported STypes, profiles, etc.).
   */
  async capabilities(): Promise<{
    version: string;
    stypes: string[];
    profiles: string[];
    capabilities: Record<string, boolean>;
  }> {
    const response = await fetch(`${this.endpoint}/capabilities`);
    return response.json();
  }
}

/**
 * Decorator to mark a function as typed with MPL.
 *
 * Note: This is a runtime marker for TypeScript/JavaScript.
 * The actual validation happens at the proxy level.
 *
 * @example
 * ```typescript
 * class CalendarService {
 *   @typed('org.calendar.Event.v1')
 *   async createEvent(payload: CalendarEvent): Promise<CalendarEvent> {
 *     return { id: 'event-123', ...payload };
 *   }
 * }
 * ```
 */
export function typed(stype: string): MethodDecorator {
  return function (
    target: object,
    propertyKey: string | symbol,
    descriptor: PropertyDescriptor,
  ): PropertyDescriptor {
    const originalMethod = descriptor.value;
    descriptor.value = function (...args: unknown[]) {
      // Attach SType metadata (can be used by middleware)
      (descriptor.value as { _mplStype?: string })._mplStype = stype;
      return originalMethod.apply(this, args);
    };
    (descriptor.value as { _mplStype?: string })._mplStype = stype;
    return descriptor;
  };
}
