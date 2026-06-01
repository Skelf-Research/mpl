---
title: QoM
description: Reference for MPL Quality of Meaning evaluation - metrics, profiles, and threshold enforcement
---

# QoM

Quality of Meaning (QoM) classes provided by the Rust core. QoM defines measurable metrics that quantify how faithfully an AI agent's output adheres to its declared semantic contract. Profiles bundle metrics with threshold requirements.

```python
from mpl_sdk import QomMetrics, QomProfile, QomEvaluation, MetricFailure
```

---

## QomMetrics

```python
@dataclass
class QomMetrics:
    schema_fidelity: float | None = None
    instruction_compliance: float | None = None
    groundedness: float | None = None
    determinism: float | None = None
    ontology_adherence: float | None = None
    tool_outcome: float | None = None
```

A set of QoM metric values. Each metric is a float between 0.0 and 1.0 (or `None` if not measured).

### Fields

| Field | Type | Range | Description |
|-------|------|-------|-------------|
| `schema_fidelity` | `float \| None` | 0.0 - 1.0 | Measures whether the payload conforms to its declared JSON Schema. 1.0 means full conformance. |
| `instruction_compliance` | `float \| None` | 0.0 - 1.0 | Measures whether all required arguments/instructions were followed. |
| `groundedness` | `float \| None` | 0.0 - 1.0 | Measures whether the output is grounded in provided context (not hallucinated). |
| `determinism` | `float \| None` | 0.0 - 1.0 | Measures output consistency across repeated invocations with the same input. |
| `ontology_adherence` | `float \| None` | 0.0 - 1.0 | Measures adherence to domain ontology constraints beyond schema. |
| `tool_outcome` | `float \| None` | 0.0 - 1.0 | Measures whether the tool call achieved the intended outcome. |

### Example

```python
from mpl_sdk import QomMetrics

# Full metrics set
metrics = QomMetrics(
    schema_fidelity=1.0,
    instruction_compliance=0.98,
    groundedness=0.95,
    determinism=0.99,
    ontology_adherence=0.92,
    tool_outcome=1.0,
)

# Partial metrics (only what was measured)
metrics = QomMetrics(
    schema_fidelity=1.0,
    instruction_compliance=0.85,
)
```

---

## QomProfile

```python
class QomProfile:
    @staticmethod
    def basic() -> "QomProfile": ...
    @staticmethod
    def strict_argcheck() -> "QomProfile": ...
    def evaluate(self, metrics: QomMetrics) -> QomEvaluation: ...
```

A QoM profile defines a set of metric thresholds that must be met. Profiles are used during session negotiation to agree on quality requirements.

### Built-in Profiles

#### basic()

```python
@staticmethod
def basic() -> QomProfile
```

The basic QoM profile. Requires only Schema Fidelity.

| Metric | Threshold |
|--------|-----------|
| Schema Fidelity | = 1.0 |

```python
from mpl_sdk import QomProfile

profile = QomProfile.basic()
```

#### strict_argcheck()

```python
@staticmethod
def strict_argcheck() -> QomProfile
```

Strict argument checking profile. Requires Schema Fidelity and high Instruction Compliance.

| Metric | Threshold |
|--------|-----------|
| Schema Fidelity | = 1.0 |
| Instruction Compliance | >= 0.97 |

```python
from mpl_sdk import QomProfile

profile = QomProfile.strict_argcheck()
```

---

### evaluate()

```python
def evaluate(self, metrics: QomMetrics) -> QomEvaluation
```

Evaluate a set of metrics against this profile's thresholds. Returns a detailed evaluation result including pass/fail status and any threshold violations.

#### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `metrics` | `QomMetrics` | The metrics to evaluate |

#### Returns

