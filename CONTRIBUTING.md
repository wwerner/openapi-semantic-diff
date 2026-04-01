# Contributing

Contributions are welcome. Here's how to get started.

## Prerequisites

- **Rust 1.70+** -- install via [rustup](https://rustup.rs/)

## Building

```
cargo build
```

## Running the test suite

```
cargo test
```

Snapshot tests use [insta](https://insta.rs/). If you change output formatting,
update snapshots with:

```
cargo insta review
```

## Code style

This project uses standard `rustfmt` formatting and `clippy` lints:

```
cargo fmt --check
cargo clippy -- -D warnings
```

Please run both before submitting a pull request.

## Submitting changes

1. Fork the repository.
2. Create a feature branch from `main`.
3. Make your changes with clear commit messages.
4. Ensure `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings` all pass.
5. Open a pull request against `main`.

## Reporting issues

Use [GitHub Issues](https://github.com/wwerner/openapi-semantic-diff/issues) to report bugs or request features.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE).
