# Releases

Schemra uses Semantic Versioning. The workspace and Studio currently identify
the project as `0.1.0`; before `1.0.0`, compatibility of the language and tool
interfaces is not guaranteed between minor versions.

## Changelog

Record notable user-facing changes in `CHANGELOG.md` under `Unreleased`, grouped
as `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, or `Security`. Keep
entries short and describe their effect for users. Internal refactors and
individual commits do not need separate entries.

## Release checklist

1. Confirm CI passes and review the `Unreleased` entries for accuracy.
2. Move those entries under a version heading such as `## [0.2.0] - YYYY-MM-DD`;
   leave a fresh `Unreleased` section at the top.
3. Update the workspace version in `Cargo.toml` and the Studio version in
   `apps/studio/package.json` to the same value. Run `cargo check --workspace`
   to refresh `Cargo.lock`, then commit the updated lockfile.
4. Create an annotated Git tag named `vX.Y.Z` on the release commit and publish
   release notes from the matching changelog section.

The CLI does not currently have a crates.io install path or prebuilt release
binary. Build it from source using
[`docs/getting-started.md`](getting-started.md). The Studio can be distributed
as the static site described in [`docs/studio.md`](studio.md).
