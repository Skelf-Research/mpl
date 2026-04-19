"""
MPL SDK - Meaning Protocol Layer for AI agents

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

from mpl_sdk.session import Session
from mpl_sdk.errors import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
)

__all__ = [
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
    # Session
    "Session",
    # Errors
    "MplError",
    "SchemaFidelityError",
    "QomBreachError",
    "NegotiationError",
    "UnknownStypeError",
    # Version
    "__version__",
]
