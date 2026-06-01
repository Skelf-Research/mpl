---
title: Creating a Custom SType
description: Design, register, and validate a custom Semantic Type from scratch
---

# Creating a Custom SType

This guide walks you through the complete process of designing, building, registering, and using a custom SType. You will create `org.support.Ticket.v1` -- a support ticket type with schema validation, business assertions, and test cases.

---

## Goal

By the end of this guide, you will:

- Understand the SType design process (namespace, domain, intent, version)
- Create the directory structure in the registry
- Write a JSON Schema with strict validation
- Write CEL assertions for business rules
- Create example payloads and negative test cases
- Register the SType with the CLI
- Validate payloads against it
- Use it in the Python and TypeScript SDKs

---

## Prerequisites

| Requirement | Version | Check Command |
|-------------|---------|---------------|
| MPL CLI | >= 0.5.0 | `mpl --version` |
| MPL Proxy | Running on `:9443` | `curl http://localhost:9443/health` |
| Python SDK | >= 0.3.0 | `pip show mpl-sdk` |
| Write access | To the `registry/` directory | `ls registry/stypes/` |

---

## Step 1: Design the SType

Before writing any code, decide on the four parts of the SType identifier:

```
namespace.domain.Intent.vMajor
```

For our support ticket:

| Part | Value | Reasoning |
|------|-------|-----------|
| **Namespace** | `org` | Standard namespace for general-purpose types |
| **Domain** | `support` | Functional area: customer support |
| **Intent** | `Ticket` | What this message represents: a support ticket |
| **Version** | `v1` | First version of this contract |

The full SType identifier: **`org.support.Ticket.v1`**

!!! tip "Naming Guidelines"
    - Use `org` for types that could be shared across organizations
    - Use `com.yourcompany` for organization-specific types
    - Keep the domain to a single word describing the functional area
    - Use PascalCase for the Intent (e.g., `Ticket`, not `ticket` or `TICKET`)
    - Always start at `v1` -- never `v0`

---

## Step 2: Create the Directory Structure

STypes live in the registry at a deterministic path:

```
registry/stypes/{namespace}/{domain}/{Intent}/v{major}/
```

Create the directory structure:

```bash
mkdir -p registry/stypes/org/support/Ticket/v1/examples
mkdir -p registry/stypes/org/support/Ticket/v1/negative
```

The final structure will be:

```
registry/stypes/org/support/Ticket/
  v1/
    schema.json          # JSON Schema (draft 2020-12)
    assertions.json      # CEL business rules
    metadata.json        # Version metadata
    examples/
      basic-ticket.json  # Valid example payload
      full-ticket.json   # Valid example with all fields
    negative/
      missing-title.json # Invalid: missing required field
      bad-priority.json  # Invalid: wrong enum value
```

---

## Step 3: Write the Schema

Create `registry/stypes/org/support/Ticket/v1/schema.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mpl.dev/stypes/org/support/Ticket/v1/schema.json",
  "title": "Support Ticket",
  "description": "A customer support ticket with priority, assignment, and categorization.",
  "type": "object",
  "required": ["ticketId", "title", "priority", "description"],
  "additionalProperties": false,
  "properties": {
    "ticketId": {
      "type": "string",
      "pattern": "^TKT-[0-9]{6,}$",
      "description": "Unique ticket identifier in format TKT-NNNNNN"
    },
    "title": {
      "type": "string",
      "minLength": 5,
      "maxLength": 200,
      "description": "Brief summary of the issue"
    },
    "priority": {
      "type": "string",
      "enum": ["critical", "high", "medium", "low"],
      "description": "Ticket priority level"
    },
    "assignee": {
      "type": "string",
      "format": "email",
      "description": "Email of the assigned support agent"
    },
    "description": {
      "type": "string",
      "minLength": 20,
      "maxLength": 5000,
      "description": "Detailed description of the issue (minimum 20 characters)"
    },
    "tags": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1,
        "maxLength": 50
      },
      "maxItems": 10,
      "uniqueItems": true,
      "description": "Categorization tags (up to 10, must be unique)"
    }
  }
}
```

### Schema Design Decisions

