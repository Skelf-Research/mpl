//! QoM Event Recording and History
//!
//! Tracks QoM evaluations, persists events to disk, and maintains history for trends.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use mpl_core::determinism::{
    DeterminismChecker, DeterminismConfig, DeterminismResult, RequestSignature,
};
use mpl_core::groundedness::{
    GroundednessChecker, GroundednessConfig, GroundednessResult, SourceDocument,
};
use mpl_core::ontology::{Ontology, OntologyChecker, OntologyResult};

/// A single QoM evaluation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// SType being evaluated
    pub stype: String,
    /// Profile used for evaluation
    pub profile: String,
    /// Whether the evaluation passed
    pub passed: bool,
    /// Individual metric scores
    pub scores: QomScores,
    /// Failure reason if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    /// Request payload hash (for determinism tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hash: Option<String>,
}

/// Individual QoM metric scores
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QomScores {
    /// Schema Fidelity (1.0 = valid, 0.0 = invalid)
    pub sf: Option<f64>,
    /// Instruction Compliance
    pub ic: Option<f64>,
    /// Tool Outcome Correctness
    pub toc: Option<f64>,
    /// Groundedness
    pub g: Option<f64>,
    /// Determinism Jitter (1 - jitter, so 1.0 = stable)
    pub dj: Option<f64>,
    /// Ontology Adherence
    pub oa: Option<f64>,
}

/// Aggregated history point for trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QomHistoryPoint {
    /// Timestamp for this aggregation point
    pub timestamp: DateTime<Utc>,
    /// Number of events in this period
    pub count: usize,
    /// Average scores
    pub sf: f64,
    pub ic: f64,
    pub toc: f64,
    pub g: f64,
    pub dj: f64,
    pub oa: f64,
    /// Pass rate
    pub pass_rate: f64,
}

/// Summary statistics for QoM metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QomSummary {
    pub schema_fidelity: MetricSummary,
    pub instruction_compliance: MetricSummary,
    pub tool_outcome_correctness: MetricSummary,
    pub groundedness: MetricSummary,
    pub determinism_jitter: MetricSummary,
    pub ontology_adherence: MetricSummary,
}

/// Summary for a single metric
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricSummary {
    pub score: Option<f64>,
    pub samples: usize,
    pub failures: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<usize>,
}

/// Configuration for the QoM recorder
#[derive(Debug, Clone)]
pub struct QomRecorderConfig {
    /// Directory to store QoM data
    pub data_dir: PathBuf,
    /// Maximum number of events to keep in memory
    pub max_events_memory: usize,
    /// Maximum number of events to keep on disk
    pub max_events_disk: usize,
    /// History aggregation interval
    pub history_interval: Duration,
    /// Whether to enable groundedness checking
    pub enable_groundedness: bool,
    /// Whether to enable determinism checking
    pub enable_determinism: bool,
    /// Whether to enable ontology checking
    pub enable_ontology: bool,
}

impl Default for QomRecorderConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".mpl/qom"),
            max_events_memory: 1000,
            max_events_disk: 10000,
            history_interval: Duration::minutes(5),
            enable_groundedness: true,
            enable_determinism: true,
            enable_ontology: true,
        }
    }
}

/// QoM recorder - tracks events, computes metrics, persists to disk
pub struct QomRecorder {
    config: QomRecorderConfig,
    /// Recent events (in memory)
    events: Arc<RwLock<VecDeque<QomEvent>>>,
    /// Running totals for summary
    totals: Arc<RwLock<QomTotals>>,
    /// Groundedness checker
    groundedness_checker: GroundednessChecker,
    /// Determinism checker (tracks response history)
    determinism_checker: Arc<RwLock<DeterminismChecker>>,
    /// Ontology specs by SType
    ontology_specs: Arc<RwLock<std::collections::HashMap<String, Ontology>>>,
    /// Event counter for ID generation
    event_counter: std::sync::atomic::AtomicU64,
}

/// Running totals for summary computation
#[derive(Debug, Default)]
struct QomTotals {
    sf_sum: f64,
    sf_count: usize,
    sf_failures: usize,
    ic_sum: f64,
    ic_count: usize,
    ic_failures: usize,
    toc_sum: f64,
    toc_count: usize,
    toc_failures: usize,
    toc_pending: usize,
    g_sum: f64,
    g_count: usize,
    g_failures: usize,
    dj_sum: f64,
    dj_count: usize,
    dj_failures: usize,
    oa_sum: f64,
    oa_count: usize,
    oa_failures: usize,
}

