use crate::cli::Args;
use crate::perf::{
    weighted_geometric_mean, BenchmarkGenerationContext, BenchmarkGit, BenchmarkHost,
    BenchmarkReferenceArtifact, ScenarioReference,
};
use crate::scan::{collect_scan_summary, ScanSummary};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const BENCHMARK_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScenarioKind {
    TinyFiles,
    DeepTree,
    WideTree,
    LargeFiles,
    MixedTree,
}

impl ScenarioKind {
    pub const ALL: [Self; 5] = [
        Self::TinyFiles,
        Self::DeepTree,
        Self::WideTree,
        Self::LargeFiles,
        Self::MixedTree,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::TinyFiles => "tiny-files",
            Self::DeepTree => "deep-tree",
            Self::WideTree => "wide-tree",
            Self::LargeFiles => "large-files",
            Self::MixedTree => "mixed-tree",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TinyFiles => "Many tiny files",
            Self::DeepTree => "Deep directory tree",
            Self::WideTree => "Wide directory fan-out",
            Self::LargeFiles => "Few large files",
            Self::MixedTree => "Mixed realistic tree",
        }
    }

    pub fn weight(self) -> f64 {
        match self {
            Self::TinyFiles => 1.3,
            Self::DeepTree => 1.0,
            Self::WideTree => 1.2,
            Self::LargeFiles => 0.9,
            Self::MixedTree => 1.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureExpectation {
    pub slug: String,
    pub label: String,
    pub total_size: u64,
    pub total_files: u64,
    pub total_dirs: u64,
}

impl FixtureExpectation {
    pub fn total_entries(&self) -> u64 {
        self.total_files + self.total_dirs
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkConfig {
    pub warmup_iterations: usize,
    pub measurement_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 1,
            measurement_iterations: 6,
        }
    }
}

#[derive(Debug)]
pub struct BenchmarkFixture {
    root: PathBuf,
    pub expectation: FixtureExpectation,
}

impl BenchmarkFixture {
    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for BenchmarkFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn prepare_fixture(kind: ScenarioKind) -> io::Result<BenchmarkFixture> {
    let mut root = env::temp_dir();
    let epoch_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.push(format!(
        "sffs-bench-{}-{}-{}",
        kind.slug(),
        std::process::id(),
        epoch_nanos
    ));
    fs::create_dir_all(&root)?;
    let expectation = create_fixture_at(&root, kind)?;

    Ok(BenchmarkFixture { root, expectation })
}

pub fn create_fixture_at(root: &Path, kind: ScenarioKind) -> io::Result<FixtureExpectation> {
    let mut expectation = FixtureExpectation {
        slug: kind.slug().to_string(),
        label: kind.label().to_string(),
        total_size: 0,
        total_files: 0,
        total_dirs: 1,
    };

    fs::create_dir_all(root)?;

    match kind {
        ScenarioKind::TinyFiles => {
            for dir_index in 0..40 {
                let dir = root.join(format!("batch-{dir_index:02}"));
                fs::create_dir_all(&dir)?;
                expectation.total_dirs += 1;

                for file_index in 0..50 {
                    let size = 64 + ((dir_index + file_index) % 7) as usize * 8;
                    expectation.total_size += write_pattern_file(
                        &dir.join(format!("tiny-{file_index:03}.bin")),
                        size,
                        dir_index as u8,
                    )?;
                    expectation.total_files += 1;
                }
            }
        }
        ScenarioKind::DeepTree => {
            let mut cursor = root.to_path_buf();
            for depth in 0..32 {
                cursor = cursor.join(format!("level-{depth:02}"));
                fs::create_dir_all(&cursor)?;
                expectation.total_dirs += 1;

                let size = 256 + depth * 17;
                expectation.total_size +=
                    write_pattern_file(&cursor.join("payload.dat"), size, depth as u8)?;
                expectation.total_files += 1;
            }
        }
        ScenarioKind::WideTree => {
            for dir_index in 0..200 {
                let dir = root.join(format!("node-{dir_index:03}"));
                fs::create_dir_all(&dir)?;
                expectation.total_dirs += 1;

                for file_index in 0..5 {
                    let size = 384 + ((dir_index + file_index) % 11) as usize * 32;
                    expectation.total_size += write_pattern_file(
                        &dir.join(format!("file-{file_index:02}.dat")),
                        size,
                        file_index as u8,
                    )?;
                    expectation.total_files += 1;
                }
            }
        }
        ScenarioKind::LargeFiles => {
            for dir_index in 0..4 {
                let dir = root.join(format!("segment-{dir_index:02}"));
                fs::create_dir_all(&dir)?;
                expectation.total_dirs += 1;

                for file_index in 0..4 {
                    let size = 1_048_576 + (dir_index * 4 + file_index) * 131_072;
                    expectation.total_size += write_pattern_file(
                        &dir.join(format!("chunk-{file_index:02}.bin")),
                        size,
                        dir_index as u8,
                    )?;
                    expectation.total_files += 1;
                }
            }
        }
        ScenarioKind::MixedTree => {
            for dir_index in 0..12 {
                let section = root.join(format!("section-{dir_index:02}"));
                fs::create_dir_all(&section)?;
                expectation.total_dirs += 1;

                for leaf_index in 0..4 {
                    let leaf = section.join(format!("leaf-{leaf_index:02}"));
                    fs::create_dir_all(&leaf)?;
                    expectation.total_dirs += 1;

                    for file_index in 0..8 {
                        let size = 1_024 + ((dir_index * 4 + leaf_index + file_index) % 9) * 128;
                        expectation.total_size += write_pattern_file(
                            &leaf.join(format!("item-{file_index:02}.txt")),
                            size,
                            (dir_index + leaf_index) as u8,
                        )?;
                        expectation.total_files += 1;
                    }
                }

                let summary_size = 16_384 + dir_index * 1_024;
                expectation.total_size +=
                    write_pattern_file(&section.join("summary.log"), summary_size, dir_index as u8)?;
                expectation.total_files += 1;
            }
        }
    }

    Ok(expectation)
}

pub fn benchmark_sffs_once(
    path: &Path,
    threads: Option<usize>,
    expected: &FixtureExpectation,
) -> io::Result<Duration> {
    let args = Args::benchmark_defaults(path.to_path_buf(), threads);
    let summary = collect_scan_summary(&args);
    validate_summary(&summary, expected)?;
    Ok(summary.duration)
}

pub fn benchmark_du_once(path: &Path) -> io::Result<Duration> {
    let start = std::time::Instant::now();
    let output = Command::new("du").arg("-sk").arg(path).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "du failed with status {:?}",
            output.status.code()
        )));
    }

    Ok(start.elapsed())
}

