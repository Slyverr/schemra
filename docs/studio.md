# Schemra Studio

Schemra Studio is a browser app for editing local Schemra projects, viewing
diagrams, and playing scenarios. It has no hosted backend: project data stays
in IndexedDB in the browser profile where it was created.

## Run locally

Requirements:

- Rust 1.88.0 and the `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` 0.2.129
- Bun 1.4.2

Install the WebAssembly target and matching binding tool with:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked --version 0.2.129
```

From the repository root:

```sh
cd apps/studio
bun install --frozen-lockfile
bun run dev
```

Open the local URL printed by Bun. The Studio builds its WebAssembly compiler
bindings before starting the development server.

## Build and host the browser app

Build a static site:

```sh
cd apps/studio
bun run build
```

The generated site is in `apps/studio/build/`. To inspect it locally, serve that
directory over HTTP:

```sh
python3 -m http.server 4173 --bind 127.0.0.1 --directory build
```

Then open `http://127.0.0.1:4173`. The generated files can be hosted by any
static web host; no Node or Bun server is needed after the build. Serve a public
deployment over HTTPS. Each browser profile keeps its own workspace data, so
hosting the app does not create project sync or shared accounts.

For a site hosted under a repository path, such as GitHub Pages at
`https://<owner>.github.io/<repository>/`, set `BASE_PATH` to that path when
building. For example, from `apps/studio`:

```sh
BASE_PATH=/schemra bun run build
```

Leave `BASE_PATH` unset for a domain-root deployment.

## Build and browser checks

From `apps/studio`, `bun run check`, `bun run test`, `bun run build`, and
`bun run test:e2e` run the type check, unit tests, production build, and browser
workflows. The root [check script](../check.sh) runs the Rust and Studio checks.
