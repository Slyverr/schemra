# Schemra

Schemra is a local authoring environment for structured diagrams and their
animated scenarios. This repository contains a minimal Rust CLI and SvelteKit
Studio shell.

## Requirements

- Rust 1.88.0 with Cargo, rustfmt, and Clippy
- Bun 1.4.2

## Run Studio

```sh
cd apps/studio
bun install --frozen-lockfile
bun run dev
```

## Run the CLI

```sh
cargo run -p schemra-cli -- --help
```

## License

Licensed under either the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
