#!/usr/bin/env bash
# Verifies that this repository's source builds to the exact WASM that is live
# on Stellar mainnet: rebuilds every contract, SHA-256s each artifact, and
# compares against the hashes recorded in docs/MAINNET_DEPLOYMENT.md §3 (the
# WASM hash Stellar assigns on upload *is* the SHA-256 of the file).
#
# Usage:
#   scripts/verify_build.sh                  # full: rebuild from source, then compare (needs stellar-cli 26.0.0)
#   scripts/verify_build.sh --fixtures-only  # fast: compare the committed test fixtures in
#                                            #   contracts/account_factory/src/wasm_fixtures/ (no build; six of seven)
#   scripts/verify_build.sh --wasm-dir DIR   # compare an existing build output directory instead of building
#   scripts/verify_build.sh --container      # full, inside a linux/amd64 container (docker or podman) that
#                                            #   mirrors .github/workflows/ci.yml: it compiles stellar-cli 26.0.0
#                                            #   from source with Rust 1.96.0, then builds the contracts with the
#                                            #   pinned Rust 1.94.1. Slow (compiles the CLI and binaryen), and it
#                                            #   needs a native x86_64 host or Rosetta: qemu-user emulation of
#                                            #   amd64 on Apple Silicon crashes rustc. Mirrors the environment that
#                                            #   produced the recorded hashes; see the notes below for what has
#                                            #   actually been demonstrated.
#
# Environment:
#   STELLAR_CLI   path to the stellar binary to build with (default: `stellar` on PATH)
#   CONTAINER_RUNTIME
#                 `docker` or `podman` for --container (default: whichever is on PATH, docker first)
#   CONTAINER_PLATFORM
#                 platform for --container (default: linux/amd64, the CI runner's)
#   CONTAINER_JOBS
#                 cargo build jobs inside the container (default: 2). Compiling
#                 stellar-cli's `stellar-xdr` crate needs several GB per job; a
#                 SIGKILL in cli-install.log means the container VM ran out of memory.
#   RECORD        deployment record to read expected hashes from (default: docs/MAINNET_DEPLOYMENT.md)
#   ALLOW_CLI_MISMATCH=1
#                 build even if the stellar CLI is not the version that produced the
#                 recorded hashes -- the comparison will then fail, see below.
#
# What "reproducible" depends on, measured 2026-09-09 against the mainnet record:
#
# - The stellar CLI version is embedded in every artifact as the `cliver`
#   contract-meta entry (`stellar contract info meta --wasm <file>`). Rust
#   1.94.1 and soroban-sdk 26.1.0 are pinned by rust-toolchain.toml and
#   Cargo.lock; the CLI version is pinned only here and in ci.yml. Building
#   with stellar 28.0.0 reproduced every artifact's *size* but none of its
#   hashes -- the code section was identical, only the meta differed.
# - With the CLI version held at 26.0.0, the bytes still depend on the host
#   the build runs on: on macOS/arm64 the 26.0.0 release binary and a
#   `cargo install`ed 26.0.0 produce identical output to each other, yet
#   differ from the deployed artifacts for 2 of 7 contracts; a linux/arm64
#   container with the 26.0.0 release binary differs for 5 of 7. The
#   deployed bytes came from the ci.yml environment (ubuntu x86_64,
#   `cargo +1.96.0 install --locked stellar-cli --version 26.0.0`), where the
#   fixture staleness check reproduces the six fixtures byte for byte on
#   every push. Every mismatch seen was inside the code section with
#   identical meta -- equivalent-but-not-identical output, not a source
#   difference. Whether it arises in rustc's codegen or in the `--optimize`
#   (wasm-opt) step is recorded in docs/MAINNET_DEPLOYMENT.md §3.1.
#
# So: the plain (host) mode is the fast check and will typically match on the
# CI runner and on hosts set up exactly like it; `--container` is the
# reference environment; `--fixtures-only` needs no build at all.
#
# Exit status: 0 when every compared artifact matches, 1 otherwise.

set -euo pipefail
cd "$(dirname "$0")/.."

