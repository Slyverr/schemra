# Schemra

Schemra is a local authoring environment for structured diagrams and their
animated scenarios. This repository contains a minimal Rust CLI and SvelteKit
Studio shell.

## Requirements

- Rust 1.88.0 with Cargo, rustfmt, and Clippy
- Bun 1.4.2
- `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` 0.2.129 for the
  Studio bindings and WASM parity checks

## Run Studio

```sh
cd apps/studio
bun install --frozen-lockfile
bun run dev
```

Studio commands build the browser WASM bindings from `schemra-wasm` before
starting, checking, testing, or building the app. They require the
`wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` 0.2.129. Run
`bun run build:wasm` from `apps/studio` to regenerate the ignored bindings
without starting the app.

## Run the CLI

```sh
cargo run -p schemra-cli -- --help
cargo run -p schemra-cli -- check path/to/visual.schemra --root path/to/project
cargo run -p schemra-cli -- fmt path/to/source.schemra --check
cargo run -p schemra-cli -- scenario-check path/to/flow.schemra --visual path/to/network.schemra
cargo run -p schemra-cli -- render path/to/network.schemra --out network.svg
```

## Verify the browser compiler API

The `schemra-wasm` crate exposes `compile_schemra` for one source document and
`compile_project_schemra` for a linked project request. Editor functions expose
formatting, context-aware completions, and project-wide rename edits. These APIs
reuse the same syntax and compiler services as the native tools. Run the
cross-target response comparison with:

```sh
bash tests/wasm/differential.sh
```

The reusable `ScenarioSampler` compiles a selected project scenario once,
caches its scene geometry, and returns time-indexed playback frames. The Studio
bridge also converts compiler byte offsets to the editor's UTF-16 offsets.

The `check` command validates a visual or scenario source with the other Schemra
files in its project. Add `--json` for machine-readable diagnostics.
The `fmt` command writes formatted source to stdout by default; use `--check`
to verify formatting or `--write` to atomically update a file. Pass `-` to read
from standard input. Use `scenario-check` to verify a scenario against a
specific visual, and `render` to write a validated visual as SVG.

## License

Licensed under either the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
