#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$repo_root/tests/wasm/fixtures/compile-contract.schemra"
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
cargo build --quiet -p schemra-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs \
	--out-dir "$temporary_root/generated" \
	"$repo_root/target/wasm32-unknown-unknown/release/schemra_wasm.wasm"
bun "$repo_root/tests/wasm/compare-native-and-wasm.mjs" \
	"$fixture" \
	"$temporary_root/native.json" \
	"$temporary_root/generated/schemra_wasm.js"
