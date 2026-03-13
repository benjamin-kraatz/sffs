# Contributing to sffs

First off, thank you for considering contributing to `sffs`! It's people like you that make the open source community such an amazing place to learn, inspire, and create.

## How Can I Contribute?

### Reporting Bugs

- Check if the bug has already been reported in the Issues.
- If not, open a new issue. Include a clear title and as much relevant information as possible, such as:
    - Steps to reproduce.
    - Expected vs actual behavior.
    - Your OS and Rust version.

### Suggesting Enhancements

- Open a new issue with the tag "enhancement".
- Explain why this feature would be useful.

### Pull Requests

1. Fork the repo and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation.
4. Ensure the test suite passes (`cargo test`).
5. Run `cargo fmt` and `cargo clippy`.
6. Submit a Pull Request!

## Development Setup

To build the project:
```bash
cargo build
```

To run tests:
```bash
cargo test
```

## Style Guide

We use standard Rust formatting. Please run `cargo fmt` before submitting.

## License

By contributing, you agree that your contributions will be licensed under its MIT license.