RECORD="${RECORD:-docs/MAINNET_DEPLOYMENT.md}"
STELLAR_CLI="${STELLAR_CLI:-stellar}"
EXPECTED_CLI_VERSION="26.0.0"
FIXTURES_DIR="contracts/account_factory/src/wasm_fixtures"

mode="build"
wasm_dir=""
while [ $# -gt 0 ]; do
  case "$1" in
    --fixtures-only) mode="fixtures" ;;
    --container) mode="container" ;;
    --wasm-dir)
      mode="dir"
      wasm_dir="${2:?--wasm-dir needs a directory}"
      shift
      ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
  shift
done

if [ ! -f "$RECORD" ]; then
  echo "deployment record not found: $RECORD" >&2
  exit 2
fi

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# Expected hashes: every "| `<contract>` | `<64 hex>` |" table row in the
# record. Reading the record itself, rather than a separate manifest, means
# there is exactly one place a hash can be wrong.
expected_names=()
expected_hashes=()
while IFS='|' read -r _ name hash _; do
  name="$(echo "$name" | tr -d '` ')"
  hash="$(echo "$hash" | tr -d '` ')"
  expected_names+=("$name")
  expected_hashes+=("$hash")
done < <(grep -E '^\| `[a-z_]+` \| `[0-9a-f]{64}` \|' "$RECORD")

if [ "${#expected_names[@]}" -eq 0 ]; then
  echo "no '| \`contract\` | \`sha256\` |' rows found in $RECORD" >&2
  exit 2
fi

case "$mode" in
  fixtures)
    wasm_dir="$FIXTURES_DIR"
    echo "Comparing committed fixtures in $wasm_dir against $RECORD (no build)."
    ;;
  dir)
    echo "Comparing existing artifacts in $wasm_dir against $RECORD (no build)."
    ;;
  container)
    runtime="${CONTAINER_RUNTIME:-}"
    if [ -z "$runtime" ]; then
      if command -v docker >/dev/null 2>&1; then runtime=docker
      elif command -v podman >/dev/null 2>&1; then runtime=podman
      else
        echo "--container needs docker or podman on PATH (or CONTAINER_RUNTIME=...)" >&2
        exit 2
      fi
    fi
    platform="${CONTAINER_PLATFORM:-linux/amd64}"
    out_root="$(mktemp -d)"
    echo "Building in a $platform rust:1.96.0-slim container via $runtime: compiling stellar-cli $EXPECTED_CLI_VERSION from source, then the contracts with Rust 1.94.1 (this takes a while) ..."
    # Mirrors .github/workflows/ci.yml step for step. The repo is mounted
    # read-only and copied inside the container (minus build outputs), so the
    # host checkout is never touched; only /out is written.
    "$runtime" run --rm --platform "$platform" -v "$PWD:/src:ro" -v "$out_root:/out" \
      -e CARGO_TARGET_DIR=/tmp/target -e CARGO_BUILD_JOBS="${CONTAINER_JOBS:-2}" \
      -e CLI_VERSION="$EXPECTED_CLI_VERSION" rust:1.96.0-slim bash -c '
        set -e
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -qq >/dev/null
        apt-get install -y -qq build-essential cmake pkg-config libdbus-1-dev libudev-dev curl ca-certificates git >/dev/null
        cargo install --locked stellar-cli --version "$CLI_VERSION" >/out/cli-install.log 2>&1
        mkdir /work && (cd /src && tar --exclude=./target --exclude=./.worktrees --exclude=./sdk/node_modules -cf - .) | tar -xf - -C /work
        cd /work
        rustup show active-toolchain >/dev/null 2>&1 || true   # rust-toolchain.toml pulls 1.94.1 + wasm32v1-none
        echo "container: $(uname -srm), $(stellar --version | head -n1), contracts built with $(rustc --version)"
        stellar contract build --optimize --out-dir /out/wasm
      ' >"$out_root/build.log" 2>&1 || {
      echo "container build failed -- see $out_root/build.log (and cli-install.log there)" >&2
      tail -n 20 "$out_root/build.log" >&2
      if grep -qs "SIGKILL" "$out_root/cli-install.log" "$out_root/build.log"; then
        echo "a compiler process was SIGKILLed: the container runtime's VM is out of memory. Give it more" >&2
        echo "memory, or lower CONTAINER_JOBS (currently ${CONTAINER_JOBS:-2})." >&2
      fi
      if grep -qs "qemu: uncaught target signal" "$out_root/cli-install.log" "$out_root/build.log"; then
        echo "linux/amd64 is being emulated with qemu-user, which crashes rustc. Use a native x86_64 host," >&2
        echo "a Rosetta-enabled runtime (podman machine set --rosetta=true), CONTAINER_PLATFORM=linux/arm64" >&2
        echo "(same recipe, but expect only a partial match -- see the notes above), or the CI workflow." >&2
      fi
      exit 2
    }
    grep '^container:' "$out_root/build.log" || true
    wasm_dir="$out_root/wasm"
    ;;
  build)
    if ! command -v "$STELLAR_CLI" >/dev/null 2>&1; then
      echo "stellar CLI not found: $STELLAR_CLI (set STELLAR_CLI=/path/to/stellar)" >&2
      exit 2
    fi
    cli_version="$("$STELLAR_CLI" --version | head -n1 | awk '{print $2}')"
    if [ "$cli_version" != "$EXPECTED_CLI_VERSION" ]; then
      echo "stellar CLI is $cli_version; the recorded hashes were produced with $EXPECTED_CLI_VERSION." >&2
      echo "The CLI stamps its version into every artifact ('cliver' contract meta), so any other" >&2
      echo "version cannot reproduce the recorded hashes. Install it with:" >&2
      echo "  cargo +1.96.0 install --locked stellar-cli --version $EXPECTED_CLI_VERSION" >&2
      echo "and point STELLAR_CLI at it, or set ALLOW_CLI_MISMATCH=1 to build anyway." >&2
      if [ "${ALLOW_CLI_MISMATCH:-0}" != "1" ]; then
        exit 2
      fi
    fi
    wasm_dir="$(mktemp -d)"
    echo "Building with $STELLAR_CLI $cli_version ($(rustc --version)) into $wasm_dir ..."
    "$STELLAR_CLI" contract build --optimize --out-dir "$wasm_dir" >"$wasm_dir/build.log" 2>&1 || {
      echo "build failed -- see $wasm_dir/build.log" >&2
      exit 2
    }
    ;;
