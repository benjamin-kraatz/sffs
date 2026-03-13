# sffs (Super Fast File Size)

[![CI](https://github.com/benn/sffs/actions/workflows/ci.yml/badge.svg)](https://github.com/benn/sffs/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)
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

*Benchmarking against `du` and `dua-cli`* (Work in progress)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. See [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## License

This project is licensed under [MIT license](LICENSE)
