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

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/) to automate versioning and changelog generation. This means all commit messages must follow a specific format:

```text
<type>(optional scope): <description>

[optional body]

[optional footer(s)]
```

Common types include:
- `feat`: A new feature (triggers a **minor** release)
- `fix`: A bug fix (triggers a **patch** release)
- `docs`: Documentation-only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools

**Breaking Changes**: Must include a `!` after the type/scope or `BREAKING CHANGE:` in the footer (triggers a **major** release).

Example: `feat(api): add support for multiple paths`

**Validation**: Your commit messages will be checked automatically by GitHub Actions on every Pull Request. Non-compliant messages will prevent merging.

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
