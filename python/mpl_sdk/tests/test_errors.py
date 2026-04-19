"""
Tests for MPL SDK error classes.
"""

import pytest

from mpl_sdk.errors import (
    MplError,
    SchemaFidelityError,
    QomBreachError,
    NegotiationError,
    UnknownStypeError,
    HashMismatchError,
    ConnectionError,
    TimeoutError,
)


class TestMplError:
    """Tests for base MplError."""

    def test_basic_error(self):
        error = MplError("Something went wrong")
        assert error.message == "Something went wrong"
        assert error.code == "E-MPL-UNKNOWN"

    def test_error_with_code(self):
        error = MplError("Test error", code="E-TEST")
        assert error.code == "E-TEST"

    def test_error_with_details(self):
        error = MplError("Test", details={"key": "value"})
        assert error.details["key"] == "value"

    def test_error_str(self):
        error = MplError("Test error", code="E-TEST")
        assert str(error) == "[E-TEST] Test error"

    def test_error_to_dict(self):
        error = MplError("Test", code="E-TEST", details={"foo": "bar"})
        d = error.to_dict()
        assert d["code"] == "E-TEST"
        assert d["message"] == "Test"
        assert d["details"]["foo"] == "bar"


class TestSchemaFidelityError:
    """Tests for SchemaFidelityError."""

    def test_schema_fidelity_error(self):
        error = SchemaFidelityError(
            message="Validation failed",
            stype="org.calendar.Event.v1",
            validation_errors=[{"path": "/title", "message": "required"}]
        )

        assert error.code == "E-SCHEMA-FIDELITY"
        assert error.stype == "org.calendar.Event.v1"
        assert len(error.validation_errors) == 1

    def test_schema_fidelity_details(self):
        error = SchemaFidelityError(
            message="Validation failed",
            stype="org.test.Type.v1"
        )

        assert error.details["stype"] == "org.test.Type.v1"


class TestQomBreachError:
    """Tests for QomBreachError."""

    def test_qom_breach_error(self):
        error = QomBreachError(
            message="Schema fidelity below threshold",
            metric="schema_fidelity",
            expected=1.0,
            actual=0.5,
            profile="qom-basic"
        )

        assert error.code == "E-QOM-BREACH"
        assert error.metric == "schema_fidelity"
        assert error.expected == 1.0
        assert error.actual == 0.5
        assert error.profile == "qom-basic"

    def test_qom_breach_details(self):
        error = QomBreachError(
            message="Breach",
            metric="instruction_compliance",
            expected=0.9,
            actual=0.7
        )

        assert error.details["metric"] == "instruction_compliance"
        assert error.details["expected"] == 0.9
        assert error.details["actual"] == 0.7


class TestNegotiationError:
    """Tests for NegotiationError."""

    def test_negotiation_error(self):
        error = NegotiationError(
            message="No common STypes",
            client_stypes=["org.calendar.Event.v1"],
            server_stypes=["org.other.Type.v1"],
            reason="No intersection"
        )

        assert error.code == "E-HANDSHAKE-FAILED"
        assert "org.calendar.Event.v1" in error.client_stypes
        assert "org.other.Type.v1" in error.server_stypes
        assert error.reason == "No intersection"

    def test_negotiation_error_defaults(self):
        error = NegotiationError(message="Failed")

        assert error.client_stypes == []
        assert error.server_stypes == []


class TestUnknownStypeError:
    """Tests for UnknownStypeError."""

    def test_unknown_stype_error(self):
        error = UnknownStypeError(
            stype="org.unknown.Type.v1",
            registry_path="./registry"
        )

        assert error.code == "E-UNKNOWN-STYPE"
        assert error.stype == "org.unknown.Type.v1"
        assert "Unknown SType" in error.message

    def test_unknown_stype_details(self):
        error = UnknownStypeError(stype="org.test.Type.v1")

        assert error.details["stype"] == "org.test.Type.v1"


class TestHashMismatchError:
    """Tests for HashMismatchError."""

    def test_hash_mismatch_error(self):
        error = HashMismatchError(
            expected="abc123" + "0" * 58,
            actual="def456" + "0" * 58,
            stype="org.test.Type.v1"
        )

        assert error.code == "E-HASH-MISMATCH"
        assert "abc123" in error.expected
        assert "def456" in error.actual
        assert "mismatch" in error.message.lower()


class TestConnectionError:
    """Tests for ConnectionError."""

    def test_connection_error(self):
        error = ConnectionError(
            message="Connection refused",
            endpoint="ws://localhost:8080",
            cause="ECONNREFUSED"
        )

        assert error.code == "E-CONNECTION-FAILED"
        assert error.endpoint == "ws://localhost:8080"
        assert error.cause == "ECONNREFUSED"


class TestTimeoutError:
    """Tests for TimeoutError."""

    def test_timeout_error(self):
        error = TimeoutError(
            message="Request timed out",
            timeout_ms=30000,
            operation="handshake"
        )

        assert error.code == "E-TIMEOUT"
        assert error.timeout_ms == 30000
        assert error.operation == "handshake"


class TestErrorInheritance:
    """Tests for error inheritance hierarchy."""

    def test_all_errors_inherit_from_mpl_error(self):
        errors = [
            SchemaFidelityError("test", "stype"),
            QomBreachError("test", "metric", 1.0, 0.5),
            NegotiationError("test"),
            UnknownStypeError("stype"),
            HashMismatchError("a", "b"),
            ConnectionError("test", "endpoint"),
            TimeoutError("test", 1000),
        ]

        for error in errors:
            assert isinstance(error, MplError)
            assert isinstance(error, Exception)

    def test_errors_are_catchable_as_mpl_error(self):
        def raise_schema_error():
            raise SchemaFidelityError("test", "stype")

        with pytest.raises(MplError):
            raise_schema_error()

    def test_errors_have_to_dict(self):
        error = QomBreachError("test", "metric", 1.0, 0.5)
        d = error.to_dict()

        assert "code" in d
        assert "message" in d
        assert "details" in d
