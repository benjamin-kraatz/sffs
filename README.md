# sffs (Super Fast File Size)

[![CI](https://github.com/benjamin-kraatz/sffs/actions/workflows/ci.yml/badge.svg)](https://github.com/benjamin-kraatz/sffs/actions/workflows/ci.yml)
[![Commit Lint](https://github.com/benjamin-kraatz/sffs/actions/workflows/commit-lint.yml/badge.svg)](https://github.com/benjamin-kraatz/sffs/actions/workflows/commit-lint.yml)
[![Release](https://github.com/benjamin-kraatz/sffs/actions/workflows/release.yml/badge.svg)](https://github.com/benjamin-kraatz/sffs/actions/workflows/release.yml)
[![Release Please](https://github.com/benjamin-kraatz/sffs/actions/workflows/release-please.yml/badge.svg)](https://github.com/benjamin-kraatz/sffs/actions/workflows/release-please.yml)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)

**sffs** is a blazingly fast, parallel disk usage analyzer for the modern terminal. It is designed to be a faster alternative to `du` with a beautiful, gradient-powered user interface and deep parallelization.

## Features

- **Parallel Scanning**: Leverages multi-core CPUs for rapid directory traversal.
- **Modern UI**: Clean output with color-rich summaries and progress bars.
- **Top Files**: Quickly identify the largest files in any directory.
- **Respectful**: Automatically honors `.gitignore`, `.ignore`, and hidden file rules.
- **Lightweight**: Zero-cost abstractions and efficient memory allocation using `mimalloc`.
- **SI/Binary Units**: Toggle between decimal (KB/MB) and binary (KiB/MiB) unit formats.

## Installation

### From Source

Ensure you have [Rust](https://rustup.rs/) installed:

```bash
cargo install --path .
```

### Pre-built Binaries

Download for your platform from the [GitHub Releases](https://github.com/benn/sffs/releases) page.

## Usage

Basic usage (current directory):

```bash
sffs
```

Check specific paths:

```bash
sffs /path/to/dir ~/Downloads
```

Show top 10 largest files:

```bash
sffs --top 10
```

Ignore hidden files and follow symlinks:

```bash
sffs -H -L
```

### Options

```text
Usage: sffs [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Path(s) to check size for. If omitted, checks the current directory.

Options:
  -L, --follow-links          Follow symbolic links
  -g, --git-ignore           Respect .gitignore files
  -i, --ignore-files         Respect .ignore files
  -H, --ignore-hidden        Ignore hidden files
  -d, --max-depth <MAX_DEPTH>  Maximum depth to recurse
  -t, --threads <THREADS>     Use the provided number of threads
  -b, --bytes                 Show size in raw bytes
      --si                    Use SI units (1000 bytes = 1 KB) instead of 1024
  -x, --one-file-system      Don't cross filesystem boundaries
      --top <N>               Show top N largest files
  -s, --silent                Suppress headers and footer
  -h, --help                  Print help
  -V, --version               Print version
```

## Benchmarks

The repository now ships a benchmark suite that compares `sffs` against the system `du` command across deterministic fixture datasets. The suite covers:

- many tiny files
- a deep directory tree
- a wide directory fan-out
- a few large files
- a mixed realistic tree
- default threading vs `--threads 1`
- correctness checks for size, file count, and directory count on every `sffs` run

Generate a fresh reference artifact:

```bash
cargo run --release --bin benchmark_reference
```

Run the Criterion suite:

```bash
cargo bench --bench cli_benchmarks
```

The current checked-in reference lives in [docs/benchmarks/reference.json](docs/benchmarks/reference.json). It is built from the weighted geometric mean of the fastest `sffs` result per scenario, is used for the always-on speed comparison in the summary output, and records the host platform metadata plus the git commit SHA that produced it.

Reference results from the current checked-in artifact:

| Scenario               | sffs default | sffs 1 thread |      du | best sffs vs du |
| ---------------------- | -----------: | ------------: | ------: | --------------: |
| Many tiny files        |  **5.30 ms** |       6.54 ms | 5.67 ms |           1.07x |
| Deep directory tree    |      2.67 ms |   **1.00 ms** | 2.16 ms |           2.17x |
| Wide directory fan-out |  **5.69 ms** |       6.56 ms | 5.72 ms |           1.00x |
| Few large files        |      1.73 ms |   **0.46 ms** | 2.26 ms |           4.88x |
| Mixed realistic tree   |      3.13 ms |   **2.33 ms** | 3.50 ms |           1.50x |

Interpretation:

- `best sffs vs du` uses the faster of `sffs default` and `sffs --threads 1` for each scenario, then compares that winning `sffs` result against `du`.
- In the current checked-in run, `sffs --threads 1` is the fastest `sffs` configuration on the deep-tree, large-file, and mixed-tree fixtures.
- In the current checked-in run, the best `sffs` result beats `du` on four scenarios and is effectively tied on the wide fan-out fixture.
- The summary line compares the current run against the shipped repository reference, not against a local calibration of your machine or your exact dataset.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. See [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## License

This project is licensed under [MIT license](LICENSE)
