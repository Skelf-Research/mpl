//! Python bindings for MPL core library
//!
//! Exposes the core MPL primitives to Python via PyO3.

// PyO3 wrappers commonly use `.map_err(|e| PyValueError::new_err(...))?` which
// converts PyErr -> PyErr via From (identity); clippy flags it as useless. The
// pattern is the standard PyO3 idiom for surfacing Rust errors as Python ones.
#![allow(clippy::useless_conversion)]
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
use std::collections::HashMap;

use mpl_core::{
    hash::{canonicalize as rust_canonicalize, semantic_hash as rust_semantic_hash},
    qom::{QomMetrics as RustQomMetrics, QomProfile as RustQomProfile},
    stype::SType as RustSType,
    validation::SchemaValidator as RustSchemaValidator,
};

/// Semantic Type (SType) - globally unique, versioned identifier
#[pyclass(name = "SType", from_py_object)]
#[derive(Clone)]
pub struct PySType {
    inner: RustSType,
}

#[pymethods]
impl PySType {
    /// Parse an SType from a string
    #[new]
    fn new(stype_str: &str) -> PyResult<Self> {
        RustSType::parse(stype_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Create an SType from components
    #[staticmethod]
    fn create(namespace: &str, domain: &str, name: &str, major_version: u32) -> Self {
        Self {
            inner: RustSType::new(namespace, domain, name, major_version),
        }
    }

    /// Get the namespace
    #[getter]
    fn namespace(&self) -> &str {
        &self.inner.namespace
    }

    /// Get the domain
    #[getter]
    fn domain(&self) -> &str {
        &self.inner.domain
    }

    /// Get the name
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get the major version
    #[getter]
    fn major_version(&self) -> u32 {
        self.inner.major_version
    }

    /// Get the short identifier
    fn id(&self) -> String {
        self.inner.id()
    }

    /// Get the full URN
    fn urn(&self) -> String {
        self.inner.urn()
    }

    /// Get the registry path
    fn registry_path(&self) -> String {
        self.inner.registry_path()
    }

    fn __str__(&self) -> String {
        self.inner.id()
    }

    fn __repr__(&self) -> String {
        format!("SType('{}')", self.inner.id())
    }
}

/// Schema Validator
#[pyclass(name = "SchemaValidator")]
pub struct PySchemaValidator {
    inner: RustSchemaValidator,
}

#[pymethods]
impl PySchemaValidator {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustSchemaValidator::new(),
        }
    }

