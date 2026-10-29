#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
studio_temp="$repo_root/.tmp/bun-check-$$"

cleanup() {
	rm -rf -- "$studio_temp"
	rmdir -- "$repo_root/.tmp" 2>/dev/null || true
}
trap cleanup EXIT

cd "$repo_root"
cargo fmt --all -- --check
cargo test --workspace --locked
cli_help="$(cargo run --locked -q -p schemra-cli -- --help)"
[[ "$cli_help" == *"Usage: schemra"* ]]

cd "$repo_root/apps/studio"
mkdir -p "$studio_temp"
export BUN_TMPDIR="$studio_temp"
bun_install_args=(--frozen-lockfile --prefer-offline)
if [[ -n "${BUN_CACHE_DIR:-}" ]]; then
	bun_install_args+=(--cache-dir "$BUN_CACHE_DIR")
fi
bun install "${bun_install_args[@]}"
bun run check
bun run test
bun run build
