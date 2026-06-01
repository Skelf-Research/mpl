---
title: REST API
---

# REST API Reference

Complete HTTP API reference for the MPL proxy and optional registry API service.

---

## Overview

The MPL ecosystem exposes two sets of HTTP endpoints:

| Service | Default Port | Description |
|---|---|---|
| **MPL Proxy** | `9443` | Core proxy endpoints for health, validation, and JSON-RPC pass-through |
| **Registry API** | Configurable | Optional REST API for querying the SType registry |

---

## Proxy Endpoints (Port 9443)

These endpoints are served by the `mpl proxy` process.

---

### `GET /health`

Health check endpoint. Returns the current health status of the proxy.

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Proxy is healthy and accepting requests |
| `503 Service Unavailable` | Proxy is unhealthy (upstream unreachable) |

#### Example

=== "Request"

    ```bash
    curl http://localhost:9443/health
    ```

=== "Response (200)"

    ```json
    {
      "status": "healthy",
      "upstream": "connected",
      "uptime_seconds": 3842,
      "version": "1.2.0"
    }
    ```

=== "Response (503)"

    ```json
    {
      "status": "unhealthy",
      "upstream": "disconnected",
      "error": "connection refused"
    }
    ```

---

### `GET /capabilities`

Returns the capabilities of the proxy, including supported features, active enforcement modes, and loaded schemas.

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Capabilities returned successfully |

#### Example

=== "Request"

    ```bash
    curl http://localhost:9443/capabilities
    ```

=== "Response"

    ```json
    {
      "proxy": {
        "version": "1.2.0",
        "mode": "strict",
        "protocol": "http"
      },
      "enforcement": {
        "schema_validation": true,
        "qom_profiling": true,
        "ic_assertions": true,
        "policy_engine": true
      },
      "registry": {
        "path": "/opt/mpl/registry",
        "stypes_loaded": 24,
        "profiles_loaded": 3,
        "policies_loaded": 7
      },
      "metrics": {
        "port": 9100,
        "format": "prometheus"
      }
    }
    ```

---

### `POST /`

JSON-RPC proxy endpoint. Forwards JSON-RPC requests to the upstream MCP/A2A server with optional schema validation, QoM evaluation, and policy enforcement applied transparently.

#### Request Body

