---
title: Hashing
description: Reference for MPL semantic hashing - BLAKE3 content hashing, JSON canonicalization, and integrity verification
---

# Hashing

Semantic hashing functions provided by the Rust core. These implement deterministic content-addressable hashing for MPL payloads using BLAKE3 over canonicalized JSON. Semantic hashes enable tamper detection, deduplication, and multi-hop integrity verification.

```python
from mpl_sdk import canonicalize, semantic_hash, verify_hash
```

---

## canonicalize()

```python
def canonicalize(json_str: str) -> str
```

Canonicalize a JSON string into a deterministic form. This ensures that semantically equivalent JSON always produces the same byte representation, which is required for consistent hashing.

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `json_str` | `str` | Any valid JSON string |

### Returns

`str` -- The canonicalized JSON string.

### Canonical Form Rules

The canonicalization follows these rules:

| Rule | Before | After |
|------|--------|-------|
| Sort object keys | `{"b":1, "a":2}` | `{"a":2,"b":1}` |
| Remove whitespace | `{ "a" : 1 }` | `{"a":1}` |
| Normalize numbers | `1.0`, `1e2` | `1`, `100` |
| Normalize unicode | `"\u0041"` | `"A"` |
| Recursive sorting | `{"b":{"d":1,"c":2}}` | `{"b":{"c":2,"d":1}}` |
| Stable array order | `[3, 1, 2]` | `[3,1,2]` (preserved) |

!!! note "Array Order"
    Array element order is preserved (not sorted). Only object keys are sorted. This ensures semantic meaning of ordered data is maintained.

### Examples

```python
from mpl_sdk import canonicalize

# Key ordering is normalized
result = canonicalize('{"name": "Alice", "age": 30}')
print(result)  # '{"age":30,"name":"Alice"}'

# Whitespace is removed
result = canonicalize('{\n  "title":  "Meeting",\n  "start":  "2024-01-15"\n}')
print(result)  # '{"start":"2024-01-15","title":"Meeting"}'

# Nested objects are recursively sorted
result = canonicalize('{"outer": {"z": 1, "a": 2}, "inner": [1, 2, 3]}')
print(result)  # '{"inner":[1,2,3],"outer":{"a":2,"z":1}}'

# Numbers are normalized
result = canonicalize('{"value": 1.0, "big": 1e2}')
print(result)  # '{"big":100,"value":1}'

# Same data, different formatting -> same canonical form
json_a = '{"title":"Meeting","start":"2024-01-15T10:00:00Z"}'
json_b = '{\n  "start": "2024-01-15T10:00:00Z",\n  "title": "Meeting"\n}'
assert canonicalize(json_a) == canonicalize(json_b)
```

---

## semantic_hash()

```python
def semantic_hash(json_str: str) -> str
```

Compute the BLAKE3 semantic hash of a JSON payload. The payload is first canonicalized, then the BLAKE3 hash is computed over the canonical byte representation.

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `json_str` | `str` | Any valid JSON string |

### Returns

`str` -- Hash string in format `"blake3:<hex_digest>"`.

### Algorithm

```
Input JSON → canonicalize() → UTF-8 bytes → BLAKE3 → "blake3:" + hex
```

1. The JSON string is canonicalized (sorted keys, no whitespace, normalized values)
2. The canonical string is encoded as UTF-8 bytes
3. BLAKE3 hash is computed over the bytes
4. Result is prefixed with `"blake3:"` for algorithm identification

### Examples

```python
from mpl_sdk import semantic_hash

# Basic usage
hash_value = semantic_hash('{"title": "Meeting", "start": "2024-01-15T10:00:00Z"}')
print(hash_value)  # "blake3:7f8a9b2c..."

# Semantically equivalent JSON produces the same hash
hash_a = semantic_hash('{"title":"Meeting","start":"2024-01-15T10:00:00Z"}')
hash_b = semantic_hash('{"start":"2024-01-15T10:00:00Z","title":"Meeting"}')
assert hash_a == hash_b  # Same content, same hash

# Different content produces different hash
hash_c = semantic_hash('{"title":"Different Meeting","start":"2024-01-15T10:00:00Z"}')
assert hash_a != hash_c
```

