# Contributing

This repository treats filesystem access as a chokepoint.

All application, library, test, benchmark, and private helper code must use
`async-fs-io`. Direct `std::fs`, `tokio::fs`, blocking filesystem calls, and
hidden wrappers are forbidden. The only direct filesystem calls allowed in
this repository are inside the audited implementation under `src`.

Run the boundary lint before submitting changes:

```text
./.scripts/lint-fs-io.sh --allow-path src
```

Downstream repositories must run the same script without `--allow-path src`
after copying it into their repository. Add the command to CI and the
pre-commit hook. A private helper is not an exception: the lint must catch it.

Run the complete local validation with:

```text
./.scripts/lint-fs-io.sh --allow-path src
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --doc
```
