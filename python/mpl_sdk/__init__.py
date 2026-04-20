"""
MPL SDK - Meaning Protocol Layer for AI agents

Simple usage (recommended):
    from mpl_sdk import Client, Mode

    client = Client("http://localhost:9443")
    result = await client.call("calendar.create", {"title": "Meeting"})

Advanced usage:
    from mpl_sdk import Session, SessionConfig, QomProfile
    # ... full control over validation, QoM, etc.

This SDK provides Python bindings for MPL, enabling:
- Semantic Types (STypes) for typed payloads
- Schema validation (Schema Fidelity)
- QoM profiles and evaluation
- Semantic hashing and canonicalization
- Session management with MCP/A2A servers
"""

from mpl_sdk._mpl_core import (
    # Core types
    SType,
    MplEnvelope,
    # Validation
    SchemaValidator,
    ValidationResult,
    SchemaError,
    # QoM
    QomMetrics,
    QomProfile,
    QomEvaluation,
    MetricFailure,
    # Functions
    canonicalize,
    semantic_hash,
    verify_hash,
    # Version
    __version__,
)

# Simple API (recommended)
from mpl_sdk.client import Client, Mode, CallResult, typed

# Advanced API
from mpl_sdk.session import Session, SessionConfig, NegotiatedCapabilities
from mpl_sdk.errors import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
)

__all__ = [
    # ===== Simple API (use these first) =====
    "Client",
    "Mode",
    "CallResult",
    "typed",
    # Errors
    "MplError",
    "SchemaFidelityError",

    # ===== Advanced API =====
    # Session management
    "Session",
    "SessionConfig",
    "NegotiatedCapabilities",
    # Core types
    "SType",
    "MplEnvelope",
    # Validation
    "SchemaValidator",
    "ValidationResult",
    "SchemaError",
    # QoM
    "QomMetrics",
    "QomProfile",
    "QomEvaluation",
    "MetricFailure",
    # Functions
    "canonicalize",
    "semantic_hash",
    "verify_hash",
    # More errors
    "QomBreachError",
    "NegotiationError",
    "UnknownStypeError",
    # Version
    "__version__",
]