pub fn generate_reference_artifact(config: BenchmarkConfig) -> io::Result<BenchmarkReferenceArtifact> {
    let mut scenarios = Vec::with_capacity(ScenarioKind::ALL.len());

    for scenario in ScenarioKind::ALL {
        let fixture = prepare_fixture(scenario)?;

        for _ in 0..config.warmup_iterations {
            benchmark_sffs_once(fixture.path(), None, &fixture.expectation)?;
            benchmark_du_once(fixture.path())?;
        }

        let mut sffs_default_runs = Vec::with_capacity(config.measurement_iterations);
        let mut sffs_single_thread_runs = Vec::with_capacity(config.measurement_iterations);
        let mut du_runs = Vec::with_capacity(config.measurement_iterations);

        for _ in 0..config.measurement_iterations {
            sffs_default_runs.push(benchmark_sffs_once(
                fixture.path(),
                None,
                &fixture.expectation,
            )?);
            sffs_single_thread_runs.push(benchmark_sffs_once(
                fixture.path(),
                Some(1),
                &fixture.expectation,
            )?);
            du_runs.push(benchmark_du_once(fixture.path())?);
        }

        let default_median = median_duration(&mut sffs_default_runs);
        let single_thread_median = median_duration(&mut sffs_single_thread_runs);
        let du_median = median_duration(&mut du_runs);

        let default_entries_per_second = throughput(fixture.expectation.total_entries(), default_median);
        let single_thread_entries_per_second =
            throughput(fixture.expectation.total_entries(), single_thread_median);
        let du_entries_per_second = throughput(fixture.expectation.total_entries(), du_median);
        let (best_profile, best_median, best_entries_per_second) = if single_thread_median < default_median {
            ("threads-1", single_thread_median, single_thread_entries_per_second)
        } else {
            ("default", default_median, default_entries_per_second)
        };

        scenarios.push(ScenarioReference {
            slug: fixture.expectation.slug.clone(),
            label: fixture.expectation.label.clone(),
            weight: scenario.weight(),
            total_size: fixture.expectation.total_size,
            total_files: fixture.expectation.total_files,
            total_dirs: fixture.expectation.total_dirs,
            sffs_default_median_ms: duration_to_millis(default_median),
            sffs_single_thread_median_ms: duration_to_millis(single_thread_median),
            sffs_best_profile: best_profile.to_string(),
            sffs_best_median_ms: duration_to_millis(best_median),
            du_median_ms: duration_to_millis(du_median),
            sffs_default_entries_per_second: default_entries_per_second,
            sffs_single_thread_entries_per_second: single_thread_entries_per_second,
            sffs_best_entries_per_second: best_entries_per_second,
            du_entries_per_second,
            sffs_vs_du_multiplier: best_entries_per_second / du_entries_per_second,
        });
    }

    let weighted_entry_values: Vec<_> = scenarios
        .iter()
        .map(|scenario| (scenario.sffs_best_entries_per_second, scenario.weight))
        .collect();
    let weighted_byte_values: Vec<_> = scenarios
        .iter()
        .map(|scenario| {
            (
                throughput(
                    scenario.total_size,
                    Duration::from_secs_f64(scenario.sffs_best_median_ms / 1000.0),
                ),
                scenario.weight,
            )
        })
        .collect();

    let generated_at_epoch_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(BenchmarkReferenceArtifact {
        schema_version: BENCHMARK_SCHEMA_VERSION,
        generated_at_epoch_seconds,
        generation_context: collect_generation_context(),
        reference_label: "repo benchmark reference".to_string(),
        reference_entries_per_second: weighted_geometric_mean(&weighted_entry_values).ok_or_else(
            || io::Error::new(ErrorKind::InvalidData, "failed to compute entry throughput reference"),
        )?,
        reference_bytes_per_second: weighted_geometric_mean(&weighted_byte_values).ok_or_else(
            || io::Error::new(ErrorKind::InvalidData, "failed to compute byte throughput reference"),
        )?,
        scenarios,
    })
}