| Decision | Rationale |
|----------|-----------|
| `additionalProperties: false` | Prevents undeclared fields from bypassing governance |
| `ticketId` pattern | Enforces consistent ID format across systems |
| `priority` enum | Limits to known priority levels; no free-form text |
| `description` minLength: 20 | Ensures tickets have meaningful descriptions |
| `tags` uniqueItems | Prevents duplicate categorization |
| `assignee` as optional | Tickets may not be assigned immediately |

!!! warning "Required: additionalProperties: false"
    This is mandatory for all SType schemas. Without it, agents could inject arbitrary data that bypasses the governance layer.

---

## Step 4: Write Assertions

Assertions are business rules that go beyond what JSON Schema can express. They are evaluated as part of the **Instruction Compliance (IC)** QoM metric.

Create `registry/stypes/org/support/Ticket/v1/assertions.json`:

```json
{
  "$schema": "https://mpl.dev/schemas/assertions/v1",
  "stype": "org.support.Ticket.v1",
  "assertions": [
    {
      "id": "critical-needs-assignee",
      "description": "Critical tickets must have an assignee",
      "expression": "payload.priority != 'critical' || has(payload.assignee)",
      "severity": "error"
    },
    {
      "id": "title-not-placeholder",
      "description": "Title must not be a placeholder or test string",
      "expression": "!payload.title.matches('^(test|TODO|FIXME|placeholder|asdf).*$')",
      "severity": "warning"
    },
    {
      "id": "description-longer-than-title",
      "description": "Description should be more detailed than the title",
      "expression": "size(payload.description) > size(payload.title)",
      "severity": "warning"
    },
    {
      "id": "tags-when-high-priority",
      "description": "High and critical tickets should have at least one tag for routing",
      "expression": "!(payload.priority in ['critical', 'high']) || (has(payload.tags) && size(payload.tags) >= 1)",
      "severity": "warning"
    }
  ]
}
```

### Assertion Syntax (CEL)

