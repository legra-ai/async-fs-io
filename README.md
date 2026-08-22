# async-fs-io

[![Crates.io](https://img.shields.io/crates/v/async-fs-io.svg)](https://crates.io/crates/async-fs-io)
[![Documentation](https://docs.rs/async-fs-io/badge.svg)](https://docs.rs/async-fs-io)
[![CI](https://github.com/legra-ai/async-fs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/legra-ai/async-fs-io/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)

Async-first filesystem primitives for Tokio applications that must keep file
I/O centralized, bounded, and observable.

The crate provides:

- `AsyncFile`, an async low-level file handle for sequential and random-access
  operations;
- bounded whole-file reads that require an explicit byte ceiling;
- streaming directory traversal with one directory entry in flight;
- atomic writes that stream through a temporary file;
- explicit asynchronous temporary-directory cleanup;
- typed filesystem errors with path and operation context.

Large files and directories are never implicitly loaded into memory. Use
`AsyncFile` or the streaming helpers for unbounded data and use bounded reads
only when the caller has an explicit size contract.

## Enforcing the filesystem boundary

The crate is designed to be the single filesystem boundary for an application.
Do not call `std::fs`, `tokio::fs`, blocking filesystem APIs, or private wrappers
around them anywhere else in the repository.

Run the repository lint with:

```text
./.scripts/lint-fs-io.sh --allow-path src
```

Copy that command into CI and the repository's pre-commit hook. In a consuming
repository, omit `--allow-path src`; the consumer has no filesystem backend
allowlist. The lint scans production code, tests, benchmarks, and private
helpers. Its only allowlist in this repository is the audited backend
implementation under `src`.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE));
- MIT License ([LICENSE-MIT](LICENSE-MIT)).
