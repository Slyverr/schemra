# Getting started

## Requirements

- Rust 1.88.0, provided by the repository's `rust-toolchain.toml`
- Cargo and the Rust components listed in that toolchain file

From the repository root, check the included visual project:

```sh
cargo run --locked -p schemra-cli -- check \
  examples/traffic-overview/visuals/traffic-overview.schemra \
  --root examples/traffic-overview
```

Format a source file without changing it:

```sh
cargo run --locked -p schemra-cli -- fmt \
  examples/traffic-overview/visuals/traffic-overview.schemra --check
```

Render the visual to SVG:

```sh
cargo run --locked -p schemra-cli -- render \
  examples/traffic-overview/visuals/traffic-overview.schemra \
  --out /tmp/traffic-overview.svg
```

To build an optimized CLI executable, run:

```sh
cargo build --locked --release -p schemra-cli
```

The executable is written to `target/release/schemra` (or
`target/release/schemra.exe` on Windows). Run `--help` for the command list and
options. Use `scenario-check` to validate a scenario against a visual, or
`lsp` to start the language server over standard input and output.

For the browser app, see [Studio](studio.md). For a guided tour of the DSL, see
[the example project](../examples/README.md).