pub fn artifact_markdown_table(artifact: &BenchmarkReferenceArtifact) -> String {
    let mut table = String::from(
        "| Scenario | sffs default | sffs 1 thread | du | best sffs vs du |\n| --- | ---: | ---: | ---: | ---: |\n",
    );

    for scenario in &artifact.scenarios {
        let best_timing = scenario
            .sffs_default_median_ms
            .min(scenario.sffs_single_thread_median_ms)
            .min(scenario.du_median_ms);

        table.push_str(&format!(
            "| {} | {} | {} | {} | {:.2}x |\n",
            scenario.label,
            format_timing_cell(scenario.sffs_default_median_ms, best_timing),
            format_timing_cell(scenario.sffs_single_thread_median_ms, best_timing),
            format_timing_cell(scenario.du_median_ms, best_timing),
            scenario.sffs_vs_du_multiplier,
        ));
    }

    table
}

fn format_timing_cell(value_ms: f64, best_timing_ms: f64) -> String {
    let formatted = format!("{value_ms:.2} ms");
    if (value_ms - best_timing_ms).abs() < 0.005 {
        format!("**{formatted}**")
    } else {
        formatted
    }
}

pub fn validate_summary(summary: &ScanSummary, expected: &FixtureExpectation) -> io::Result<()> {
    if summary.total_size != expected.total_size
        || summary.total_files != expected.total_files
        || summary.total_dirs != expected.total_dirs
    {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "unexpected summary for {}: got size/files/dirs = {}/{}/{}, expected {}/{}/{}",
                expected.slug,
                summary.total_size,
                summary.total_files,
                summary.total_dirs,
                expected.total_size,
                expected.total_files,
                expected.total_dirs,
            ),
        ));
    }

    Ok(())
}

