# deadcode

Fast cross-language dead-code detector. Single binary, offline.

Detects exported symbols that are never referenced anywhere in the repo.

```bash
deadcode ./src
```

Supports: JS, TS, TSX, JSX, Python, Rust, Go.

## Build

```bash
cargo build --release
```

Binary: `target/release/deadcode`