impl QomRecorder {
    /// Create a new QoM recorder
    pub fn new(config: QomRecorderConfig) -> Self {
        // Ensure data directory exists
        if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
            warn!("Failed to create QoM data directory: {}", e);
        }

        Self {
            config,
            events: Arc::new(RwLock::new(VecDeque::new())),
            totals: Arc::new(RwLock::new(QomTotals::default())),
            groundedness_checker: GroundednessChecker::new(GroundednessConfig::default()),
            determinism_checker: Arc::new(RwLock::new(DeterminismChecker::new(
                DeterminismConfig::default(),
            ))),
            ontology_specs: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Generate a unique event ID
    fn next_event_id(&self) -> String {
        let count = self
            .event_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("evt_{:016x}", count)
    }

    /// Record a QoM evaluation event
    pub async fn record_event(&self, event: QomEvent) {
        // Update totals
        {
            let mut totals = self.totals.write().await;
            if let Some(sf) = event.scores.sf {
                totals.sf_sum += sf;
                totals.sf_count += 1;
                if sf < 1.0 {
                    totals.sf_failures += 1;
                }
            }
            if let Some(ic) = event.scores.ic {
                totals.ic_sum += ic;
                totals.ic_count += 1;
                if ic < 0.97 {
                    totals.ic_failures += 1;
                }
            }
            if let Some(toc) = event.scores.toc {
                totals.toc_sum += toc;
                totals.toc_count += 1;
                if toc < 0.9 {
                    totals.toc_failures += 1;
                }
            }
            if let Some(g) = event.scores.g {
                totals.g_sum += g;
                totals.g_count += 1;
                if g < 0.8 {
                    totals.g_failures += 1;
                }
            }
            if let Some(dj) = event.scores.dj {
                totals.dj_sum += dj;
                totals.dj_count += 1;
                if dj < 0.9 {
                    totals.dj_failures += 1;
                }
            }
            if let Some(oa) = event.scores.oa {
                totals.oa_sum += oa;
                totals.oa_count += 1;
                if oa < 0.95 {
                    totals.oa_failures += 1;
                }
            }
        }

        // Add to in-memory events
        {
            let mut events = self.events.write().await;
            events.push_back(event.clone());
            while events.len() > self.config.max_events_memory {
                events.pop_front();
            }
        }

        // Persist to disk (append to events file)
        self.persist_event(&event).await;
    }

    /// Persist an event to disk
    async fn persist_event(&self, event: &QomEvent) {
        let events_file = self.config.data_dir.join("qom_events.jsonl");

        if let Ok(line) = serde_json::to_string(event) {
            use tokio::io::AsyncWriteExt;
            if let Ok(mut file) = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&events_file)
                .await
            {
                let _ = file.write_all(format!("{}\n", line).as_bytes()).await;
            }
        }
    }

    /// Create a QoM event from validation results
    pub fn create_event(
        &self,
        stype: &str,
        profile: &str,
        passed: bool,
        scores: QomScores,
        failure_reason: Option<String>,
        payload_hash: Option<String>,
    ) -> QomEvent {
        QomEvent {
            id: self.next_event_id(),
            timestamp: Utc::now(),
            stype: stype.to_string(),
            profile: profile.to_string(),
            passed,
            scores,
            failure_reason,
            payload_hash,
        }
    }

    /// Check groundedness of a response
    pub fn check_groundedness(
        &self,
        response: &str,
        sources: &[SourceDocument],
    ) -> GroundednessResult {
        if !self.config.enable_groundedness {
            return GroundednessResult {
                score: 1.0,
                total_claims: 0,
                grounded_claims: 0,
                ungrounded_claims: 0,
                needs_review_count: 0,
                claim_results: vec![],
                method: mpl_core::groundedness::GroundingMethod::Skipped,
            };
        }

        self.groundedness_checker.check(response, sources, None)
    }

    /// Check determinism of a response
    pub async fn check_determinism(
        &self,
        stype: &str,
        payload_hash: &str,
        response: &serde_json::Value,
    ) -> DeterminismResult {
        if !self.config.enable_determinism {
            return DeterminismResult {
                similarity: 1.0,
                is_deterministic: true,
                differences: vec![],
                comparison_count: 0,
                average_similarity: 1.0,
                jitter: 0.0,
            };
        }

        let signature = RequestSignature {
            stype: stype.to_string(),
            payload_hash: payload_hash.to_string(),
            tool_name: None,
        };

        let mut checker = self.determinism_checker.write().await;
        checker.check_and_record(&signature, response)
    }