Standard JSON-RPC 2.0 request:

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "query": "machine learning papers",
      "limit": 10
    }
  },
  "id": 1
}
```

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Request proxied successfully (JSON-RPC response from upstream) |
| `422 Unprocessable Entity` | Schema validation failed (strict mode only) |
| `403 Forbidden` | Policy violation (strict mode only) |
| `502 Bad Gateway` | Upstream server error |
| `504 Gateway Timeout` | Upstream timeout |

#### Examples

=== "Successful Request"

    ```bash
    curl -X POST http://localhost:9443/ \
      -H "Content-Type: application/json" \
      -d '{
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
          "name": "search",
          "arguments": {"query": "hello", "limit": 10}
        },
        "id": 1
      }'
    ```

    ```json
    {
      "jsonrpc": "2.0",
      "result": {
        "content": [
          {
            "type": "text",
            "text": "Found 3 results for 'hello'..."
          }
        ]
      },
      "id": 1
    }
    ```

=== "Validation Failure (422)"

    ```bash
    curl -X POST http://localhost:9443/ \
      -H "Content-Type: application/json" \
      -d '{
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
          "name": "search",
          "arguments": {"limit": -5}
        },
        "id": 1
      }'
    ```

    ```json
    {
      "jsonrpc": "2.0",
      "error": {
        "code": -32602,
        "message": "Schema validation failed",
        "data": {
          "stype": "mcp.tool.search",
          "violations": [
            {
              "path": "$.arguments.query",
              "message": "required property missing"
            },
            {
              "path": "$.arguments.limit",
              "message": "value -5 is less than minimum 1"
            }
          ]
        }
      },
      "id": 1
    }
    ```

=== "Policy Violation (403)"

    ```json
    {
      "jsonrpc": "2.0",
      "error": {
        "code": -32603,
        "message": "Policy violation",
        "data": {
          "policy": "rate-limit",
          "detail": "Rate limit exceeded: 100 requests/minute"
        }
      },
      "id": 1
    }
    ```

#### Response Headers

The proxy adds diagnostic headers to all responses:

| Header | Description |
|---|---|
| `X-MPL-SType` | Resolved SType for the request |
| `X-MPL-Schema-Version` | Schema version used for validation |
| `X-MPL-QoM-Score` | QoM score (if profiling is enabled) |
| `X-MPL-Mode` | Current enforcement mode |
| `X-MPL-Request-Id` | Unique request identifier |

---

### `POST /validate`

Validate a payload against a registered SType schema without forwarding to the upstream server.

#### Request Body

```json
{
  "stype": "mcp.tool.search",
  "payload": {
    "query": "machine learning",
    "limit": 10
  }
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `stype` | `string` | Yes | SType identifier to validate against |
| `payload` | `object` | Yes | The payload to validate |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Validation passed |
| `422 Unprocessable Entity` | Validation failed |
| `404 Not Found` | SType not found in registry |

#### Examples

=== "Validation Passed"

    ```bash
    curl -X POST http://localhost:9443/validate \
      -H "Content-Type: application/json" \
      -d '{
        "stype": "mcp.tool.search",
        "payload": {"query": "hello", "limit": 10}
      }'
    ```

    ```json
    {
      "valid": true,
      "stype": "mcp.tool.search",
      "schema_version": "1.2.0",
      "hash": "sha256:a1b2c3d4..."
    }
    ```

=== "Validation Failed"

    ```bash
    curl -X POST http://localhost:9443/validate \
      -H "Content-Type: application/json" \
      -d '{
        "stype": "mcp.tool.search",
        "payload": {"limit": -5}
      }'
    ```

    ```json
    {
      "valid": false,
      "stype": "mcp.tool.search",
      "schema_version": "1.2.0",
      "errors": [
        {
          "path": "$.query",
          "message": "required property missing"
        },
        {
          "path": "$.limit",
          "message": "value -5 is less than minimum 1"
        }
      ]
    }
    ```

=== "SType Not Found"

    ```json
    {
      "error": "stype_not_found",
      "message": "SType 'mcp.tool.unknown' not found in registry",
      "available_stypes": [
        "mcp.tool.search",
        "mcp.tool.calculate"
      ]
    }
    ```

---

### `POST /mcp`

Direct MCP typed send endpoint. Sends a fully-typed MCP message through the proxy with all enforcement applied, using the MCP message envelope format.

#### Request Body

```json
{
  "type": "tools/call",
  "tool": "search",
  "arguments": {
    "query": "machine learning papers",
    "limit": 10
  },
  "metadata": {
    "stype": "mcp.tool.search",
    "qom_profile": "qom-basic"
  }
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `type` | `string` | Yes | MCP message type (e.g., `tools/call`, `resources/read`) |
| `tool` | `string` | Conditional | Tool name (required for `tools/call`) |
| `arguments` | `object` | Conditional | Tool arguments |
| `metadata` | `object` | No | MPL metadata including SType and QoM profile overrides |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Message processed successfully |
| `422 Unprocessable Entity` | Validation or QoM failure |
| `403 Forbidden` | Policy violation |
| `502 Bad Gateway` | Upstream error |

#### Examples

=== "Request"

    ```bash
    curl -X POST http://localhost:9443/mcp \
      -H "Content-Type: application/json" \
      -d '{
        "type": "tools/call",
        "tool": "search",
        "arguments": {"query": "hello world", "limit": 5},
        "metadata": {"stype": "mcp.tool.search"}
      }'
    ```

=== "Response (200)"

    ```json
    {
      "content": [
        {
          "type": "text",
          "text": "Found 2 results for 'hello world'..."
        }
      ],
      "metadata": {
        "stype": "mcp.tool.search",
        "qom_score": 0.92,
        "schema_valid": true,
        "request_id": "req_abc123"
      }
    }
    ```

---

## Registry API Endpoints

These endpoints are served by the optional `mpl-registry-api` service. They provide a REST interface for querying and managing the SType registry.

---

### `GET /api/stypes`

List all registered STypes with pagination.

#### Query Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `page` | `integer` | `1` | Page number (1-indexed) |
| `limit` | `integer` | `20` | Results per page (max: 100) |
| `status` | `string` | `all` | Filter by status: `approved`, `pending`, `all` |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | List returned successfully |

#### Example

=== "Request"

    ```bash
    curl "http://localhost:8080/api/stypes?page=1&limit=20"
    ```

=== "Response"

    ```json
    {
      "data": [
        {
          "stype": "mcp.tool.search",
          "status": "approved",
          "version": "1.2.0",
          "description": "Full-text search tool",
          "samples": 847,
          "created_at": "2025-01-15T10:30:00Z",
          "updated_at": "2025-02-01T14:22:00Z"
        },
        {
          "stype": "mcp.tool.calculate",
          "status": "approved",
          "version": "1.0.0",
          "description": "Mathematical calculation tool",
          "samples": 312,
          "created_at": "2025-01-20T08:00:00Z",
          "updated_at": "2025-01-20T08:00:00Z"
        }
      ],
      "pagination": {
        "page": 1,
        "limit": 20,
        "total": 24,
        "total_pages": 2
      }
    }
    ```

---

### `GET /api/stypes/:stype`

Get detailed information about a specific SType.

#### Path Parameters

| Parameter | Description |
|---|---|
| `:stype` | SType identifier (e.g., `mcp.tool.search`) |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | SType found |
| `404 Not Found` | SType does not exist |

#### Example

=== "Request"

    ```bash
    curl http://localhost:8080/api/stypes/mcp.tool.search
    ```

=== "Response (200)"

    ```json
    {
      "stype": "mcp.tool.search",
      "status": "approved",
      "version": "1.2.0",
      "description": "Full-text search tool",
      "namespace": "mcp.tool",
      "samples": 847,
      "hash": "sha256:a1b2c3d4e5f6...",
      "schema_url": "/api/stypes/mcp.tool.search/schema",
      "qom_profiles": ["qom-basic", "qom-strict"],
      "policies": ["rate-limit"],
      "created_at": "2025-01-15T10:30:00Z",
      "updated_at": "2025-02-01T14:22:00Z"
    }
    ```

=== "Response (404)"

    ```json
    {
      "error": "not_found",
      "message": "SType 'mcp.tool.unknown' not found"
    }
    ```

---

### `GET /api/stypes/:stype/schema`

Get the JSON Schema definition for a specific SType.

#### Path Parameters

| Parameter | Description |
|---|---|
| `:stype` | SType identifier (e.g., `mcp.tool.search`) |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Schema returned (Content-Type: `application/schema+json`) |
| `404 Not Found` | SType or schema does not exist |

#### Example

=== "Request"

    ```bash
    curl http://localhost:8080/api/stypes/mcp.tool.search/schema
    ```

=== "Response (200)"

    ```json
    {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "$id": "mcp.tool.search",
      "title": "Search Tool",
      "description": "Full-text search across indexed documents",
      "type": "object",
      "properties": {
        "query": {
          "type": "string",
          "minLength": 1,
          "description": "Search query string"
        },
        "limit": {
          "type": "integer",
          "minimum": 1,
          "maximum": 100,
          "default": 10,
          "description": "Maximum number of results"
        },
        "filters": {
          "type": "object",
          "properties": {
            "date_range": {
              "type": "string",
              "enum": ["today", "week", "month", "year", "all"]
            },
            "category": {
              "type": "string"
            }
          }
        }
      },
      "required": ["query"]
    }
    ```

---

### `POST /api/validate`

Validate a payload against a registered SType schema via the registry API.

#### Request Body

```json
{
  "stype": "mcp.tool.search",
  "payload": {
    "query": "machine learning",
    "limit": 10
  }
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `stype` | `string` | Yes | SType identifier |
| `payload` | `object` | Yes | Payload to validate |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Validation result (may be valid or invalid) |
| `404 Not Found` | SType not found |

#### Example

=== "Request"

    ```bash
    curl -X POST http://localhost:8080/api/validate \
      -H "Content-Type: application/json" \
      -d '{
        "stype": "mcp.tool.search",
        "payload": {"query": "hello", "limit": 10}
      }'
    ```

=== "Response (Valid)"

    ```json
    {
      "valid": true,
      "stype": "mcp.tool.search",
      "schema_version": "1.2.0"
    }
    ```

=== "Response (Invalid)"

    ```json
    {
      "valid": false,
      "stype": "mcp.tool.search",
      "schema_version": "1.2.0",
      "errors": [
        {
          "path": "$.query",
          "message": "required property missing"
        }
      ]
    }
    ```

---

### `GET /api/profiles`

List all available QoM profiles.

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Profiles listed successfully |

#### Example

=== "Request"

    ```bash
    curl http://localhost:8080/api/profiles
    ```

=== "Response"

    ```json
    {
      "profiles": [
        {
          "name": "qom-basic",
          "description": "Basic quality checks: completeness and consistency",
          "threshold": 0.80,
          "dimensions": ["completeness", "consistency", "specificity", "schema_conform"]
        },
        {
          "name": "qom-strict",
          "description": "Strict quality enforcement with higher thresholds",
          "threshold": 0.95,
          "dimensions": ["completeness", "consistency", "specificity", "schema_conform", "safety", "relevance"]
        },
        {
          "name": "qom-minimal",
          "description": "Minimal checks for high-throughput scenarios",
          "threshold": 0.60,
          "dimensions": ["completeness", "schema_conform"]
        }
      ]
    }
    ```

---

### `GET /api/search`

Full-text search across SType definitions, descriptions, and schema content.

#### Query Parameters

| Parameter | Type | Required | Description |
|---|---|---|---|
| `q` | `string` | Yes | Search query |

#### Response

| Status Code | Description |
|---|---|
| `200 OK` | Search results returned |
| `400 Bad Request` | Missing or empty query parameter |

#### Example

=== "Request"

    ```bash
    curl "http://localhost:8080/api/search?q=search"
    ```

=== "Response"

    ```json
    {
      "query": "search",
      "results": [
        {
          "stype": "mcp.tool.search",
          "score": 0.95,
          "matches": [
            {"field": "stype", "snippet": "mcp.tool.**search**"},
            {"field": "description", "snippet": "Full-text **search** tool"}
          ]
        },
        {
          "stype": "mcp.tool.search_advanced",
          "score": 0.82,
          "matches": [
            {"field": "stype", "snippet": "mcp.tool.**search**_advanced"}
          ]
        }
      ],
      "total": 2
    }
    ```

---

## Error Response Format

All API endpoints use a consistent error response format:

```json
{
  "error": "error_code",
  "message": "Human-readable error description",
  "details": {}
}
```

### Common Error Codes

| HTTP Status | Error Code | Description |
|---|---|---|
| `400` | `bad_request` | Malformed request body or missing required fields |
| `404` | `not_found` | Requested resource does not exist |
| `422` | `validation_failed` | Payload does not conform to schema |
| `403` | `policy_violation` | Request blocked by policy engine |
| `429` | `rate_limited` | Too many requests |
| `500` | `internal_error` | Unexpected server error |
| `502` | `upstream_error` | Upstream server returned an error |
| `504` | `upstream_timeout` | Upstream server did not respond in time |

---

## Authentication

By default, the MPL proxy and registry API do not require authentication. For production deployments, configure authentication at the network layer (e.g., mTLS, API gateway) or enable the built-in token authentication:

```yaml title="mpl-config.yaml"
# Optional authentication configuration
auth:
  enabled: true
  token_header: "X-MPL-Token"
```

When enabled, include the token in all requests:

```bash
curl -H "X-MPL-Token: your-secret-token" http://localhost:9443/health
```

---

## Rate Limiting

The proxy supports configurable rate limiting per client:

| Header | Description |
|---|---|
| `X-RateLimit-Limit` | Maximum requests per window |
| `X-RateLimit-Remaining` | Remaining requests in current window |
| `X-RateLimit-Reset` | Unix timestamp when the window resets |

When rate limited, the API returns:

```json
{
  "error": "rate_limited",
  "message": "Rate limit exceeded",
  "retry_after_seconds": 30
}
```

---

## Content Types

| Endpoint | Request Content-Type | Response Content-Type |
|---|---|---|
| `GET /health` | - | `application/json` |
| `GET /capabilities` | - | `application/json` |
| `POST /` | `application/json` | `application/json` |
| `POST /validate` | `application/json` | `application/json` |
| `POST /mcp` | `application/json` | `application/json` |
| `GET /api/stypes/:stype/schema` | - | `application/schema+json` |
| All other endpoints | `application/json` | `application/json` |