Assertions use [Common Expression Language (CEL)](https://github.com/google/cel-spec) expressions:

| Expression | Meaning |
|------------|---------|
| `has(payload.field)` | Field is present (not null/undefined) |
| `payload.field.matches('regex')` | Field matches regex pattern |
| `size(payload.field)` | Length of string or array |
| `payload.field in ['a', 'b']` | Field value is in the list |
| `\|\|` / `&&` | Logical OR / AND |

### Severity Levels

| Severity | Effect on IC Score | Behavior |
|----------|-------------------|----------|
| `error` | Failed assertion reduces IC score | May trigger E-QOM-BREACH |
| `warning` | Failed assertion reduces IC score (weighted less) | Logged but less impactful |
| `info` | No impact on IC score | Informational only |

!!! info "IC Score Calculation"
    The Instruction Compliance score is calculated as: `IC = passing_assertions / total_assertions`. With 4 assertions, each one that fails reduces the score by 0.25. Under `qom-strict-argcheck` (IC >= 0.97), any assertion failure would trigger a breach.

---

## Step 5: Add Example Payloads

Examples serve as documentation and are used for testing. Create valid examples:

### examples/basic-ticket.json

```json
{
  "ticketId": "TKT-000123",
  "title": "Login page returns 500 error",
  "priority": "high",
  "description": "When attempting to log in with valid credentials, the login page returns a 500 Internal Server Error. This started happening after the deployment at 14:00 UTC.",
  "tags": ["auth", "production"]
}
```

### examples/full-ticket.json

```json
{
  "ticketId": "TKT-000456",
  "title": "Critical: Payment processing timeout in EU region",
  "priority": "critical",
  "assignee": "oncall@example.com",
  "description": "Payment processing is timing out for all EU-region customers. The Stripe webhook is returning 504 errors. Revenue impact estimated at $50k/hour. Payment team has been notified and is investigating the upstream provider status.",
  "tags": ["payments", "eu-region", "critical-outage", "stripe"]
}
```

---

## Step 6: Add Negative Test Cases

Negative tests verify that invalid payloads are correctly rejected:

### negative/missing-title.json

```json
{
  "_description": "Missing required 'title' field",
  "_expected_error": "E-SCHEMA-FIDELITY",
  "payload": {
    "ticketId": "TKT-000789",
    "priority": "medium",
    "description": "This ticket is missing the required title field and should be rejected."
  }
}
```

### negative/bad-priority.json

```json
{
  "_description": "Invalid priority enum value",
  "_expected_error": "E-SCHEMA-FIDELITY",
  "payload": {
    "ticketId": "TKT-000790",
    "title": "Something is broken",
    "priority": "urgent",
    "description": "Priority 'urgent' is not a valid enum value. Must be critical, high, medium, or low."
  }
}
```

### negative/critical-no-assignee.json

```json
{
  "_description": "Critical ticket without assignee (assertion failure)",
  "_expected_error": "E-QOM-BREACH",
  "_expected_metric": "instruction_compliance",
  "payload": {
    "ticketId": "TKT-000791",
    "title": "Database cluster is down",
    "priority": "critical",
    "description": "The primary database cluster is unresponsive. All services depending on it are affected. No assignee is set, which violates the critical-needs-assignee assertion."
  }
}
```

### negative/extra-fields.json

```json
{
  "_description": "Undeclared field 'severity' (additionalProperties: false)",
  "_expected_error": "E-SCHEMA-FIDELITY",
  "payload": {
    "ticketId": "TKT-000792",
    "title": "Minor UI alignment issue",
    "priority": "low",
    "description": "The submit button is slightly misaligned on mobile. This payload includes an undeclared 'severity' field.",
    "severity": "minor"
  }
}
```

---

## Step 7: Register the SType

Use the MPL CLI to register and approve the new SType:

```bash
# Validate the schema is well-formed
mpl schemas validate registry/stypes/org/support/Ticket/v1/schema.json

# Run the negative tests to verify rejection
mpl schemas test org.support.Ticket.v1

# Register (marks as pending review)
mpl schemas register org.support.Ticket.v1

# Approve for use (moves to active)
mpl schemas approve org.support.Ticket.v1

# Verify it is registered
mpl schemas show org.support.Ticket.v1
```

Expected output from `mpl schemas show`:

```
SType:       org.support.Ticket.v1
Status:      active
Schema:      registry/stypes/org/support/Ticket/v1/schema.json
Assertions:  4 rules (2 error, 2 warning)
Examples:    2 valid, 4 negative
Created:     2025-02-01T10:00:00Z
```

!!! tip "Schema Governance Workflow"
    In team environments, the `register` step creates a pending review. Another team member approves it after reviewing the schema, assertions, and test cases. This mirrors code review practices for API contracts.

---

## Step 8: Validate a Payload

Test validation against your new SType:

=== "CLI"

    ```bash
    # Validate a valid payload
    mpl validate --stype org.support.Ticket.v1 examples/basic-ticket.json

    # Validate an invalid payload (expect failure)
    mpl validate --stype org.support.Ticket.v1 negative/missing-title.json
    ```

=== "Python"

    ```python
    from mpl_sdk import Client

    client = Client("http://localhost:9443")

    # Valid ticket
    result = await client.validate(
        stype="org.support.Ticket.v1",
        payload={
            "ticketId": "TKT-001000",
            "title": "API rate limiting not working",
            "priority": "high",
            "assignee": "backend-team@example.com",
            "description": "The API rate limiter is not enforcing the 1000 req/min limit. Clients are able to send unlimited requests without being throttled.",
            "tags": ["api", "rate-limiting"]
        }
    )

    print(f"Valid: {result.valid}")           # True
    print(f"IC score: {result.qom_report.metrics.instruction_compliance.score}")  # 1.0
    ```

=== "curl"

    ```bash
    curl -X POST http://localhost:9443/validate \
      -H "Content-Type: application/json" \
      -H "X-MPL-SType: org.support.Ticket.v1" \
      -d '{
        "ticketId": "TKT-001000",
        "title": "API rate limiting not working",
        "priority": "high",
        "assignee": "backend-team@example.com",
        "description": "The API rate limiter is not enforcing the 1000 req/min limit. Clients are able to send unlimited requests without being throttled.",
        "tags": ["api", "rate-limiting"]
      }'
    ```

### Validation Response (Valid)

```json
{
  "valid": true,
  "stype": "org.support.Ticket.v1",
  "sem_hash": "sha256:b4c5d6e7...",
  "qom_report": {
    "profile": "qom-strict-argcheck",
    "meets_profile": true,
    "metrics": {
      "schema_fidelity": {
        "score": 1.0,
        "details": { "validation_errors": [] }
      },
      "instruction_compliance": {
        "score": 1.0,
        "details": {
          "assertions_total": 4,
          "assertions_passed": 4,
          "failures": []
        }
      }
    }
  }
}
```

### Validation Response (Assertion Failure)

Testing a critical ticket without an assignee:

```json
{
  "valid": true,
  "stype": "org.support.Ticket.v1",
  "sem_hash": "sha256:f8g9h0i1...",
  "qom_report": {
    "profile": "qom-strict-argcheck",
    "meets_profile": false,
    "metrics": {
      "schema_fidelity": {
        "score": 1.0,
        "details": { "validation_errors": [] }
      },
      "instruction_compliance": {
        "score": 0.75,
        "details": {
          "assertions_total": 4,
          "assertions_passed": 3,
          "failures": [
            {
              "id": "critical-needs-assignee",
              "description": "Critical tickets must have an assignee",
              "severity": "error"
            }
          ]
        }
      }
    }
  }
}
```

!!! note "Schema Valid, QoM Breach"
    Notice that the payload passes Schema Fidelity (all fields are valid JSON Schema) but fails Instruction Compliance (business assertion violated). The schema checks *structure*; assertions check *business rules*.

---

## Step 9: Use in the SDK

Integrate your custom SType into application code:

=== "Python"

    ```python
    from mpl_sdk import Client, Mode

    client = Client("http://localhost:9443", mode=Mode.PRODUCTION)

    async def create_ticket(title: str, priority: str, description: str,
                            assignee: str = None, tags: list = None):
        """Create a support ticket with MPL validation."""
        ticket_id = generate_ticket_id()  # e.g., "TKT-001234"

        payload = {
            "ticketId": ticket_id,
            "title": title,
            "priority": priority,
            "description": description,
        }

        if assignee:
            payload["assignee"] = assignee
        if tags:
            payload["tags"] = tags

        result = await client.call(
            "support.create_ticket",
            payload=payload,
            headers={"X-MPL-SType": "org.support.Ticket.v1"}
        )

        return {
            "ticketId": ticket_id,
            "valid": result.valid,
            "qom_passed": result.qom_passed,
            "sem_hash": result.sem_hash,
            "data": result.data
        }

    # Usage
    ticket = await create_ticket(
        title="Search indexing is 3 hours behind",
        priority="high",
        description="The Elasticsearch indexing pipeline is lagging. New documents are not appearing in search results for over 3 hours. The queue depth shows 2.4M pending documents.",
        assignee="search-team@example.com",
        tags=["search", "elasticsearch", "indexing"]
    )
    print(f"Created ticket: {ticket['ticketId']}")
    print(f"Semantic hash: {ticket['sem_hash']}")
    ```

=== "TypeScript"

    ```typescript
    import { MplClient, Mode } from '@mpl/sdk';

    const client = new MplClient('http://localhost:9443', { mode: Mode.Production });

    interface TicketInput {
      title: string;
      priority: 'critical' | 'high' | 'medium' | 'low';
      description: string;
      assignee?: string;
      tags?: string[];
    }

    async function createTicket(input: TicketInput) {
      const ticketId = generateTicketId(); // e.g., "TKT-001234"

      const payload: Record<string, unknown> = {
        ticketId,
        title: input.title,
        priority: input.priority,
        description: input.description,
      };

      if (input.assignee) payload.assignee = input.assignee;
      if (input.tags) payload.tags = input.tags;

      const result = await client.call('support.create_ticket', {
        payload,
        headers: { 'X-MPL-SType': 'org.support.Ticket.v1' },
      });

      return {
        ticketId,
        valid: result.valid,
        qomPassed: result.qomPassed,
        semHash: result.semHash,
        data: result.data,
      };
    }

    // Usage
    const ticket = await createTicket({
      title: 'Search indexing is 3 hours behind',
      priority: 'high',
      description: 'The Elasticsearch indexing pipeline is lagging. New documents are not appearing in search results for over 3 hours. The queue depth shows 2.4M pending documents.',
      assignee: 'search-team@example.com',
      tags: ['search', 'elasticsearch', 'indexing'],
    });

    console.log(`Created ticket: ${ticket.ticketId}`);
    console.log(`Semantic hash: ${ticket.semHash}`);
    ```

---

## Complete File Listing

Here is the complete set of files you created:

### schema.json

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://mpl.dev/stypes/org/support/Ticket/v1/schema.json",
  "title": "Support Ticket",
  "description": "A customer support ticket with priority, assignment, and categorization.",
  "type": "object",
  "required": ["ticketId", "title", "priority", "description"],
  "additionalProperties": false,
  "properties": {
    "ticketId": {
      "type": "string",
      "pattern": "^TKT-[0-9]{6,}$",
      "description": "Unique ticket identifier in format TKT-NNNNNN"
    },
    "title": {
      "type": "string",
      "minLength": 5,
      "maxLength": 200,
      "description": "Brief summary of the issue"
    },
    "priority": {
      "type": "string",
      "enum": ["critical", "high", "medium", "low"],
      "description": "Ticket priority level"
    },
    "assignee": {
      "type": "string",
      "format": "email",
      "description": "Email of the assigned support agent"
    },
    "description": {
      "type": "string",
      "minLength": 20,
      "maxLength": 5000,
      "description": "Detailed description of the issue (minimum 20 characters)"
    },
    "tags": {
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1,
        "maxLength": 50
      },
      "maxItems": 10,
      "uniqueItems": true,
      "description": "Categorization tags (up to 10, must be unique)"
    }
  }
}
```

### assertions.json

```json
{
  "$schema": "https://mpl.dev/schemas/assertions/v1",
  "stype": "org.support.Ticket.v1",
  "assertions": [
    {
      "id": "critical-needs-assignee",
      "description": "Critical tickets must have an assignee",
      "expression": "payload.priority != 'critical' || has(payload.assignee)",
      "severity": "error"
    },
    {
      "id": "title-not-placeholder",
      "description": "Title must not be a placeholder or test string",
      "expression": "!payload.title.matches('^(test|TODO|FIXME|placeholder|asdf).*$')",
      "severity": "warning"
    },
    {
      "id": "description-longer-than-title",
      "description": "Description should be more detailed than the title",
      "expression": "size(payload.description) > size(payload.title)",
      "severity": "warning"
    },
    {
      "id": "tags-when-high-priority",
      "description": "High and critical tickets should have at least one tag for routing",
      "expression": "!(payload.priority in ['critical', 'high']) || (has(payload.tags) && size(payload.tags) >= 1)",
      "severity": "warning"
    }
  ]
}
```

---

## Design Checklist

Use this checklist when creating any new SType:

- [ ] **Namespace** chosen (org, com.yourcompany, eval, ai, data)
- [ ] **Domain** is a single lowercase word
- [ ] **Intent** is PascalCase, specific, and descriptive
- [ ] **Version** starts at v1
- [ ] **schema.json** uses draft 2020-12
- [ ] **additionalProperties: false** is set
- [ ] All fields have `description`
- [ ] `required` array lists mandatory fields
- [ ] Format keywords used where applicable (email, date-time, uuid, uri)
- [ ] **assertions.json** covers business rules beyond schema
- [ ] **examples/** has at least one valid payload
- [ ] **negative/** has test cases for each validation rule
- [ ] Schema validated with `mpl schemas validate`
- [ ] Tests pass with `mpl schemas test`
- [ ] Registered and approved

---

## What You Learned

In this guide, you:

1. **Designed an SType** by choosing namespace, domain, intent, and version
2. **Created the directory structure** in the registry
3. **Wrote a JSON Schema** with strict validation rules
4. **Wrote CEL assertions** for business logic beyond schema
5. **Created example payloads** for documentation and testing
6. **Created negative test cases** to verify rejection behavior
7. **Registered the SType** using the CLI
8. **Validated payloads** and understood the difference between schema and assertion failures
9. **Used the SType in code** with the Python and TypeScript SDKs

---

## Next Steps

- **[Calendar Workflow](calendar-workflow.md)** -- See SType validation in action
- **[RAG with QoM](rag-workflow.md)** -- Understand groundedness for generated content
- **[Multi-Agent Workflow](multi-agent.md)** -- Use STypes for agent orchestration
- **[STypes Concepts](../../concepts/stypes.md)** -- Deep dive into versioning and governance
- **[QoM Concepts](../../concepts/qom.md)** -- Learn how assertions affect IC scores
