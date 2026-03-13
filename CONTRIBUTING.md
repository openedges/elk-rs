# Contributing to elk-rs

Thank you for your interest in contributing to elk-rs!

## How to Contribute

1. **Fork** the repository and create your branch from `main`.
2. **Make changes** and ensure they pass all quality gates:
   ```bash
   cargo build --workspace
   cargo clippy --workspace --all-targets
   cargo test --workspace
   ```
3. **Submit a pull request** with a clear description of your changes.

## Reporting Issues

- Use [GitHub Issues](https://github.com/openedges/elk-rs/issues) to report bugs or request features.
- Include a minimal reproduction case when reporting bugs.

## Code Style

- Follow standard Rust conventions (`rustfmt`, `clippy`).
- Keep commit messages concise: `<scope>: <summary>`.

## License

By contributing, you agree that your contributions will be licensed under the [Eclipse Public License 2.0](LICENSE).
