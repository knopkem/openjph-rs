#!/usr/bin/env bash
# Publishes openjph-core and openjph-cli to crates.io in dependency order.
# Usage: ./scripts/publish.sh [OPTIONS]
#
# Options:
#   --dry-run          Run cargo publish --dry-run (no actual upload)
#   --core-only        Publish openjph-core only
#   --cli-only         Publish openjph-cli only (assumes core is already published)
#   --no-verify        Pass --no-verify to cargo publish (skip local build check)
#   --version VERSION  Override the version in workspace Cargo.toml before publishing
#   --wait SECS        Seconds to wait for crates.io to index core before publishing cli
#                      (default: 20; ignored with --dry-run or --core-only)

set -euo pipefail

DRY_RUN=0
CORE_ONLY=0
CLI_ONLY=0
NO_VERIFY=0
VERSION=""
WAIT_SECS=20

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)    DRY_RUN=1 ;;
    --core-only)  CORE_ONLY=1 ;;
    --cli-only)   CLI_ONLY=1 ;;
    --no-verify)  NO_VERIFY=1 ;;
    --version)    VERSION="$2"; shift ;;
    --wait)       WAIT_SECS="$2"; shift ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
  shift
done

if [[ $CORE_ONLY -eq 1 && $CLI_ONLY -eq 1 ]]; then
  echo "Error: --core-only and --cli-only are mutually exclusive." >&2
  exit 1
fi

BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RESET='\033[0m'

step()  { echo -e "\n${BOLD}${CYAN}==> $*${RESET}"; }
ok()    { echo -e "${GREEN}✓ $*${RESET}"; }
warn()  { echo -e "${YELLOW}! $*${RESET}"; }
fail()  { echo -e "${RED}✗ $*${RESET}" >&2; exit 1; }

cd "$(dirname "$0")/.."

# ── Helpers ───────────────────────────────────────────────────────────────────

publish_args() {
  local args=()
  [[ $DRY_RUN   -eq 1 ]] && args+=("--dry-run")
  [[ $NO_VERIFY -eq 1 ]] && args+=("--no-verify")
  echo "${args[@]+"${args[@]}"}"
}

# ── Version bump (optional) ───────────────────────────────────────────────────

if [[ -n "$VERSION" ]]; then
  step "Updating workspace version to $VERSION"

  TOML="Cargo.toml"
  if ! grep -q "^version = " "$TOML"; then
    fail "Could not find 'version = ...' line in $TOML"
  fi

  # Use sed to replace the version line inside [workspace.package]
  sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" "$TOML"
  rm -f "${TOML}.bak"
  ok "Version set to $VERSION in $TOML"
fi

# ── Pre-flight: confirm git is clean (warn only) ──────────────────────────────

if git diff --quiet && git diff --cached --quiet; then
  ok "Working tree is clean"
else
  warn "Working tree has uncommitted changes — proceeding anyway"
fi

# ── Determine current version ─────────────────────────────────────────────────

CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
echo -e "\n  ${BOLD}Crates to publish:${RESET}"
if [[ $CLI_ONLY -eq 0 ]]; then
  echo "    openjph-core  v${CURRENT_VERSION}"
fi
if [[ $CORE_ONLY -eq 0 ]]; then
  echo "    openjph-cli   v${CURRENT_VERSION}"
fi

if [[ $DRY_RUN -eq 1 ]]; then
  warn "DRY RUN — nothing will be uploaded to crates.io"
fi

echo ""
read -r -p "Continue? [y/N] " confirm
[[ "$confirm" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 0; }

# ── Publish openjph-core ──────────────────────────────────────────────────────

if [[ $CLI_ONLY -eq 0 ]]; then
  step "Publishing openjph-core"
  # shellcheck disable=SC2046
  cargo publish -p openjph-core $(publish_args)
  ok "openjph-core published"
fi

# ── Wait for crates.io to index core before cli tries to resolve it ───────────

if [[ $CORE_ONLY -eq 0 && $CLI_ONLY -eq 0 && $DRY_RUN -eq 0 ]]; then
  step "Waiting ${WAIT_SECS}s for crates.io to index openjph-core..."
  sleep "$WAIT_SECS"
  ok "Wait complete"
fi

# ── Publish openjph-cli ───────────────────────────────────────────────────────

if [[ $CORE_ONLY -eq 0 ]]; then
  step "Publishing openjph-cli"
  # shellcheck disable=SC2046
  cargo publish -p openjph-cli $(publish_args)
  ok "openjph-cli published"
fi

# ── Done ──────────────────────────────────────────────────────────────────────

echo -e "\n${BOLD}${GREEN}All crates published successfully.${RESET}"
if [[ -n "$VERSION" ]]; then
  echo -e "${YELLOW}Reminder: commit the version bump and tag the release:${RESET}"
  echo "  git add Cargo.toml Cargo.lock"
  echo "  git commit -m \"chore: release v${CURRENT_VERSION}\""
  echo "  git tag v${CURRENT_VERSION}"
  echo "  git push && git push --tags"
fi