    /// Check ontology adherence
    pub async fn check_ontology(&self, stype: &str, payload: &serde_json::Value) -> OntologyResult {
        if !self.config.enable_ontology {
            return OntologyResult {
                adheres: true,
                score: 1.0,
                violations: vec![],
                constraints_checked: 0,
                violation_count: 0,
                error_count: 0,
                warning_count: 0,
            };
        }

        let specs = self.ontology_specs.read().await;
        if let Some(spec) = specs.get(stype) {
            let checker = OntologyChecker::new(spec.clone());
            return checker.check(payload);
        }

        // No ontology spec for this SType
        OntologyResult {
            adheres: true,
            score: 1.0,
            violations: vec![],
            constraints_checked: 0,
            violation_count: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Load ontology spec for an SType
    pub async fn load_ontology(&self, stype: &str, spec: Ontology) {
        let mut specs = self.ontology_specs.write().await;
        specs.insert(stype.to_string(), spec);
    }

    /// Get QoM summary statistics
    pub async fn get_summary(&self) -> QomSummary {
        let t = self.totals.read().await;

        QomSummary {
            schema_fidelity: MetricSummary {
                score: if t.sf_count > 0 {
                    Some(t.sf_sum / t.sf_count as f64)
                } else {
                    None
                },
                samples: t.sf_count,
                failures: t.sf_failures,
                pending: None,
            },
            instruction_compliance: MetricSummary {
                score: if t.ic_count > 0 {
                    Some(t.ic_sum / t.ic_count as f64)
                } else {
                    None
                },
                samples: t.ic_count,
                failures: t.ic_failures,
                pending: None,
            },
            tool_outcome_correctness: MetricSummary {
                score: if t.toc_count > 0 {
                    Some(t.toc_sum / t.toc_count as f64)
                } else {
                    None
                },
                samples: t.toc_count,
                failures: t.toc_failures,
                pending: Some(t.toc_pending),
            },
            groundedness: MetricSummary {
                score: if t.g_count > 0 {
                    Some(t.g_sum / t.g_count as f64)
                } else {
                    None
                },
                samples: t.g_count,
                failures: t.g_failures,
                pending: None,
            },
            determinism_jitter: MetricSummary {
                score: if t.dj_count > 0 {
                    Some(t.dj_sum / t.dj_count as f64)
                } else {
                    None
                },
                samples: t.dj_count,
                failures: t.dj_failures,
                pending: None,
            },
            ontology_adherence: MetricSummary {
                score: if t.oa_count > 0 {
                    Some(t.oa_sum / t.oa_count as f64)
                } else {
                    None
                },
                samples: t.oa_count,
                failures: t.oa_failures,
                pending: None,
            },
        }
    }

    /// Get recent events
    pub async fn get_events(&self, limit: usize) -> Vec<QomEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get history for a time period
    pub async fn get_history(&self, period: &str) -> Vec<QomHistoryPoint> {
        let now = Utc::now();
        let (duration, points) = match period {
            "1h" => (Duration::hours(1), 12),
            "6h" => (Duration::hours(6), 12),
            "7d" => (Duration::days(7), 14),
            _ => (Duration::hours(24), 24), // 24h default
        };

        // Load history from disk or compute from events
        let history_file = self.config.data_dir.join("qom_history.json");

        if history_file.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&history_file).await {
                if let Ok(history) = serde_json::from_str::<Vec<QomHistoryPoint>>(&content) {
                    // Filter to requested period
                    let cutoff = now - duration;
                    return history
                        .into_iter()
                        .filter(|p| p.timestamp > cutoff)
                        .collect();
                }
            }
        }

        // Generate from in-memory events
        self.compute_history_from_events(duration, points).await
    }

