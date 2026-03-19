#!/usr/bin/env bash
# Simulates the CI pipeline locally. Exits non-zero on the first failure.
# Usage: ./scripts/ci.sh [--no-fmt] [--no-clippy] [--no-test] [--no-doc]

set -euo pipefail

SKIP_FMT=0
SKIP_CLIPPY=0
SKIP_TEST=0
SKIP_DOC=0

for arg in "$@"; do
  case "$arg" in
    --no-fmt)    SKIP_FMT=1 ;;
    --no-clippy) SKIP_CLIPPY=1 ;;
    --no-test)   SKIP_TEST=1 ;;
    --no-doc)    SKIP_DOC=1 ;;
    *) echo "Unknown option: $arg" >&2; exit 1 ;;
  esac
done

BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
RESET='\033[0m'

step() { echo -e "\n${BOLD}${CYAN}==> $*${RESET}"; }
ok()   { echo -e "${GREEN}✓ $*${RESET}"; }
fail() { echo -e "${RED}✗ $*${RESET}" >&2; exit 1; }

cd "$(dirname "$0")/.."

# ── 1. Format check ──────────────────────────────────────────────────────────
if [[ $SKIP_FMT -eq 0 ]]; then
  step "cargo fmt --check"
  if ! cargo fmt --all -- --check; then
    fail "Formatting issues found. Run: cargo fmt --all"
  fi
  ok "fmt"
fi

# ── 2. Clippy (deny warnings) ─────────────────────────────────────────────────
if [[ $SKIP_CLIPPY -eq 0 ]]; then
  step "cargo clippy -- -W clippy::all -D warnings"
  if ! cargo clippy --all-targets -- -W clippy::all -D warnings 2>&1; then
    fail "Clippy reported warnings/errors."
  fi
  ok "clippy"
fi

# ── 3. Build ──────────────────────────────────────────────────────────────────
step "cargo build --all"
if ! cargo build --all; then
  fail "Build failed."
fi
ok "build"

# ── 4. Tests ──────────────────────────────────────────────────────────────────
if [[ $SKIP_TEST -eq 0 ]]; then
  step "cargo test --all"
  if ! cargo test --all; then
    fail "Tests failed."
  fi
  ok "tests"
fi

# ── 5. Doc check ─────────────────────────────────────────────────────────────
if [[ $SKIP_DOC -eq 0 ]]; then
  step "cargo doc --no-deps"
  if ! RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all 2>&1; then
    fail "Doc generation failed."
  fi
  ok "docs"
fi

echo -e "\n${BOLD}${GREEN}All CI checks passed.${RESET}"