    /// Register a schema for an SType
    fn register(&mut self, stype: &str, schema_json: &str) -> PyResult<()> {
        self.inner
            .register_json(stype, schema_json)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Check if a schema is registered
    fn has_schema(&self, stype: &str) -> bool {
        self.inner.has_schema(stype)
    }

    /// Validate a payload against an SType
    fn validate(&self, stype: &str, payload_json: &str) -> PyResult<PyValidationResult> {
        let payload: serde_json::Value = serde_json::from_str(payload_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        let result = self
            .inner
            .validate(stype, &payload)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyValidationResult {
            valid: result.valid,
            errors: result
                .errors
                .iter()
                .map(|e| PySchemaError {
                    path: e.path.clone(),
                    message: e.message.clone(),
                })
                .collect(),
        })
    }

    /// Validate and raise exception if invalid
    fn validate_or_raise(&self, stype: &str, payload_json: &str) -> PyResult<()> {
        let payload: serde_json::Value = serde_json::from_str(payload_json)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

        self.inner
            .validate_or_error(stype, &payload)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Get all registered STypes
    fn registered_stypes(&self) -> Vec<String> {
        self.inner
            .registered_stypes()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

/// Validation result
#[pyclass(name = "ValidationResult", from_py_object)]
#[derive(Clone)]
pub struct PyValidationResult {
    #[pyo3(get)]
    valid: bool,
    #[pyo3(get)]
    errors: Vec<PySchemaError>,
}

#[pymethods]
impl PyValidationResult {
    fn __bool__(&self) -> bool {
        self.valid
    }

    fn __repr__(&self) -> String {
        if self.valid {
            "ValidationResult(valid=True)".to_string()
        } else {
            format!(
                "ValidationResult(valid=False, errors={})",
                self.errors.len()
            )
        }
    }
}

/// Schema validation error
#[pyclass(name = "SchemaError", from_py_object)]
#[derive(Clone)]
pub struct PySchemaError {
    #[pyo3(get)]
    path: String,
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl PySchemaError {
    fn __repr__(&self) -> String {
        format!(
            "SchemaError(path='{}', message='{}')",
            self.path, self.message
        )
    }
}

/// QoM Metrics
#[pyclass(name = "QomMetrics", from_py_object)]
#[derive(Clone)]
pub struct PyQomMetrics {
    #[pyo3(get, set)]
    schema_fidelity: f64,
    #[pyo3(get, set)]
    instruction_compliance: Option<f64>,
    #[pyo3(get, set)]
    groundedness: Option<f64>,
    #[pyo3(get, set)]
    determinism_jitter: Option<f64>,
    #[pyo3(get, set)]
    ontology_adherence: Option<f64>,
    #[pyo3(get, set)]
    tool_outcome_correctness: Option<f64>,
}

#[pymethods]
impl PyQomMetrics {
    #[new]
    #[pyo3(signature = (schema_fidelity=1.0, instruction_compliance=None, groundedness=None, determinism_jitter=None, ontology_adherence=None, tool_outcome_correctness=None))]
    fn new(
        schema_fidelity: f64,
        instruction_compliance: Option<f64>,
        groundedness: Option<f64>,
        determinism_jitter: Option<f64>,
        ontology_adherence: Option<f64>,
        tool_outcome_correctness: Option<f64>,
    ) -> Self {
        Self {
            schema_fidelity,
            instruction_compliance,
            groundedness,
            determinism_jitter,
            ontology_adherence,
            tool_outcome_correctness,
        }
    }

    /// Create metrics for valid schema
    #[staticmethod]
    fn schema_valid() -> Self {
        Self {
            schema_fidelity: 1.0,
            instruction_compliance: None,
            groundedness: None,
            determinism_jitter: None,
            ontology_adherence: None,
            tool_outcome_correctness: None,
        }
    }

    /// Create metrics for invalid schema
    #[staticmethod]
    fn schema_invalid() -> Self {
        Self {
            schema_fidelity: 0.0,
            instruction_compliance: None,
            groundedness: None,
            determinism_jitter: None,
            ontology_adherence: None,
            tool_outcome_correctness: None,
        }
    }

    /// Convert to dictionary
    fn to_dict(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("schema_fidelity".to_string(), self.schema_fidelity);
        if let Some(ic) = self.instruction_compliance {
            map.insert("instruction_compliance".to_string(), ic);
        }
        if let Some(g) = self.groundedness {
            map.insert("groundedness".to_string(), g);
        }
        if let Some(dj) = self.determinism_jitter {
            map.insert("determinism_jitter".to_string(), dj);
        }
        if let Some(oa) = self.ontology_adherence {
            map.insert("ontology_adherence".to_string(), oa);
        }
        if let Some(toc) = self.tool_outcome_correctness {
            map.insert("tool_outcome_correctness".to_string(), toc);
        }
        map
    }

    fn __repr__(&self) -> String {
        format!("QomMetrics(schema_fidelity={:.2})", self.schema_fidelity)
    }
}

impl From<PyQomMetrics> for RustQomMetrics {
    fn from(py: PyQomMetrics) -> Self {
        RustQomMetrics {
            schema_fidelity: py.schema_fidelity,
            instruction_compliance: py.instruction_compliance,
            groundedness: py.groundedness,
            determinism_jitter: py.determinism_jitter,
            ontology_adherence: py.ontology_adherence,
            tool_outcome_correctness: py.tool_outcome_correctness,
        }
    }
}

/// QoM Profile
#[pyclass(name = "QomProfile", from_py_object)]
#[derive(Clone)]
pub struct PyQomProfile {
    inner: RustQomProfile,
}

#[pymethods]
impl PyQomProfile {
    /// Create a basic profile (Schema Fidelity only)
    #[staticmethod]
    fn basic() -> Self {
        Self {
            inner: RustQomProfile::basic(),
        }
    }

    /// Create a strict profile (SF + IC)
    #[staticmethod]
    fn strict_argcheck() -> Self {
        Self {
            inner: RustQomProfile::strict_argcheck(),
        }
    }

    /// Get profile name
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Get description
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// Evaluate metrics against this profile
    fn evaluate(&self, metrics: &PyQomMetrics) -> PyQomEvaluation {
        let rust_metrics: RustQomMetrics = metrics.clone().into();
        let eval = self.inner.evaluate(&rust_metrics);
        PyQomEvaluation {
            meets_profile: eval.meets_profile,
            profile: eval.profile,
            failures: eval
                .failures
                .iter()
                .map(|f| PyMetricFailure {
                    metric: f.metric.clone(),
                    actual: f.actual,
                    threshold: f.threshold,
                })
                .collect(),
        }
    }

    fn __repr__(&self) -> String {
        format!("QomProfile(name='{}')", self.inner.name)
    }
}

/// QoM Evaluation result
#[pyclass(name = "QomEvaluation", from_py_object)]
#[derive(Clone)]
pub struct PyQomEvaluation {
    #[pyo3(get)]
    meets_profile: bool,
    #[pyo3(get)]
    profile: String,
    #[pyo3(get)]
    failures: Vec<PyMetricFailure>,
}

#[pymethods]
impl PyQomEvaluation {
    fn __bool__(&self) -> bool {
        self.meets_profile
    }

    fn __repr__(&self) -> String {
        if self.meets_profile {
            format!(
                "QomEvaluation(meets_profile=True, profile='{}')",
                self.profile
            )
        } else {
            format!(
                "QomEvaluation(meets_profile=False, failures={})",
                self.failures.len()
            )
        }
    }
}

/// Metric failure
#[pyclass(name = "MetricFailure", from_py_object)]
#[derive(Clone)]
pub struct PyMetricFailure {
    #[pyo3(get)]
    metric: String,
    #[pyo3(get)]
    actual: f64,
    #[pyo3(get)]
    threshold: f64,
}

#[pymethods]
impl PyMetricFailure {
    fn __repr__(&self) -> String {
        format!(
            "MetricFailure(metric='{}', actual={:.2}, threshold={:.2})",
            self.metric, self.actual, self.threshold
        )
    }
}

/// Canonicalize a JSON payload
#[pyfunction]
fn canonicalize(json_str: &str) -> PyResult<String> {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

    rust_canonicalize(&value).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Compute semantic hash of a JSON payload
#[pyfunction]
fn semantic_hash(json_str: &str) -> PyResult<String> {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

    rust_semantic_hash(&value).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Verify semantic hash matches payload
#[pyfunction]
fn verify_hash(json_str: &str, expected_hash: &str) -> PyResult<bool> {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;

    mpl_core::hash::verify_hash(&value, expected_hash)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// MPL Envelope
#[pyclass(name = "MplEnvelope", from_py_object)]
#[derive(Clone)]
pub struct PyMplEnvelope {
    #[pyo3(get)]
    id: String,
    #[pyo3(get, set)]
    stype: String,
    #[pyo3(get, set)]
    payload: String, // JSON string
    #[pyo3(get, set)]
    args_stype: Option<String>,
    #[pyo3(get, set)]
    profile: Option<String>,
    #[pyo3(get, set)]
    sem_hash: Option<String>,
    #[pyo3(get, set)]
    features: Vec<String>,
}

#[pymethods]
impl PyMplEnvelope {
    #[new]
    #[pyo3(signature = (stype, payload, args_stype=None, profile=None))]
    fn new(
        stype: String,
        payload: String,
        args_stype: Option<String>,
        profile: Option<String>,
    ) -> PyResult<Self> {
        // Validate payload is valid JSON
        let _: serde_json::Value = serde_json::from_str(&payload)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON payload: {}", e)))?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            stype,
            payload,
            args_stype,
            profile,
            sem_hash: None,
            features: Vec::new(),
        })
    }

    /// Compute and set the semantic hash
    fn compute_hash(&mut self) -> PyResult<String> {
        let hash = semantic_hash(&self.payload)?;
        self.sem_hash = Some(hash.clone());
        Ok(hash)
    }

    /// Verify the semantic hash
    fn verify_hash(&self) -> PyResult<bool> {
        match &self.sem_hash {
            Some(expected) => verify_hash(&self.payload, expected),
            None => Ok(true),
        }
    }

    /// Get payload as Python dict
    fn get_payload(&self) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let value: serde_json::Value = serde_json::from_str(&self.payload)
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            json_to_py(py, &value)
        })
    }

    /// Convert to JSON string
    fn to_json(&self) -> PyResult<String> {
        let payload_value: serde_json::Value = serde_json::from_str(&self.payload)
            .map_err(|e| PyValueError::new_err(format!("Invalid payload JSON: {}", e)))?;
        let envelope = serde_json::json!({
            "id": self.id,
            "stype": self.stype,
            "payload": payload_value,
            "args_stype": self.args_stype,
            "profile": self.profile,
            "sem_hash": self.sem_hash,
            "features": self.features,
        });
        serde_json::to_string_pretty(&envelope).map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("MplEnvelope(id='{}', stype='{}')", self.id, self.stype)
    }
}

/// Convert serde_json::Value to Python object.
///
/// Uses pyo3 0.29's `IntoPyObjectExt::into_py_any` — the idiomatic
/// replacement for the removed `IntoPy::into_py`. PyDict construction uses
/// the renamed `PyDict::new` (was `PyDict::new_bound` in 0.22).
fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => b.into_py_any(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py_any(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py_any(py)
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => s.into_py_any(py),
        serde_json::Value::Array(arr) => {
            let list: Vec<Py<PyAny>> = arr
                .iter()
                .map(|v| json_to_py(py, v))
                .collect::<PyResult<_>>()?;
            list.into_py_any(py)
        }
        serde_json::Value::Object(map) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

/// Python module
#[pymodule]
fn _mpl_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySType>()?;
    m.add_class::<PySchemaValidator>()?;
    m.add_class::<PyValidationResult>()?;
    m.add_class::<PySchemaError>()?;
    m.add_class::<PyQomMetrics>()?;
    m.add_class::<PyQomProfile>()?;
    m.add_class::<PyQomEvaluation>()?;
    m.add_class::<PyMetricFailure>()?;
    m.add_class::<PyMplEnvelope>()?;
    m.add_function(wrap_pyfunction!(canonicalize, m)?)?;
    m.add_function(wrap_pyfunction!(semantic_hash, m)?)?;
    m.add_function(wrap_pyfunction!(verify_hash, m)?)?;

    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