    /// Compute history points from in-memory events
    async fn compute_history_from_events(
        &self,
        duration: Duration,
        points: usize,
    ) -> Vec<QomHistoryPoint> {
        let now = Utc::now();
        let interval = duration / points as i32;
        let mut history = Vec::with_capacity(points);

        let events = self.events.read().await;

        for i in 0..points {
            let point_start = now - duration + interval * i as i32;
            let point_end = point_start + interval;

            let mut sf_sum = 0.0;
            let mut ic_sum = 0.0;
            let mut toc_sum = 0.0;
            let mut g_sum = 0.0;
            let mut dj_sum = 0.0;
            let mut oa_sum = 0.0;
            let mut pass_count = 0;
            let mut total_count = 0;
            let mut sf_count = 0;
            let mut ic_count = 0;
            let mut toc_count = 0;
            let mut g_count = 0;
            let mut dj_count = 0;
            let mut oa_count = 0;

            for event in events.iter() {
                if event.timestamp >= point_start && event.timestamp < point_end {
                    total_count += 1;
                    if event.passed {
                        pass_count += 1;
                    }
                    if let Some(sf) = event.scores.sf {
                        sf_sum += sf;
                        sf_count += 1;
                    }
                    if let Some(ic) = event.scores.ic {
                        ic_sum += ic;
                        ic_count += 1;
                    }
                    if let Some(toc) = event.scores.toc {
                        toc_sum += toc;
                        toc_count += 1;
                    }
                    if let Some(g) = event.scores.g {
                        g_sum += g;
                        g_count += 1;
                    }
                    if let Some(dj) = event.scores.dj {
                        dj_sum += dj;
                        dj_count += 1;
                    }
                    if let Some(oa) = event.scores.oa {
                        oa_sum += oa;
                        oa_count += 1;
                    }
                }
            }

            history.push(QomHistoryPoint {
                timestamp: point_start,
                count: total_count,
                sf: if sf_count > 0 {
                    sf_sum / sf_count as f64
                } else {
                    1.0
                },
                ic: if ic_count > 0 {
                    ic_sum / ic_count as f64
                } else {
                    0.0
                },
                toc: if toc_count > 0 {
                    toc_sum / toc_count as f64
                } else {
                    0.0
                },
                g: if g_count > 0 {
                    g_sum / g_count as f64
                } else {
                    0.0
                },
                dj: if dj_count > 0 {
                    dj_sum / dj_count as f64
                } else {
                    0.0
                },
                oa: if oa_count > 0 {
                    oa_sum / oa_count as f64
                } else {
                    0.0
                },
                pass_rate: if total_count > 0 {
                    pass_count as f64 / total_count as f64
                } else {
                    1.0
                },
            });
        }

        history
    }

    /// Increment TOC pending count
    pub async fn inc_toc_pending(&self) {
        let mut totals = self.totals.write().await;
        totals.toc_pending += 1;
    }

    /// Decrement TOC pending count
    pub async fn dec_toc_pending(&self) {
        let mut totals = self.totals.write().await;
        if totals.toc_pending > 0 {
            totals.toc_pending -= 1;
        }
    }

    /// Persist history to disk (should be called periodically)
    pub async fn persist_history(&self) {
        let history = self
            .compute_history_from_events(Duration::days(7), 168)
            .await; // 7 days, hourly
        let history_file = self.config.data_dir.join("qom_history.json");

        if let Ok(content) = serde_json::to_string_pretty(&history) {
            if let Err(e) = tokio::fs::write(&history_file, content).await {
                warn!("Failed to persist QoM history: {}", e);
            } else {
                debug!("Persisted {} history points", history.len());
            }
        }
    }

    /// Load events from disk on startup
    pub async fn load_from_disk(&self) -> anyhow::Result<()> {
        let events_file = self.config.data_dir.join("qom_events.jsonl");

        if !events_file.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&events_file).await?;
        let mut loaded = 0;

        let lines: Vec<&str> = content.lines().collect();
        let mut events = self.events.write().await;

        for line in lines.iter().rev().take(self.config.max_events_memory) {
            if let Ok(event) = serde_json::from_str::<QomEvent>(line) {
                events.push_front(event);
                loaded += 1;
            }
        }

        debug!("Loaded {} QoM events from disk", loaded);
        Ok(())
    }
}

impl Default for QomRecorder {
    fn default() -> Self {
        Self::new(QomRecorderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_event() {
        let recorder = QomRecorder::new(QomRecorderConfig {
            data_dir: PathBuf::from("/tmp/mpl_test_qom"),
            ..Default::default()
        });

        let event = recorder.create_event(
            "org.test.Type.v1",
            "qom-basic",
            true,
            QomScores {
                sf: Some(1.0),
                ic: Some(0.95),
                ..Default::default()
            },
            None,
            None,
        );

        recorder.record_event(event).await;

        let summary = recorder.get_summary().await;
        assert_eq!(summary.schema_fidelity.samples, 1);
        assert_eq!(summary.instruction_compliance.samples, 1);
    }

    #[tokio::test]
    async fn test_get_events() {
        let recorder = QomRecorder::new(QomRecorderConfig {
            data_dir: PathBuf::from("/tmp/mpl_test_qom2"),
            ..Default::default()
        });

        for i in 0..5 {
            let event = recorder.create_event(
                &format!("org.test.Type{}.v1", i),
                "qom-basic",
                true,
                QomScores::default(),
                None,
                None,
            );
            recorder.record_event(event).await;
        }

        let events = recorder.get_events(3).await;
        assert_eq!(events.len(), 3);
    }
}
