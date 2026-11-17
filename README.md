# deadcode

**Fast, offline, cross-language dead-code detector.** One binary. Zero config. 100+ languages.

Scans a directory and reports every exported/public symbol that is never referenced anywhere else in the repo — with file, line, and name.

```
DEAD  src/utils.ts:42  legacyHelper
DEAD  src/api.ts:17    deprecatedFetch

2 dead symbol(s) found across 38 files.
```

## Why this exists

Existing tools are fragmented and slow:

- `ts-prune` / `knip` — TypeScript only, needs config, Node runtime.
- `vulture` — Python only, needs Python installed.
- `cargo-udeps` — Rust only, needs nightly.
- IDE "unused" hints — one file at a time.

`deadcode` is a single static binary that covers all of them in one pass and runs in CI in milliseconds. No telemetry, no network.

## Install

### From source
```bash
cargo install --git https://github.com/JovianOps/deadcode
```

### Prebuilt binary
Download from [Releases](https://github.com/JovianOps/deadcode/releases).

## Usage

```bash
deadcode ./src                      # scan a directory
deadcode .                          # scan current dir
deadcode ./src --json               # JSON output for CI
deadcode ./src --exclude node_modules --exclude dist
deadcode . -h                       # help
```

### Flags

| Flag | Description |
|---|---|
| `PATH` | Directory to scan (default `.`) |
| `--json` | Emit JSON lines instead of text |
| `--exclude PATTERN` | Skip any path containing `PATTERN` (repeatable) |
| `-h`, `--help` | Show help |

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Scan completed (dead symbols may or may not exist) |
| `1` | Invalid arguments |
| `2` | Path not found |

Use `--json` in CI and filter with `jq`.

## JSON output

```bash
deadcode ./src --json
```
```json
{"dead":1,"files":38}
{"file":"src/api.ts","line":17,"symbol":"deprecatedFetch"}
```

## What counts as dead

A **named declaration** (`function`, `def`, `fn`, `func`, `sub`, `proc`, `class`, `struct`, `enum`, `trait`, `interface`, `type`, `record`, `module`, `namespace`, `package`) is flagged when it has **zero references outside its own declaration line**.

Optional visibility keywords are understood, so `pub fn x`, `export const x`, `public void x` and plain `def x` all work.

### Automatically excluded

- `node_modules`, `target`, `dist`, `build`, `vendor`, `deps`, `.venv`, `venv`, `__pycache__`, `.git`
- Test files: `*.test.ts`, `*.test.js`, `*.spec.*`, `*_test.go`, `*_test.py`, `*_test.rs`

## Languages supported (111 extensions)

**Systems / compiled**
`c` `h` `cpp` `cc` `cxx` `hpp` `hxx` `rs` `go` `zig` `odin` `nim` `c3` `d` `v` `pas` `pp` `asm` `s` `m` `mm` `swift` `swiftui` `kt` `kts` `java` `scala` `groovy` `cs` `vb` `fs` `fsi` `fsx` `jl`

**Web / scripting**
`js` `mjs` `cjs` `ts` `tsx` `jsx` `py` `pyw` `rb` `php` `pl` `pm` `lua` `tcl` `awk` `sed` `sh` `bash` `zsh` `fish` `ps1` `bat` `cmd` `r` `R` `cr` `raku` `rakumod` `ex` `exs` `erl` `hrl` `hs` `lhs` `ml` `mli` `elm` `purs` `pure` `res` `resi` `gleam` `clj` `cljs` `cljc` `dart` `vbs`

**Frameworks / UI**
`vue` `svelte` `astro` `mdx` `elm`

**Data / config**
`sql` `graphql` `gql` `proto` `thrift` `toml` `yaml` `yml` `json` `md` `rst` `adoc` `tex` `sty` `cls` `bib` `tf` `tfvars` `hcl` `pkl` `tofu`

## CI example

```yaml
- name: Dead code check
  run: |
    cargo install --git https://github.com/JovianOps/deadcode
    deadcode ./src
```

Fail the build if any dead code exists:

```bash
deadcode ./src --json | grep -q '"dead":0' || { echo "dead code found"; exit 1; }
```

## Limitations

- **Heuristic, not semantic.** Symbol detection is regex-based on the declaration line. A symbol used only via dynamic import, reflection, or string lookup can be falsely flagged.
- **Re-exports** (`export * from`) are not resolved across files.
- **Shadowed names** in different files share one global count — a rare name in file A may be "used" by coincidence in file B.
- For byte-exact accuracy on a single language, use its native tool (knip, vulture, cargo-udeps).

Verify every finding before deleting code.

## Development

```bash
cargo build --release
cargo test --release
./target/release/deadcode ./src
```

## License

MIT
