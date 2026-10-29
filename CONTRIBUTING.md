# Contributing

Keep each change focused, buildable, and covered by the most relevant checks.
Do not commit generated build output, local project data, or credentials.

## Commit messages

The root commit uses the exact subject `Initial commit`. The local hook permits
that subject only while the repository has no commit; every later commit must
use the conventional format below.

Use a conventional type and an imperative, concise subject. Prefer:

```text
type(target/scope): concise imperative message
```

Use `type(target): message` or `type: message` when a narrower scope does not
apply. Valid types include `feat`, `fix`, `refactor`, `perf`, `test`, `docs`,
`build`, `ci`, and `chore`. Targets must name a real crate or component; scopes
should identify the affected subsystem. Use `type: message` for repository-wide
changes; do not use `repo` as a target or scope.

Examples:

```text
feat(schemra-compiler/registry): add construct registration
fix(schemra-engine/routing): correct port selection
chore(schemra-engine/deps): update routing dependencies
docs: document project setup
```

When a subject does not explain the motivation or consequences, add a short body
covering why the change was made, key decisions, and compatibility effects. Keep
trivial commits brief.

Install the repository's local commit-message hook with:

```sh
git config core.hooksPath .githooks
```

## Verification

### Test layout

Keep tests separate from production source. Add directories only when a suite
needs them; do not create empty test scaffolding.

| Scope | Location | Approach |
| --- | --- | --- |
| Rust crate contract | `crates/<crate>/tests/` | Integration tests through the crate's public API, like an external consumer |
| Rust private invariant | `crates/<crate>/src/tests.rs` | Only when private access is necessary; reference it with `#[cfg(test)] mod tests;` |
| Studio TypeScript logic | `apps/studio/tests/` | Existing Bun test runner |
| Studio components | `apps/studio/component-tests/` | Vitest and Svelte Testing Library when component behavior needs it |
| Studio browser workflows | `apps/studio/e2e/` | Playwright in a real browser |
| Shared system behavior | root `tests/{integration,parity,rendering}/` | Cross-crate, native/WASM, rendering, and regression coverage |

Do not put test implementations under `apps/studio/src/`, and do not embed
large `#[cfg(test)] mod tests { ... }` blocks in Rust implementation files. If
private Rust behavior needs unit access, put it in a separate test module file
and keep the declaration in the source file minimal. Do not expose private APIs
or add dependencies solely to make testing easier.

Name Rust integration files by subsystem and behavior, such as
`tests/compiler_repeat_expansion.rs`; name Rust tests and Bun/Vitest tests for
the observable condition and expected result. Use `*.test.js` or `*.test.ts`
for Bun/Vitest and `*.spec.ts` for Playwright. Keep tests focused on behavior,
public contracts, edge cases, and determinism. Prefer explicit assertions over
large snapshots; avoid excess mocking and tests coupled to private structure.

Keep fixtures near the suite that owns them: crate-specific fixtures under
`crates/<crate>/tests/fixtures/`, Studio fixtures under the corresponding
`apps/studio` test directory, and shared fixtures under `tests/fixtures/`.
Golden fixtures are reviewed source files: preserve them during migration, do
not regenerate them silently, and inspect expected-output changes. Compare
migrated behavior with the read-only `schemra-next` baseline where equivalent
behavior exists; claim parity only when the comparisons have passed.

### Test commands

Run the full existing check before committing:

```sh
./check.sh
```

The focused commands are:

```sh
cargo test --workspace
cargo fmt --all -- --check
cd apps/studio
bun run check
bun run test
bun run build
```

When component or browser suites are introduced, add their runner dependencies
and package scripts in the same change, then run `bun run test:components` or
`bun run test:e2e` as applicable. Playwright coverage must exercise the real
browser workflow; a successful type check or build is not a substitute.

### Clean-tree commit verification

Every commit must be supported by checks run against an isolated clean copy of
the exact staged tree, with no untracked, ignored, or local-state files needed
for success. One way to create that candidate tree is:

```sh
candidate_tree="$(git write-tree)"
tmpdir="$(mktemp -d)"
trap 'rm -rf -- "$tmpdir"' EXIT
git archive "$candidate_tree" | tar -x -C "$tmpdir"
(cd "$tmpdir" && ./check.sh)
```

Commit only the tree that was checked. If the index or tracked files change
after validation, repeat the checks. For each migration slice, include the
relevant behavior and fixture comparisons in the verification record; never
claim parity without passing evidence.
