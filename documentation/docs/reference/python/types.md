---
title: Types
description: Reference for core MPL type classes - SType identifiers and MplEnvelope message containers
---

# Types

Core type classes provided by the Rust bindings (`mpl_sdk._mpl_core`). These represent the fundamental data structures of the MPL protocol.

```python
from mpl_sdk import SType, MplEnvelope
```

---

## SType

```python
class SType:
    def __init__(self, identifier: str): ...
```

Represents a Semantic Type identifier. Parses and validates the four-part naming format: `namespace.domain.Intent.vMajor`.

### Constructor

```python
SType(identifier: str)
```

Parse an SType identifier string into its component parts.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `identifier` | `str` | Full SType identifier (e.g., `"org.calendar.Event.v1"`) |

#### Raises

| Exception | Condition |
|-----------|-----------|
| `ValueError` | Invalid SType format |

#### Example

```python
from mpl_sdk import SType

stype = SType("org.calendar.Event.v1")
print(stype.namespace)      # "org"
print(stype.domain)         # "calendar"
print(stype.name)           # "Event"
print(stype.major_version)  # 1
```

---

### Properties

#### namespace

```python
@property
def namespace(self) -> str
```

The organization namespace. Lowercase, dot-separated identifier (e.g., `"org"`, `"com.acme"`).

#### domain

```python
@property
def domain(self) -> str
```

The functional domain. Lowercase, single word (e.g., `"calendar"`, `"finance"`, `"medical"`).

#### name

```python
@property
def name(self) -> str
```

The semantic intent name. PascalCase (e.g., `"Event"`, `"Transaction"`, `"Diagnosis"`).

#### major_version

```python
@property
def major_version(self) -> int
```

The major version number. Incremented on breaking changes.

---

### Methods

#### urn()

```python
def urn(self) -> str
```

Returns the URN representation of the SType.

**Format:** `urn:stype:namespace.domain.Intent.vMajor`

```python
stype = SType("org.calendar.Event.v1")
print(stype.urn())  # "urn:stype:org.calendar.Event.v1"
```

#### registry_path()

```python
def registry_path(self) -> str
```

Returns the filesystem path where this SType's schema is stored in the registry.

**Format:** `stypes/namespace/domain/Intent/vMajor`

```python
stype = SType("org.calendar.Event.v1")
print(stype.registry_path())  # "stypes/org/calendar/Event/v1"
```

This path is relative to the registry root. The full schema path would be:

```python
schema_file = f"{registry_root}/{stype.registry_path()}/schema.json"
```

---

### Complete Example

```python
from mpl_sdk import SType

# Parse an SType
stype = SType("com.acme.finance.Transaction.v2")

# Access components
print(f"Namespace: {stype.namespace}")       # "com.acme"
print(f"Domain: {stype.domain}")             # "finance"
print(f"Name: {stype.name}")                 # "Transaction"
print(f"Version: {stype.major_version}")     # 2

# Generate paths
print(f"URN: {stype.urn()}")                 # "urn:stype:com.acme.finance.Transaction.v2"
print(f"Registry: {stype.registry_path()}")  # "stypes/com.acme/finance/Transaction/v2"

# Use in application logic
SUPPORTED_STYPES = [
    SType("org.calendar.Event.v1"),
    SType("org.agent.TaskPlan.v1"),
    SType("org.agent.TaskResult.v1"),
]

def is_supported(identifier: str) -> bool:
    """Check if an SType is in our supported list."""
    parsed = SType(identifier)
    return any(
        s.namespace == parsed.namespace
        and s.domain == parsed.domain
        and s.name == parsed.name
        and s.major_version == parsed.major_version
        for s in SUPPORTED_STYPES
    )
```

---

## MplEnvelope

```python
class MplEnvelope:
    def __init__(
        self,
        stype: str,
        payload: str,
        args_stype: str | None = None,
        profile: str | None = None,
    ): ...
```

The MPL message envelope. Wraps a typed payload with metadata including SType declarations, semantic hashes, QoM profile information, and provenance tracking.

### Constructor

```python
MplEnvelope(
    stype: str,
    payload: str,
    args_stype: str | None = None,
    profile: str | None = None,
)
```

Create a new MPL envelope.

#### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `stype` | `str` | *(required)* | SType identifier for the payload |
| `payload` | `str` | *(required)* | JSON string of the payload data |
| `args_stype` | `str \| None` | `None` | SType for the arguments/input that produced this payload |
| `profile` | `str \| None` | `None` | QoM profile name applied to this envelope |

#### Example

```python
import json
from mpl_sdk import MplEnvelope

envelope = MplEnvelope(
    stype="org.calendar.Event.v1",
    payload=json.dumps({
        "title": "Team Standup",
        "start": "2024-01-15T10:00:00Z",
        "duration_minutes": 15,
    }),
    profile="qom-basic",
)
```

