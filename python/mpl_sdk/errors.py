"""
MPL SDK Error Classes

Provides Python exception hierarchy for MPL operations.
"""

from typing import Any, Dict, List, Optional


class MplError(Exception):
    """Base exception for all MPL errors."""

    def __init__(self, message: str, code: Optional[str] = None, details: Optional[Dict[str, Any]] = None):
        super().__init__(message)
        self.message = message
        self.code = code or "E-MPL-UNKNOWN"
        self.details = details or {}

    def __str__(self) -> str:
        return f"[{self.code}] {self.message}"

    def to_dict(self) -> Dict[str, Any]:
        """Convert error to dictionary for serialization."""
        return {
            "code": self.code,
            "message": self.message,
            "details": self.details,
        }


class SchemaFidelityError(MplError):
    """
    Raised when payload fails schema validation.

    Error code: E-SCHEMA-FIDELITY
    """

    def __init__(
        self,
        message: str,
        stype: str,
        validation_errors: Optional[List[Dict[str, Any]]] = None,
    ):
        super().__init__(
            message=message,
            code="E-SCHEMA-FIDELITY",
            details={
                "stype": stype,
                "validation_errors": validation_errors or [],
            },
        )
        self.stype = stype
        self.validation_errors = validation_errors or []


class QomBreachError(MplError):
    """
    Raised when QoM metrics fall below threshold.

    Error code: E-QOM-BREACH
    """

    def __init__(
        self,
        message: str,
        metric: str,
        expected: float,
        actual: float,
        profile: Optional[str] = None,
    ):
        super().__init__(
            message=message,
            code="E-QOM-BREACH",
            details={
                "metric": metric,
                "expected": expected,
                "actual": actual,
                "profile": profile,
            },
        )
        self.metric = metric
        self.expected = expected
        self.actual = actual
        self.profile = profile


class NegotiationError(MplError):
    """
    Raised when AI-ALPN handshake fails.

    Error code: E-HANDSHAKE-FAILED
    """

    def __init__(
        self,
        message: str,
        client_stypes: Optional[List[str]] = None,
        server_stypes: Optional[List[str]] = None,
        reason: Optional[str] = None,
    ):
        super().__init__(
            message=message,
            code="E-HANDSHAKE-FAILED",
            details={
                "client_stypes": client_stypes or [],
                "server_stypes": server_stypes or [],
                "reason": reason,
            },
        )
        self.client_stypes = client_stypes or []
        self.server_stypes = server_stypes or []
        self.reason = reason


class UnknownStypeError(MplError):
    """
    Raised when an SType is not found in the registry.

    Error code: E-UNKNOWN-STYPE
    """

    def __init__(self, stype: str, registry_path: Optional[str] = None):
        super().__init__(
            message=f"Unknown SType: {stype}",
            code="E-UNKNOWN-STYPE",
            details={
                "stype": stype,
                "registry_path": registry_path,
            },
        )
        self.stype = stype
        self.registry_path = registry_path


class HashMismatchError(MplError):
    """
    Raised when semantic hash verification fails.

    Error code: E-HASH-MISMATCH
    """

    def __init__(self, expected: str, actual: str, stype: Optional[str] = None):
        super().__init__(
            message=f"Hash mismatch: expected {expected[:16]}..., got {actual[:16]}...",
            code="E-HASH-MISMATCH",
            details={
                "expected": expected,
                "actual": actual,
                "stype": stype,
            },
        )
        self.expected = expected
        self.actual = actual


class ConnectionError(MplError):
    """
    Raised when connection to MCP/A2A server fails.

    Error code: E-CONNECTION-FAILED
    """

    def __init__(self, message: str, endpoint: str, cause: Optional[str] = None):
        super().__init__(
            message=message,
            code="E-CONNECTION-FAILED",
            details={
                "endpoint": endpoint,
                "cause": cause,
            },
        )
        self.endpoint = endpoint
        self.cause = cause


class TimeoutError(MplError):
    """
    Raised when operation times out.

    Error code: E-TIMEOUT
    """

    def __init__(self, message: str, timeout_ms: int, operation: Optional[str] = None):
        super().__init__(
            message=message,
            code="E-TIMEOUT",
            details={
                "timeout_ms": timeout_ms,
                "operation": operation,
            },
        )
        self.timeout_ms = timeout_ms
        self.operation = operation