esac

printf '\n%-18s %-8s %-64s %s\n' "contract" "bytes" "sha256" "result"
failures=0
compared=0
for i in "${!expected_names[@]}"; do
  name="${expected_names[$i]}"
  expected="${expected_hashes[$i]}"
  file="$wasm_dir/sta_${name}.wasm"
  if [ ! -f "$file" ]; then
    if [ "$mode" = "fixtures" ]; then
      printf '%-18s %-8s %-64s %s\n' "$name" "-" "$expected" "SKIPPED (not a committed fixture)"
    else
      printf '%-18s %-8s %-64s %s\n' "$name" "-" "$expected" "MISSING"
      failures=$((failures + 1))
    fi
    continue
  fi
  actual="$(sha256_of "$file")"
  size="$(wc -c <"$file" | tr -d ' ')"
  compared=$((compared + 1))
  if [ "$actual" = "$expected" ]; then
    printf '%-18s %-8s %-64s %s\n' "$name" "$size" "$actual" "MATCH"
  else
    printf '%-18s %-8s %-64s %s\n' "$name" "$size" "$actual" "MISMATCH (expected $expected)"
    failures=$((failures + 1))
  fi
done

echo
if [ "$failures" -eq 0 ]; then
  echo "OK: $compared artifact(s) match $RECORD."
  exit 0
fi
echo "FAILED: $failures artifact(s) differ from $RECORD." >&2
if [ "$mode" = "build" ] && [ "${cli_version:-}" != "$EXPECTED_CLI_VERSION" ]; then
  echo "(built with stellar $cli_version, not $EXPECTED_CLI_VERSION -- compare 'cliver' via: $STELLAR_CLI contract info meta --wasm <file>)" >&2
fi
exit 1
