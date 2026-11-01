#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$repo_root/tests/wasm/fixtures/compile-contract.schemra"
cross_target_fixture="$repo_root/tests/wasm/fixtures/cross-target.schemra"
project_fixture="$repo_root/tests/wasm/fixtures/project-contract.json"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/schemra-wasm-parity.XXXXXX")"
trap 'rm -rf -- "$temporary_root"' EXIT

if ! command -v bun >/dev/null 2>&1 || ! command -v wasm-bindgen >/dev/null 2>&1; then
	echo 'This check requires Bun and wasm-bindgen-cli 0.2.129.' >&2
	exit 127
fi
if [[ "$(wasm-bindgen --version)" != 'wasm-bindgen 0.2.129' ]]; then
	echo 'wasm-bindgen-cli must be version 0.2.129 to match the Rust binding.' >&2
	exit 1
fi
if ! rustup target list --installed | grep -Fxq wasm32-unknown-unknown; then
	echo 'Install the Rust target with: rustup target add wasm32-unknown-unknown' >&2
	exit 1
fi

mkdir -p "$temporary_root/generated"
cd "$repo_root"
cargo run --quiet -p schemra-wasm --example native_compile < "$fixture" > "$temporary_root/native.json"
cargo run --quiet -p schemra-wasm --example native_project_compile < "$project_fixture" > "$temporary_root/native-project.json"
cargo build --quiet -p schemra-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs \
	--out-dir "$temporary_root/generated" \
	"$repo_root/target/wasm32-unknown-unknown/release/schemra_wasm.wasm"
bun "$repo_root/tests/wasm/compare-native-and-wasm.mjs" \
	"$fixture" \
	"$temporary_root/native.json" \
	"$temporary_root/generated/schemra_wasm.js"
cargo run --quiet -p schemra-wasm --example native_compile -- --compile-only \
	< "$cross_target_fixture" > "$temporary_root/cross-target-native.json"
bun "$repo_root/tests/wasm/compare-native-and-wasm-compile.mjs" \
	"$cross_target_fixture" \
	"$temporary_root/cross-target-native.json" \
	"$temporary_root/generated/schemra_wasm.js"
bun "$repo_root/tests/wasm/compare-native-and-wasm-project.mjs" \
	"$project_fixture" \
	"$temporary_root/native-project.json" \
	"$temporary_root/generated/schemra_wasm.js"
