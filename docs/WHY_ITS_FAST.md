# Why It's Fast

This document explains the performance decisions behind `sffs`, the benchmark infrastructure used to measure them, and the tradeoffs that shaped the current defaults.

## Overview

`sffs` is fast for two reasons:

1. It keeps the scan loop simple and cheap.
2. It measures real CLI behavior instead of assuming that more threads always helps.

The recent performance work added a full benchmark pipeline, a shipped benchmark reference artifact, and a series of runtime changes driven by measured results instead of intuition.

## Benchmark-Driven Development

The benchmark suite lives in the codebase and compares `sffs` against the system `du` command over deterministic fixture datasets.

The current benchmark scenarios cover:

- many tiny files
- a deep directory tree
- a wide directory fan-out
- a few large files
- a mixed realistic tree

For each scenario, the benchmark runner measures:

- default `sffs`
- `sffs --threads 1`
- `du`

It then records:

- per-scenario median times
- throughput in entries per second
- throughput in bytes per second
- the fastest `sffs` profile for that scenario
- the `best sffs vs du` multiplier
- host platform metadata and git revision provenance

This data is written to `docs/benchmarks/reference.json` and also used to produce the benchmark table in the README.

## The Main Performance Findings

The benchmark work surfaced one important fact early: thread coordination cost is real, and on some trees it is larger than the useful work.

That means the slowest path was not “Rust is too slow” or “filesystem traversal is too slow.” In several cases, the cost came from the overhead of setting up and coordinating parallel work for inputs that were too small or too irregular to benefit from it.

The measured patterns were:

- Deep trees often favored a low-overhead single-threaded scan.
- Large-file scenarios heavily favored the low-overhead path because there were few entries to distribute.
- Wide trees and many-tiny-file trees could benefit from broader parallelism, but not if the heuristic itself became expensive.
- A costly heuristic can erase the gain it was trying to unlock.

That last point was especially important. An early attempt at a deeper workload estimator improved decision quality but added too much overhead to the default path, so it was removed.

## Current Performance Decisions

### 1. Cheap Default Traversal Decision

The current code avoids expensive pre-scans. Instead of walking the tree twice or building a complex predictor, it uses a lightweight default path and keeps explicit `--threads` available for users who want to force parallel execution.

This is a pragmatic tradeoff:

- better worst-case latency on small and irregular trees
- no duplicate I/O just to decide how to walk
- predictable user override behavior with `--threads`

The guiding rule is simple: if a heuristic is expensive enough to show up in the benchmark, it is too expensive for the default path.

### 2. Hot-Path Directory Fast Path

Inside the walker, directory entries now take a cheaper path when the file type already tells us an entry is a directory.

That matters because a full metadata fetch is more expensive than a cached or cheaper file-type check, and directory entries are frequent in real trees. Avoiding unnecessary metadata work reduces overhead on every scan, not just benchmark fixtures.

### 3. Shared Scan Summary and Performance Model

The scan logic, speed calculations, and benchmark reference handling were separated into reusable internal modules.

This matters for performance work because it makes the measured quantities consistent across:

- the CLI summary output
- the benchmark generator
- the checked-in benchmark artifact

Without that, it would be easy to benchmark one definition of “speed” and display another.

### 4. Best-Case `sffs` vs `du` Comparison

The benchmark table does not blindly compare `du` against only one `sffs` mode. Instead, the `best sffs vs du` multiplier uses the faster of:

- default `sffs`
- `sffs --threads 1`

for each scenario.

That keeps the benchmark interpretation honest. It separates:

- how the default product behaves
- what the fastest currently implemented `sffs` mode can do

## Why Not Just Parallelize Everything?

Because on real hardware, more concurrency is not free.

Parallel traversal introduces costs such as:

- worker startup and scheduling overhead
- shared-state coordination
- queueing and work distribution costs
- extra cache pressure
- worse behavior on small or unbalanced directory trees

If the workload is large and broad enough, those costs can be amortized. If it is not, they dominate runtime.

The benchmark data made that visible, so the implementation moved away from “parallel by default at all costs” and toward “low overhead first, parallel when it actually earns its keep.”

## Why the Benchmarks Matter

It is easy to make a scanner look faster by changing the dataset, omitting correctness checks, or measuring only internal functions. The current benchmark setup tries not to do that.

The suite is intentionally process-level:

- it measures the actual CLI binary
- it includes the real startup and argument-parsing path
- it validates reported size, file count, and directory count
- it compares against a real baseline tool, `du`

That makes the results noisier than a microbenchmark, but much more representative of real usage.

## Current Tradeoffs

The current implementation favors low overhead and predictable behavior over maximum theoretical parallelism.

That means:

- some scenarios still favor `du`
- `sffs` can be significantly faster on other shapes, especially when the chosen mode matches the tree shape well
- there is still room to optimize the hot path further without reintroducing expensive decision logic

The next likely performance levers are:

- reducing path allocation churn for `--top`
- reducing heap work in top-file tracking
- tightening metadata access further in the walker
- revisiting parallel defaults only if a cheaper and better predictor emerges

## Files Involved

- `src/walker.rs`: traversal strategy and hot-path entry processing
- `src/scan.rs`: shared scan summary used by the CLI and benchmark code
- `src/perf.rs`: benchmark artifact structures and reference-speed math
- `src/benchmark.rs`: benchmark fixtures, artifact generation, and markdown table generation
- `docs/benchmarks/reference.json`: checked-in benchmark artifact with provenance

## Practical Takeaway

`sffs` is fast today not because it assumes parallelism is always good, but because it measures real workloads and keeps the default path cheap.

The important engineering choice was not “add more threads.” It was “stop paying for work that does not produce speed.”