### Performance

BLAKE3 is designed for high performance:

- Parallelizable across CPU cores for large inputs
- Significantly faster than SHA-256 for all input sizes
- Consistent performance regardless of input patterns

Typical performance on modern hardware:

| Payload Size | Approximate Time |
|--------------|-----------------|
| 100 bytes | < 1 microsecond |
| 1 KB | ~2 microseconds |
| 100 KB | ~50 microseconds |
| 1 MB | ~500 microseconds |

---

## verify_hash()

```python
def verify_hash(json_str: str, expected_hash: str) -> bool
```

Verify that a JSON payload matches an expected semantic hash. Computes the hash of the payload and compares it to the expected value.

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `json_str` | `str` | The JSON payload to verify |
| `expected_hash` | `str` | The expected hash string (e.g., `"blake3:7f8a..."`) |

### Returns

`bool` -- `True` if the computed hash matches the expected hash, `False` otherwise.

### Examples

```python
from mpl_sdk import semantic_hash, verify_hash

# Compute hash for a payload
payload = '{"title": "Meeting", "start": "2024-01-15T10:00:00Z"}'
hash_value = semantic_hash(payload)

# Later, verify the payload hasn't been modified
assert verify_hash(payload, hash_value) == True

# Modified payload fails verification
modified = '{"title": "MODIFIED Meeting", "start": "2024-01-15T10:00:00Z"}'
assert verify_hash(modified, hash_value) == False

# Semantically equivalent reformatting still passes
reformatted = '{\n  "start": "2024-01-15T10:00:00Z",\n  "title": "Meeting"\n}'
assert verify_hash(reformatted, hash_value) == True
```

---

## Use Cases

### Tamper Detection

Detect if a payload has been modified during transmission through multi-hop agent chains:

```python
from mpl_sdk import semantic_hash, verify_hash, MplEnvelope
from mpl_sdk.errors import HashMismatchError

def verify_envelope_integrity(envelope: MplEnvelope) -> bool:
    """Verify an envelope's payload hasn't been tampered with."""
    if not envelope.sem_hash:
        return True  # No hash to verify

    if not verify_hash(envelope.payload, envelope.sem_hash):
        raise HashMismatchError(
            expected=envelope.sem_hash,
            actual=semantic_hash(envelope.payload),
        )
    return True

# Usage in a receiving agent
async def receive_message(envelope: MplEnvelope):
    try:
        verify_envelope_integrity(envelope)
        # Safe to process
        data = envelope.get_payload()
        await process(data)
    except HashMismatchError as e:
        logger.critical("Payload tampered! Expected %s, got %s", e.expected, e.actual)
        await reject_message(envelope)
```

### Deduplication

Use semantic hashes to detect duplicate messages, even if they arrive with different formatting:

```python
from mpl_sdk import semantic_hash

class MessageDeduplicator:
    """Detect and skip duplicate messages based on content hash."""

    def __init__(self, max_size: int = 10000):
        self._seen: set[str] = set()
        self._max_size = max_size

    def is_duplicate(self, payload_json: str) -> bool:
        """Check if this payload has been seen before."""
        hash_value = semantic_hash(payload_json)

        if hash_value in self._seen:
            return True

        self._seen.add(hash_value)

        # Prevent unbounded growth
        if len(self._seen) > self._max_size:
            # Remove oldest entries (simplified - use OrderedDict in production)
            self._seen = set(list(self._seen)[-self._max_size // 2:])

        return False

# Usage
dedup = MessageDeduplicator()

messages = [
    '{"title": "Meeting", "start": "2024-01-15T10:00:00Z"}',
    '{"start": "2024-01-15T10:00:00Z", "title": "Meeting"}',  # Same content!
    '{"title": "Different", "start": "2024-01-15T11:00:00Z"}',
]

for msg in messages:
    if dedup.is_duplicate(msg):
        print(f"Skipping duplicate: {msg[:40]}...")
    else:
        print(f"Processing: {msg[:40]}...")

# Output:
#   Processing: {"title": "Meeting", "start": "2024-0...
#   Skipping duplicate: {"start": "2024-01-15T10:00:00Z", "t...
#   Processing: {"title": "Different", "start": "2024...
```

