# sffs User Guide

This guide provides detailed information on how to use `sffs` effectively.

## Installation

### From Source

Ensure you have Rust installed (1.80+ for `let_chains` support):
```bash
cargo install --path .
```

### Pre-built Binaries

*(Coming soon)*

## Basic Usage

Run `sffs` without any arguments to scan the current directory:
```bash
sffs
```

Specify one or more paths:
```bash
sffs /path/to/dir ~/Downloads
```

## Advanced Options

### Display Units

By default, `sffs` uses binary units (1024-based, KiB, MiB, etc.).
Use `--si` for decimal units (1000-based, KB, MB, etc.):
```bash
sffs --si
```

Use `--bytes` to show raw byte counts:
```bash
sffs --bytes
```

### Filtering and Respecting Ignore Files

`sffs` is designed to play well with standard ignore files.

- `-g, --git-ignore`: Respect `.gitignore` rules.
- `-i, --ignore-files`: Respect `.ignore` rules.
- `-H, --ignore-hidden`: Ignore files starting with a dot `.`.

Example:
```bash
sffs -g -H
```

### Symbolic Links

By default, `sffs` does not follow symbolic links. Use `-L, --follow-links` to enable it:
```bash
sffs -L
```

### Filesystem Boundaries

Use `-x, --one-file-system` to avoid crossing into different filesystems (mount points):
```bash
sffs -x
```

### Finding the Largest Files

Use `--top <N>` to show the top N largest files encountered during the scan:
```bash
sffs --top 10
```

### Controlling Parallelism

`sffs` automatically chooses the number of threads based on your CPU.
Use `-t, --threads <N>` to override this:
```bash
sffs --threads 4
```

### Scripting and Silent Mode

If you're using `sffs` in a script and just want the result, use `-s, --silent`:
```bash
sffs --silent
```

Output format for silent mode:
```text
Total Size: 1.23 GB
```

## Tips for Speed

- `sffs` is fastest when scanning local SSDs. Network drives may experience latency.
- Large numbers of small files are where `sffs` shines due to its parallel nature.
- `mimalloc` is used by default for better memory performance.
