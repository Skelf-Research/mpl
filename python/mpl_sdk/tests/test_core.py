"""
Tests for MPL SDK core functionality.
"""

import json
import pytest

from mpl_sdk import (
    SType,
    MplEnvelope,
    SchemaValidator,
    ValidationResult,
    QomMetrics,
    QomProfile,
    QomEvaluation,
    canonicalize,
    semantic_hash,
    verify_hash,
)


class TestSType:
    """Tests for SType parsing and manipulation."""

    def test_parse_valid_stype(self):
        stype = SType("org.calendar.Event.v1")
        assert stype.namespace == "org"
        assert stype.domain == "calendar"
        assert stype.name == "Event"
        assert stype.major_version == 1

    def test_parse_complex_stype(self):
        stype = SType("com.acme.finance.InvestmentRecommendation.v2")
        # Namespace includes all but the last two segments (name.vN)
        assert stype.namespace == "com.acme"
        assert stype.domain == "finance"
        assert stype.name == "InvestmentRecommendation"
        assert stype.major_version == 2

    def test_stype_id(self):
        stype = SType("org.calendar.Event.v1")
        assert stype.id() == "org.calendar.Event.v1"

    def test_stype_urn(self):
        stype = SType("org.calendar.Event.v1")
        urn = stype.urn()
        assert "org" in urn
        assert "calendar" in urn
        assert "Event" in urn

    def test_stype_registry_path(self):
        stype = SType("org.calendar.Event.v1")
        path = stype.registry_path()
        assert "org" in path
        assert "calendar" in path
        assert "Event" in path

    def test_stype_str(self):
        stype = SType("org.calendar.Event.v1")
        assert str(stype) == "org.calendar.Event.v1"

    def test_stype_repr(self):
        stype = SType("org.calendar.Event.v1")
        assert "SType" in repr(stype)
        assert "org.calendar.Event.v1" in repr(stype)

    def test_invalid_stype_raises(self):
        with pytest.raises(ValueError):
            SType("invalid")

    def test_invalid_version_raises(self):
        with pytest.raises(ValueError):
            SType("org.calendar.Event.vX")