fn median_duration(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn throughput(units: u64, duration: Duration) -> f64 {
    let seconds = duration.as_secs_f64();
    if units == 0 || seconds <= f64::EPSILON {
        0.0
    } else {
        units as f64 / seconds
    }
}

fn duration_to_millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn collect_generation_context() -> BenchmarkGenerationContext {
    BenchmarkGenerationContext {
        host: BenchmarkHost {
            os: env::consts::OS.to_string(),
            architecture: env::consts::ARCH.to_string(),
            family: env::consts::FAMILY.to_string(),
            available_parallelism: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
        },
        git: BenchmarkGit {
            commit_sha: git_output(&["rev-parse", "HEAD"]),
            short_commit_sha: git_output(&["rev-parse", "--short", "HEAD"]),
            dirty: git_dirty_state(),
        },
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn git_dirty_state() -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(!output.stdout.is_empty())
}

fn write_pattern_file(path: &Path, size: usize, seed: u8) -> io::Result<u64> {
    const CHUNK_SIZE: usize = 8192;

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let mut written = 0usize;
    let mut chunk = [0u8; CHUNK_SIZE];

    for (idx, byte) in chunk.iter_mut().enumerate() {
        *byte = seed.wrapping_add(idx as u8);
    }

    while written < size {
        let remaining = size - written;
        let next = remaining.min(CHUNK_SIZE);
        writer.write_all(&chunk[..next])?;
        written += next;
    }
    writer.flush()?;

    Ok(size as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_generation_matches_expectations() {
        let fixture = prepare_fixture(ScenarioKind::MixedTree).unwrap();
        let args = Args::benchmark_defaults(fixture.path().to_path_buf(), None);
        let summary = collect_scan_summary(&args);

        validate_summary(&summary, &fixture.expectation).unwrap();
    }

    #[test]
    fn markdown_table_contains_scenario_rows() {
        let artifact = BenchmarkReferenceArtifact {
            schema_version: BENCHMARK_SCHEMA_VERSION,
            generated_at_epoch_seconds: 0,
            generation_context: BenchmarkGenerationContext {
                host: BenchmarkHost {
                    os: "macos".to_string(),
                    architecture: "aarch64".to_string(),
                    family: "unix".to_string(),
                    available_parallelism: 8,
                },
                git: BenchmarkGit {
                    commit_sha: Some("deadbeef".to_string()),
                    short_commit_sha: Some("deadbee".to_string()),
                    dirty: Some(false),
                },
            },
            reference_label: "ref".to_string(),
            reference_entries_per_second: 1.0,
            reference_bytes_per_second: 1.0,
            scenarios: vec![ScenarioReference {
                slug: "tiny-files".to_string(),
                label: "Many tiny files".to_string(),
                weight: 1.0,
                total_size: 1,
                total_files: 1,
                total_dirs: 1,
                sffs_default_median_ms: 1.0,
                sffs_single_thread_median_ms: 2.0,
                sffs_best_profile: "default".to_string(),
                sffs_best_median_ms: 1.0,
                du_median_ms: 3.0,
                sffs_default_entries_per_second: 4.0,
                sffs_single_thread_entries_per_second: 5.0,
                sffs_best_entries_per_second: 4.0,
                du_entries_per_second: 6.0,
                sffs_vs_du_multiplier: 0.67,
            }],
        };

        let table = artifact_markdown_table(&artifact);
        assert!(table.contains("Many tiny files"));
        assert!(table.contains("0.67x"));
        assert!(table.contains("**1.00 ms**"));
    }
}