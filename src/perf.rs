use crate::scan::ScanSummary;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const BUILT_IN_REFERENCE_LABEL: &str = "built-in ref";
const REFERENCE_JSON: &str = include_str!("../docs/benchmarks/reference.json");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedMetrics {
    pub total_ms: f64,
    pub ms_per_file: Option<f64>,
    pub entries_per_second: Option<f64>,
    pub bytes_per_second: Option<f64>,
}

impl SpeedMetrics {
    pub fn from_summary(summary: &ScanSummary) -> Self {
        let total_seconds = summary.duration.as_secs_f64();
        let total_ms = total_seconds * 1000.0;
        let entries = summary.total_entries();

        Self {
            total_ms,
            ms_per_file: (summary.total_files > 0).then(|| total_ms / summary.total_files as f64),
            entries_per_second: (entries > 0 && total_seconds > 0.0)
                .then(|| entries as f64 / total_seconds),
            bytes_per_second: (summary.total_size > 0 && total_seconds > 0.0)
                .then(|| summary.total_size as f64 / total_seconds),
        }
    }

    pub fn comparison_multiplier(&self, reference_entries_per_second: f64) -> Option<f64> {
        let current = self.entries_per_second?;
        (reference_entries_per_second.is_finite() && reference_entries_per_second > 0.0)
            .then_some(current / reference_entries_per_second)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReferenceArtifact {
    pub schema_version: u32,
    pub generated_at_epoch_seconds: u64,
    pub generation_context: BenchmarkGenerationContext,
    pub reference_label: String,
    pub reference_entries_per_second: f64,
    pub reference_bytes_per_second: f64,
    pub scenarios: Vec<ScenarioReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkGenerationContext {
    pub host: BenchmarkHost,
    pub git: BenchmarkGit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkHost {
    pub os: String,
    pub architecture: String,
    pub family: String,
    pub available_parallelism: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkGit {
    pub commit_sha: Option<String>,
    pub short_commit_sha: Option<String>,
    pub dirty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReference {
    pub slug: String,
    pub label: String,
    pub weight: f64,
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
    pub sffs_default_median_ms: f64,
    pub sffs_single_thread_median_ms: f64,
    pub sffs_best_profile: String,
    pub sffs_best_median_ms: f64,
    pub du_median_ms: f64,
    pub sffs_default_entries_per_second: f64,
    pub sffs_single_thread_entries_per_second: f64,
    pub sffs_best_entries_per_second: f64,
    pub du_entries_per_second: f64,
    pub sffs_vs_du_multiplier: f64,
}

pub fn built_in_reference() -> Option<&'static BenchmarkReferenceArtifact> {
    static REFERENCE: OnceLock<Option<BenchmarkReferenceArtifact>> = OnceLock::new();

    REFERENCE
        .get_or_init(|| serde_json::from_str(REFERENCE_JSON).ok())
        .as_ref()
}

pub fn format_speed_comparison(multiplier: f64) -> String {
    format!("{multiplier:.2}x vs {BUILT_IN_REFERENCE_LABEL}")
}

pub fn weighted_geometric_mean(weighted_values: &[(f64, f64)]) -> Option<f64> {
    if weighted_values.is_empty() {
        return None;
    }

    let mut weighted_log_sum = 0.0;
    let mut total_weight = 0.0;

    for (value, weight) in weighted_values {
        if !value.is_finite() || *value <= 0.0 || !weight.is_finite() || *weight <= 0.0 {
            return None;
        }
        weighted_log_sum += weight * value.ln();
        total_weight += weight;
    }

    (total_weight > 0.0).then(|| (weighted_log_sum / total_weight).exp())
}

impl ScanSummary {
    pub fn total_entries(&self) -> u64 {
        self.total_files + self.total_dirs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample_summary() -> ScanSummary {
        ScanSummary {
            total_size: 2_048,
            total_files: 4,
            total_dirs: 2,
            duration: Duration::from_millis(20),
            top_files: Vec::new(),
        }
    }

    #[test]
    fn computes_speed_metrics() {
        let metrics = SpeedMetrics::from_summary(&sample_summary());

        assert_eq!(metrics.total_ms, 20.0);
        assert_eq!(metrics.ms_per_file, Some(5.0));
        assert!(metrics.entries_per_second.unwrap() > 0.0);
        assert!(metrics.bytes_per_second.unwrap() > 0.0);
    }

    #[test]
    fn formats_speed_comparison() {
        assert_eq!(format_speed_comparison(1.25), "1.25x vs built-in ref");
    }

    #[test]
    fn parses_built_in_reference() {
        let reference = built_in_reference().expect("reference should parse");

        assert_eq!(reference.schema_version, 3);
        assert!(!reference.generation_context.host.os.is_empty());
        assert!(reference.reference_entries_per_second > 0.0);
        assert!(!reference.scenarios.is_empty());
    }

    #[test]
    fn computes_weighted_geometric_mean() {
        let result = weighted_geometric_mean(&[(100.0, 1.0), (400.0, 3.0)]).unwrap();

        assert!(result > 250.0);
        assert!(result < 400.0);
    }
}
