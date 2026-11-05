# Schemra

Schemra is a text-based language and toolkit for structured diagrams and timed
scenarios. Describe a system in `.schemra` files, then validate, format, render,
and animate it with the Rust tools and browser-based Studio.

## Try the Studio

The Studio runs locally in your browser. See [Studio setup and hosting](docs/studio.md)
for prerequisites and the static production build.

```sh
cd apps/studio
bun install --frozen-lockfile
bun run dev
```

Open the local URL printed by Bun (normally `http://127.0.0.1:5173`).

## Try the CLI

With the Rust toolchain installed, validate the example project:

```sh
cargo run --locked -p schemra-cli -- check \
  examples/traffic-overview/visuals/traffic-overview.schemra \
  --root examples/traffic-overview
```

See [Getting started](docs/getting-started.md) for rendering, formatting, and
building the CLI.

## Project guide

- [Documentation index](docs/README.md)
- [Examples and language tour](examples/README.md)
- [Contributing and verification](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
- [MIT License](LICENSE-MIT) / [Apache License 2.0](LICENSE-APACHE)

Schemra is currently at `0.1.0`; language and tool interfaces may change before
`1.0.0`. See the changelog for notable user-facing changes.
