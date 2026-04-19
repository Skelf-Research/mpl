/**
 * Semantic Type (SType) - globally unique, versioned identifier
 *
 * Format: namespace.domain.Name.vMajor
 * Example: org.calendar.Event.v1
 */

export interface STypeComponents {
  namespace: string;
  domain: string;
  name: string;
  majorVersion: number;
}

export class SType {
  readonly namespace: string;
  readonly domain: string;
  readonly name: string;
  readonly majorVersion: number;

  private constructor(components: STypeComponents) {
    this.namespace = components.namespace;
    this.domain = components.domain;
    this.name = components.name;
    this.majorVersion = components.majorVersion;
  }

  /**
   * Parse an SType from a string
   * @param stypeStr - String in format "namespace.domain.Name.vMajor"
   */
  static parse(stypeStr: string): SType {
    const parts = stypeStr.split('.');
    if (parts.length < 4) {
      throw new STypeParseError(
        `Invalid SType format: ${stypeStr}. Expected namespace.domain.Name.vMajor`
      );
    }

    // Last part should be version (vN)
    const versionPart = parts[parts.length - 1];
    const versionMatch = versionPart.match(/^v(\d+)$/);
    if (!versionMatch) {
      throw new STypeParseError(
        `Invalid version format in SType: ${stypeStr}. Expected vN (e.g., v1)`
      );
    }
    const majorVersion = parseInt(versionMatch[1], 10);

    // Second to last is the name (should start with uppercase)
    const name = parts[parts.length - 2];
    if (!/^[A-Z]/.test(name)) {
      throw new STypeParseError(
        `SType name should start with uppercase: ${name}`
      );
    }

    // Everything else is namespace.domain
    // Last namespace segment is domain
    const namespaceParts = parts.slice(0, -2);
    if (namespaceParts.length < 2) {
      throw new STypeParseError(
        `Invalid namespace in SType: ${stypeStr}. Expected at least namespace.domain`
      );
    }

    const domain = namespaceParts[namespaceParts.length - 1];
    const namespace = namespaceParts.slice(0, -1).join('.');

    return new SType({ namespace, domain, name, majorVersion });
  }

  /**
   * Create an SType from components
   */
  static create(
    namespace: string,
    domain: string,
    name: string,
    majorVersion: number
  ): SType {
    return new SType({ namespace, domain, name, majorVersion });
  }

  /**
   * Get the short identifier (namespace.domain.Name.vMajor)
   */
  id(): string {
    return `${this.namespace}.${this.domain}.${this.name}.v${this.majorVersion}`;
  }

  /**
   * Get the full URN
   */
  urn(): string {
    return `urn:stype:${this.id()}`;
  }

  /**
   * Get the registry path for this SType
   */
  registryPath(): string {
    return `stypes/${this.namespace}/${this.domain}/${this.name}/v${this.majorVersion}`;
  }

  toString(): string {
    return this.id();
  }

  toJSON(): string {
    return this.id();
  }
}

export class STypeParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'STypeParseError';
  }
}
