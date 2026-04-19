//! Metrics collection for the proxy

use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe metrics state
pub struct MetricsState {
    pub requests_total: AtomicU64,
    pub schema_pass: AtomicU64,
    pub schema_fail: AtomicU64,
    pub qom_pass: AtomicU64,
    pub qom_fail: AtomicU64,
    pub handshakes: AtomicU64,
    pub downgrades: AtomicU64,
}

impl MetricsState {
    /// Create a new metrics state
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            schema_pass: AtomicU64::new(0),
            schema_fail: AtomicU64::new(0),
            qom_pass: AtomicU64::new(0),
            qom_fail: AtomicU64::new(0),
            handshakes: AtomicU64::new(0),
            downgrades: AtomicU64::new(0),
        }
    }

    /// Increment total requests
    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment schema pass count
    pub fn inc_schema_pass(&self) {
        self.schema_pass.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment schema fail count
    pub fn inc_schema_fail(&self) {
        self.schema_fail.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment QoM pass count
    pub fn inc_qom_pass(&self) {
        self.qom_pass.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment QoM fail count
    pub fn inc_qom_fail(&self) {
        self.qom_fail.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment handshake count
    pub fn inc_handshakes(&self) {
        self.handshakes.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment downgrade count
    pub fn inc_downgrades(&self) {
        self.downgrades.fetch_add(1, Ordering::Relaxed);
    }

    /// Calculate schema pass rate
    pub fn schema_pass_rate(&self) -> f64 {
        let pass = self.schema_pass.load(Ordering::Relaxed);
        let fail = self.schema_fail.load(Ordering::Relaxed);
        let total = pass + fail;
        if total == 0 {
            1.0
        } else {
            pass as f64 / total as f64
        }
    }

    /// Calculate QoM pass rate
    pub fn qom_pass_rate(&self) -> f64 {
        let pass = self.qom_pass.load(Ordering::Relaxed);
        let fail = self.qom_fail.load(Ordering::Relaxed);
        let total = pass + fail;
        if total == 0 {
            1.0
        } else {
            pass as f64 / total as f64
        }
    }

    /// Calculate downgrade rate
    pub fn downgrade_rate(&self) -> f64 {
        let downgrades = self.downgrades.load(Ordering::Relaxed);
        let handshakes = self.handshakes.load(Ordering::Relaxed);
        if handshakes == 0 {
            0.0
        } else {
            downgrades as f64 / handshakes as f64
        }
    }
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}
