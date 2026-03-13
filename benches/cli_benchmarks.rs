use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sffs::benchmark::{benchmark_du_once, benchmark_sffs_once, prepare_fixture, ScenarioKind};
use std::time::Duration;

fn cli_benchmarks(c: &mut Criterion) {
    let mut fixtures = Vec::new();
    let mut group = c.benchmark_group("cli_processes");

    for scenario in ScenarioKind::ALL {
        let fixture = prepare_fixture(scenario).expect("fixture should build");
        let throughput = Throughput::Elements(fixture.expectation.total_entries());
        let path = fixture.path().to_path_buf();
        let expectation = fixture.expectation.clone();
        group.throughput(throughput);

        group.bench_function(BenchmarkId::new("sffs/default", scenario.slug()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += benchmark_sffs_once(&path, None, &expectation)
                        .expect("sffs default benchmark run should succeed");
                }
                total
            })
        });

        let path = fixture.path().to_path_buf();
        let expectation = fixture.expectation.clone();
        group.bench_function(BenchmarkId::new("sffs/threads-1", scenario.slug()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += benchmark_sffs_once(&path, Some(1), &expectation)
                        .expect("sffs single-thread benchmark run should succeed");
                }
                total
            })
        });

        let path = fixture.path().to_path_buf();
        group.bench_function(BenchmarkId::new("du", scenario.slug()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += benchmark_du_once(&path).expect("du benchmark run should succeed");
                }
                total
            })
        });

        fixtures.push(fixture);
    }

    group.finish();
    drop(fixtures);
}

criterion_group!(benches, cli_benchmarks);
criterion_main!(benches);