### Multi-Hop Integrity

Verify payload integrity across a chain of agents where each agent passes the message forward:

```python
import json
from mpl_sdk import semantic_hash, verify_hash, MplEnvelope

async def forward_with_integrity(
    session,
    envelope: MplEnvelope,
    next_stype: str,
    transform_fn=None,
):
    """Forward a message to the next agent with integrity tracking."""
    payload = envelope.get_payload()

    # Optionally transform the payload
    if transform_fn:
        payload = transform_fn(payload)

    payload_json = json.dumps(payload)

    # Create new envelope with provenance chain
    new_envelope = MplEnvelope(
        stype=next_stype,
        payload=payload_json,
        profile=envelope.profile,
    )
    new_envelope.sem_hash = semantic_hash(payload_json)

    # Track provenance
    new_envelope.provenance = {
        "previous_hash": envelope.sem_hash,
        "previous_stype": envelope.stype,
        "previous_id": envelope.id,
    }

    await session.send(next_stype, payload)
    return new_envelope
```

### Content-Addressable Caching

Use semantic hashes as cache keys for deterministic operations:

```python
from mpl_sdk import semantic_hash

class SemanticCache:
    """Cache results keyed by semantic content hash."""

    def __init__(self):
        self._cache: dict[str, dict] = {}

    def get(self, payload_json: str) -> dict | None:
        """Get cached result for this payload, if any."""
        key = semantic_hash(payload_json)
        return self._cache.get(key)

    def put(self, payload_json: str, result: dict) -> None:
        """Cache a result for this payload."""
        key = semantic_hash(payload_json)
        self._cache[key] = result

    def has(self, payload_json: str) -> bool:
        """Check if result is cached for this payload."""
        key = semantic_hash(payload_json)
        return key in self._cache

# Usage
cache = SemanticCache()

payload = '{"query": "What is MPL?", "context": "documentation"}'
cached = cache.get(payload)

if cached:
    print("Cache hit!")
    result = cached
else:
    result = await expensive_operation(payload)
    cache.put(payload, result)
```

---

## BLAKE3 Algorithm Details

MPL uses [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) as its hash algorithm. Key properties:

| Property | Value |
|----------|-------|
| Output size | 256 bits (64 hex characters) |
| Algorithm family | Merkle tree-based |
| Collision resistance | 128-bit security level |
| Performance | ~3x faster than SHA-256 |
| Parallelism | Built-in SIMD and multi-threading support |

### Hash Format

All semantic hashes use the format:

```
blake3:<64_hex_characters>
```

Example:
```
blake3:7f8a9b2c4d5e6f708192a3b4c5d6e7f80192a3b4c5d6e7f89a0b1c2d3e4f506
```

The `blake3:` prefix enables future algorithm agility -- if a hash algorithm upgrade is needed, the prefix distinguishes old hashes from new ones.

---

## Canonical JSON Specification

The canonical form used by MPL follows these rules precisely:

1. **Object keys**: Sorted lexicographically by Unicode code point
2. **Whitespace**: No whitespace between tokens (no spaces, newlines, or tabs)
3. **Numbers**: Shortest representation (no trailing zeros, no unnecessary exponent notation)
4. **Strings**: UTF-8 encoded, minimal escaping (only `"`, `\`, and control characters escaped)
5. **Arrays**: Element order preserved exactly as-is
6. **Nested objects**: Rules applied recursively to all depth levels
7. **Null/boolean**: Represented as `null`, `true`, `false` (lowercase)

This ensures that any two implementations producing canonical JSON from the same semantic content will produce byte-identical output, guaranteeing hash consistency across different languages and platforms.