[`QomEvaluation`](#qomevaluation) with the evaluation result.

#### Example

```python
from mpl_sdk import QomProfile, QomMetrics

profile = QomProfile.strict_argcheck()

# Passing metrics
metrics = QomMetrics(
    schema_fidelity=1.0,
    instruction_compliance=0.99,
)
evaluation = profile.evaluate(metrics)
print(evaluation.meets_profile)  # True
print(evaluation.failures)       # []

# Failing metrics
metrics = QomMetrics(
    schema_fidelity=1.0,
    instruction_compliance=0.85,  # Below 0.97 threshold
)
evaluation = profile.evaluate(metrics)
print(evaluation.meets_profile)  # False
print(evaluation.failures[0].metric)    # "instruction_compliance"
print(evaluation.failures[0].expected)  # 0.97
print(evaluation.failures[0].actual)    # 0.85
```

---

## QomEvaluation

```python
@dataclass
class QomEvaluation:
    meets_profile: bool
    metrics: QomMetrics
    failures: list[MetricFailure]
```

The result of evaluating metrics against a QoM profile.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `meets_profile` | `bool` | `True` if all metrics meet or exceed the profile thresholds |
| `metrics` | `QomMetrics` | The metrics that were evaluated |
| `failures` | `list[MetricFailure]` | List of metrics that failed to meet thresholds. Empty when `meets_profile` is `True`. |

### Example

```python
evaluation = profile.evaluate(metrics)

if evaluation.meets_profile:
    print("All QoM thresholds met!")
else:
    print(f"{len(evaluation.failures)} metric(s) below threshold:")
    for failure in evaluation.failures:
        print(f"  {failure.metric}: {failure.actual:.2f} < {failure.expected:.2f}")
```

---

## MetricFailure

```python
@dataclass
class MetricFailure:
    metric: str
    expected: float
    actual: float
```

Describes a single metric that failed to meet its threshold.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `metric` | `str` | Name of the metric that failed (e.g., `"instruction_compliance"`) |
| `expected` | `float` | The threshold value required by the profile |
| `actual` | `float` | The actual measured value |

---

## Complete Examples

### Evaluating Tool Call Quality

```python
from mpl_sdk import QomProfile, QomMetrics, QomEvaluation

def assess_tool_output(
    profile_name: str,
    schema_valid: bool,
    instruction_score: float,
) -> QomEvaluation:
    """Evaluate a tool call's quality against a named profile."""

    # Select profile
    if profile_name == "qom-strict-argcheck":
        profile = QomProfile.strict_argcheck()
    else:
        profile = QomProfile.basic()

    # Build metrics
    metrics = QomMetrics(
        schema_fidelity=1.0 if schema_valid else 0.0,
        instruction_compliance=instruction_score,
    )

    # Evaluate
    return profile.evaluate(metrics)

# Usage
evaluation = assess_tool_output(
    profile_name="qom-strict-argcheck",
    schema_valid=True,
    instruction_score=0.92,
)

if not evaluation.meets_profile:
    for failure in evaluation.failures:
        print(f"QoM breach: {failure.metric} = {failure.actual} (need {failure.expected})")
```

### Integrating QoM with Session

```python
import json
from mpl_sdk import (
    Session, SessionConfig, MplEnvelope,
    QomProfile, QomMetrics, SchemaValidator,
)
from mpl_sdk.errors import QomBreachError

async def send_with_qom_check(
    session: Session,
    stype: str,
    payload: dict,
    profile: QomProfile,
) -> MplEnvelope:
    """Send a payload and verify QoM on the response."""

    response = await session.send(stype=stype, payload=payload)

    # Extract QoM report from response if available
    if response.qom_report:
        metrics = QomMetrics(
            schema_fidelity=response.qom_report.get("schema_fidelity"),
            instruction_compliance=response.qom_report.get("instruction_compliance"),
            groundedness=response.qom_report.get("groundedness"),
        )

        evaluation = profile.evaluate(metrics)
        if not evaluation.meets_profile:
            raise QomBreachError(
                message=f"Response QoM below threshold",
                metric=evaluation.failures[0].metric,
                expected=evaluation.failures[0].expected,
                actual=evaluation.failures[0].actual,
                profile="qom-strict-argcheck",
            )

    return response
```

### QoM Profile Comparison

```python
from mpl_sdk import QomProfile, QomMetrics

# Same metrics, different profiles
metrics = QomMetrics(
    schema_fidelity=1.0,
    instruction_compliance=0.95,
    groundedness=0.88,
)

# Basic profile: only checks schema_fidelity
basic = QomProfile.basic()
basic_eval = basic.evaluate(metrics)
print(f"Basic profile: {'PASS' if basic_eval.meets_profile else 'FAIL'}")
# Output: Basic profile: PASS

# Strict profile: checks schema_fidelity AND instruction_compliance >= 0.97
strict = QomProfile.strict_argcheck()
strict_eval = strict.evaluate(metrics)
print(f"Strict profile: {'PASS' if strict_eval.meets_profile else 'FAIL'}")
# Output: Strict profile: FAIL

if not strict_eval.meets_profile:
    for f in strict_eval.failures:
        gap = f.expected - f.actual
        print(f"  {f.metric}: {f.actual:.2f} (need {f.expected:.2f}, gap: {gap:.2f})")
```

### Monitoring QoM Over Time

```python
import asyncio
from dataclasses import dataclass, field
from mpl_sdk import QomProfile, QomMetrics, QomEvaluation

@dataclass
class QomMonitor:
    """Track QoM metrics over multiple interactions."""

    profile: QomProfile
    history: list[QomEvaluation] = field(default_factory=list)

    def record(self, metrics: QomMetrics) -> QomEvaluation:
        """Record and evaluate a new set of metrics."""
        evaluation = self.profile.evaluate(metrics)
        self.history.append(evaluation)
        return evaluation

    @property
    def pass_rate(self) -> float:
        """Percentage of evaluations that met the profile."""
        if not self.history:
            return 0.0
        passed = sum(1 for e in self.history if e.meets_profile)
        return passed / len(self.history)

    @property
    def total_evaluations(self) -> int:
        return len(self.history)

    def summary(self) -> str:
        return (
            f"QoM Monitor: {self.total_evaluations} evaluations, "
            f"{self.pass_rate:.1%} pass rate"
        )

# Usage
monitor = QomMonitor(profile=QomProfile.strict_argcheck())

# Record metrics from each interaction
monitor.record(QomMetrics(schema_fidelity=1.0, instruction_compliance=0.99))
monitor.record(QomMetrics(schema_fidelity=1.0, instruction_compliance=0.98))
monitor.record(QomMetrics(schema_fidelity=1.0, instruction_compliance=0.85))
monitor.record(QomMetrics(schema_fidelity=0.0, instruction_compliance=0.99))

print(monitor.summary())
# Output: QoM Monitor: 4 evaluations, 50.0% pass rate
```
