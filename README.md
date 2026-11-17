# deadcode

Fast, offline, cross-language dead-code detector. Single binary, no runtime, no network.

Scans a directory for exported symbols that are never referenced anywhere else in the repo and reports them with file + line number.

## Why

- knip, ts-prune, vulture are slow, language-specific, and need config.
- deadcode is one binary, zero config, works on JS/TS/TSX/JSX/Python/Rust/Go in the same invocation.
- Runs in CI in milliseconds. No telemetry.

## Install

### From source
```bash
cargo install --git https://github.com/JovianOps/deadcode
```

### From release
Download the binary for your platform from Releases.

## Usage

```bash
deadcode ./src
deadcode ./src --json
deadcode ./src --exclude node_modules --exclude dist
deadcode . --exclude _test
```

### Output
```
DEAD  src/utils.ts:42  legacyHelper
DEAD  src/api.ts:17     deprecatedFetch

2 dead symbol(s) found across 38 files.
```

### JSON mode
```bash
deadcode ./src --json
```

## What counts as dead

An exported symbol is dead when it has zero references outside its own defining line. Test files, node_modules, target, dist, build, vendor, .venv are excluded by default.

## Languages supported

| Language | Export detection |
|---|---|
| JavaScript / TypeScript / JSX / TSX | export function/const/let/var/class/interface/type/enum + export { ... } |
| Python | def / class at module top level |
| Rust | pub fn/struct/enum/trait/const/static/type/mod |
| Go | exported func (capitalized) |

## Limitations

- Heuristic: a symbol referenced only via dynamic import, reflection, or string-based lookup may be falsely flagged. Verify before deleting.
- Does not follow re-exports across files.
- No type-aware analysis. For 100% accuracy, use language-specific tools (knip, vulture, cargo-udeps).

## License

MIT