class TestSchemaValidator:
    """Tests for schema validation."""

    @pytest.fixture
    def validator(self):
        v = SchemaValidator()
        schema = json.dumps({
            "type": "object",
            "properties": {
                "title": {"type": "string"},
                "start": {"type": "string", "format": "date-time"},
                "attendees": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["title", "start"]
        })
        v.register("org.calendar.Event.v1", schema)
        return v

    def test_register_schema(self, validator):
        assert validator.has_schema("org.calendar.Event.v1")

    def test_validate_valid_payload(self, validator):
        payload = json.dumps({
            "title": "Team Meeting",
            "start": "2024-01-15T10:00:00Z",
            "attendees": ["alice@example.com"]
        })
        result = validator.validate("org.calendar.Event.v1", payload)
        assert result.valid
        assert len(result.errors) == 0

    def test_validate_invalid_payload(self, validator):
        payload = json.dumps({
            "title": 123,  # Should be string
            "start": "2024-01-15T10:00:00Z"
        })
        result = validator.validate("org.calendar.Event.v1", payload)
        assert not result.valid
        assert len(result.errors) > 0

    def test_validate_missing_required(self, validator):
        payload = json.dumps({
            "title": "Meeting"
            # Missing "start"
        })
        result = validator.validate("org.calendar.Event.v1", payload)
        assert not result.valid

    def test_validate_or_raise_valid(self, validator):
        payload = json.dumps({
            "title": "Meeting",
            "start": "2024-01-15T10:00:00Z"
        })
        # Should not raise
        validator.validate_or_raise("org.calendar.Event.v1", payload)

    def test_validate_or_raise_invalid(self, validator):
        payload = json.dumps({"title": "Meeting"})
        with pytest.raises(ValueError):
            validator.validate_or_raise("org.calendar.Event.v1", payload)

    def test_registered_stypes(self, validator):
        stypes = validator.registered_stypes()
        assert "org.calendar.Event.v1" in stypes


class TestValidationResult:
    """Tests for ValidationResult."""

    def test_validation_result_bool_true(self):
        v = SchemaValidator()
        v.register("test.Type.v1", json.dumps({"type": "object"}))
        result = v.validate("test.Type.v1", json.dumps({}))
        assert bool(result) is True

    def test_validation_result_repr(self):
        v = SchemaValidator()
        v.register("test.Type.v1", json.dumps({"type": "object"}))
        result = v.validate("test.Type.v1", json.dumps({}))
        assert "ValidationResult" in repr(result)


class TestSemanticHash:
    """Tests for semantic hashing."""

    def test_canonicalize_orders_keys(self):
        # Different key orders should produce same canonical form
        json1 = json.dumps({"b": 2, "a": 1})
        json2 = json.dumps({"a": 1, "b": 2})

        canon1 = canonicalize(json1)
        canon2 = canonicalize(json2)

        assert canon1 == canon2

    def test_semantic_hash_deterministic(self):
        payload = json.dumps({"title": "Meeting", "count": 5})

        hash1 = semantic_hash(payload)
        hash2 = semantic_hash(payload)

        assert hash1 == hash2

    def test_semantic_hash_different_key_order(self):
        # Same content, different key order should produce same hash
        json1 = json.dumps({"b": 2, "a": 1})
        json2 = json.dumps({"a": 1, "b": 2})

        hash1 = semantic_hash(json1)
        hash2 = semantic_hash(json2)

        assert hash1 == hash2

    def test_verify_hash_valid(self):
        payload = json.dumps({"title": "Meeting"})
        hash_val = semantic_hash(payload)

        assert verify_hash(payload, hash_val)

    def test_verify_hash_invalid(self):
        payload = json.dumps({"title": "Meeting"})

        # Wrong hash
        assert not verify_hash(payload, "0" * 64)

    def test_hash_format(self):
        payload = json.dumps({"test": "data"})
        hash_val = semantic_hash(payload)

        # Hash format: "b3:" prefix + 64 hex characters (BLAKE3)
        assert hash_val.startswith("b3:")
        hex_part = hash_val[3:]
        assert len(hex_part) == 64
        assert all(c in "0123456789abcdef" for c in hex_part)


class TestQomMetrics:
    """Tests for QoM metrics."""

    def test_create_metrics(self):
        metrics = QomMetrics(
            schema_fidelity=1.0,
            instruction_compliance=0.95
        )
        assert metrics.schema_fidelity == 1.0
        assert metrics.instruction_compliance == 0.95

    def test_metrics_defaults(self):
        metrics = QomMetrics()
        assert metrics.schema_fidelity == 1.0
        assert metrics.instruction_compliance is None

    def test_schema_valid_factory(self):
        metrics = QomMetrics.schema_valid()
        assert metrics.schema_fidelity == 1.0

    def test_schema_invalid_factory(self):
        metrics = QomMetrics.schema_invalid()
        assert metrics.schema_fidelity == 0.0

    def test_metrics_to_dict(self):
        metrics = QomMetrics(
            schema_fidelity=1.0,
            instruction_compliance=0.9
        )
        d = metrics.to_dict()
        assert d["schema_fidelity"] == 1.0
        assert d["instruction_compliance"] == 0.9

    def test_metrics_repr(self):
        metrics = QomMetrics(schema_fidelity=0.95)
        assert "QomMetrics" in repr(metrics)


class TestQomProfile:
    """Tests for QoM profiles."""

    def test_basic_profile(self):
        profile = QomProfile.basic()
        assert profile.name == "qom-basic"

    def test_strict_argcheck_profile(self):
        profile = QomProfile.strict_argcheck()
        assert profile.name == "qom-strict-argcheck"

    def test_profile_evaluate_pass(self):
        profile = QomProfile.basic()
        metrics = QomMetrics(schema_fidelity=1.0)

        evaluation = profile.evaluate(metrics)

        assert evaluation.meets_profile
        assert len(evaluation.failures) == 0

    def test_profile_evaluate_fail(self):
        profile = QomProfile.basic()
        metrics = QomMetrics(schema_fidelity=0.0)

        evaluation = profile.evaluate(metrics)

        assert not evaluation.meets_profile
        assert len(evaluation.failures) > 0

    def test_strict_profile_requires_ic(self):
        profile = QomProfile.strict_argcheck()
        metrics = QomMetrics(
            schema_fidelity=1.0,
            instruction_compliance=0.5  # Below threshold
        )

        evaluation = profile.evaluate(metrics)

        assert not evaluation.meets_profile

    def test_profile_repr(self):
        profile = QomProfile.basic()
        assert "QomProfile" in repr(profile)
        assert "basic" in repr(profile)


class TestQomEvaluation:
    """Tests for QoM evaluation results."""

    def test_evaluation_bool(self):
        profile = QomProfile.basic()
        metrics = QomMetrics(schema_fidelity=1.0)
        evaluation = profile.evaluate(metrics)

        assert bool(evaluation) is True

    def test_evaluation_repr(self):
        profile = QomProfile.basic()
        metrics = QomMetrics(schema_fidelity=1.0)
        evaluation = profile.evaluate(metrics)

        assert "QomEvaluation" in repr(evaluation)


class TestMplEnvelope:
    """Tests for MPL Envelope."""

    def test_create_envelope(self):
        payload = json.dumps({"title": "Meeting", "start": "2024-01-15T10:00:00Z"})
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=payload
        )

        assert envelope.stype == "org.calendar.Event.v1"
        assert envelope.payload == payload
        assert envelope.id is not None

    def test_envelope_with_profile(self):
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps({"title": "Meeting"}),
            profile="qom-basic"
        )

        assert envelope.profile == "qom-basic"

    def test_envelope_compute_hash(self):
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps({"title": "Meeting"})
        )

        hash_val = envelope.compute_hash()

        assert envelope.sem_hash == hash_val
        # Hash format: "b3:" prefix + 64 hex characters
        assert hash_val.startswith("b3:")
        assert len(hash_val) == 67  # 3 + 64

    def test_envelope_verify_hash(self):
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps({"title": "Meeting"})
        )
        envelope.compute_hash()

        assert envelope.verify_hash()

    def test_envelope_get_payload(self):
        data = {"title": "Meeting", "count": 5}
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps(data)
        )

        payload = envelope.get_payload()

        assert payload["title"] == "Meeting"
        assert payload["count"] == 5

    def test_envelope_to_json(self):
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps({"title": "Meeting"})
        )

        json_str = envelope.to_json()
        parsed = json.loads(json_str)

        assert parsed["stype"] == "org.calendar.Event.v1"
        assert parsed["payload"]["title"] == "Meeting"

    def test_envelope_invalid_json_raises(self):
        with pytest.raises(ValueError):
            MplEnvelope(
                stype="org.calendar.Event.v1",
                payload="not valid json"
            )

    def test_envelope_repr(self):
        envelope = MplEnvelope(
            stype="org.calendar.Event.v1",
            payload=json.dumps({})
        )

        assert "MplEnvelope" in repr(envelope)
        assert envelope.id in repr(envelope)