---

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `id` | `str` | Unique envelope identifier (UUID) |
| `stype` | `str` | SType identifier for the payload |
| `payload` | `str` | Raw JSON payload string |
| `args_stype` | `str \| None` | SType of the input arguments |
| `profile` | `str \| None` | QoM profile name |
| `sem_hash` | `str \| None` | Semantic hash of the payload (`"blake3:..."`) |
| `provenance` | `dict \| None` | Provenance tracking metadata |
| `qom_report` | `dict \| None` | QoM evaluation report |
| `features` | `dict \| None` | Additional feature flags |
| `timestamp` | `str \| None` | ISO 8601 timestamp of envelope creation |

---

### Methods

#### compute_hash()

```python
def compute_hash(self) -> str
```

Compute the BLAKE3 semantic hash of the payload. This canonicalizes the JSON (sorted keys, normalized encoding) and then computes the hash.

**Returns:** Hash string in format `"blake3:<hex_digest>"`

```python
envelope = MplEnvelope(
    stype="org.calendar.Event.v1",
    payload='{"title":"Meeting","start":"2024-01-15T10:00:00Z"}',
)
hash_value = envelope.compute_hash()
print(hash_value)  # "blake3:a1b2c3d4..."
```

#### verify_hash()

```python
def verify_hash(self) -> bool
```

Verify that the current payload matches the stored `sem_hash`. Returns `False` if `sem_hash` is not set.

```python
envelope.sem_hash = envelope.compute_hash()

# Payload unchanged - verification passes
assert envelope.verify_hash() == True

# If payload were modified, verification would fail
```

#### get_payload()

```python
def get_payload(self) -> dict
```

Parse the JSON payload string and return it as a Python dictionary.

```python
envelope = MplEnvelope(
    stype="org.calendar.Event.v1",
    payload='{"title":"Meeting","start":"2024-01-15T10:00:00Z"}',
)
data = envelope.get_payload()
print(data["title"])  # "Meeting"
```

#### to_json()

```python
def to_json(self) -> str
```

Serialize the entire envelope to a JSON string, including all metadata.

```python
envelope = MplEnvelope(
    stype="org.calendar.Event.v1",
    payload='{"title":"Meeting"}',
    profile="qom-basic",
)
envelope.sem_hash = envelope.compute_hash()

json_str = envelope.to_json()
print(json_str)
# {
#   "id": "...",
#   "stype": "org.calendar.Event.v1",
#   "payload": {"title": "Meeting"},
#   "profile": "qom-basic",
#   "sem_hash": "blake3:...",
#   "timestamp": "2024-01-15T10:00:00Z"
# }
```

---

### Complete Example

```python
import json
from mpl_sdk import MplEnvelope, SType, semantic_hash

# Create an envelope with full metadata
payload_data = {
    "title": "Architecture Review",
    "start": "2024-03-01T14:00:00Z",
    "duration_minutes": 60,
    "attendees": ["alice@example.com", "bob@example.com"],
}

envelope = MplEnvelope(
    stype="org.calendar.Event.v1",
    payload=json.dumps(payload_data),
    args_stype="org.calendar.CreateEventArgs.v1",
    profile="qom-strict-argcheck",
)

# Compute and store hash
envelope.sem_hash = envelope.compute_hash()
print(f"Hash: {envelope.sem_hash}")

# Verify integrity
assert envelope.verify_hash()

# Access parsed payload
data = envelope.get_payload()
print(f"Event: {data['title']} at {data['start']}")

# Serialize for transmission
wire_format = envelope.to_json()

# Parse SType for routing
stype = SType(envelope.stype)
print(f"Domain: {stype.domain}, Intent: {stype.name}")

# Check envelope metadata
print(f"ID: {envelope.id}")
print(f"Profile: {envelope.profile}")
print(f"Timestamp: {envelope.timestamp}")
```

---

## Type Relationships

The following diagram shows how the core types relate to each other:

```
┌─────────────────────────────────────────────────────┐
│  MplEnvelope                                        │
│  ┌───────────────────────────────────────────────┐  │
│  │  stype: "org.calendar.Event.v1" ─────────────────── SType
│  │  payload: '{"title":"Meeting",...}'           │  │
│  │  args_stype: "org.calendar.CreateArgs.v1"     │  │
│  │  profile: "qom-basic"                        │  │
│  │  sem_hash: "blake3:a1b2c3..." ───────────────────── semantic_hash()
│  │  timestamp: "2024-01-15T10:00:00Z"           │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
         │                            │
         ▼                            ▼
  SchemaValidator.validate()    QomProfile.evaluate()
```
