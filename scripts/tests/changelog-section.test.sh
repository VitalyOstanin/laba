#!/usr/bin/env bash
# Regression tests for scripts/changelog-section.sh. Self-contained: just bash.
# Exits non-zero on the first failure.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
script="$here/../changelog-section.sh"

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

# A sample CHANGELOG with three released sections and an Unreleased header.
d="$(mktemp -d)"
cl="$d/CHANGELOG.md"
cat >"$cl" <<'EOF'
# Changelog

## [Unreleased]

### Added

- work in progress

## [0.2.0] - 2026-07-18

### Added

- header update indicator
- setting to turn update checks off

### Fixed

- hide the indicator at once

## [0.1.0] - 2026-01-01

### Added

- first release
EOF

# 1. Extracts the exact body of the requested section, trimmed, without header.
out="$("$script" 0.2.0 "$cl")"
expected="### Added

- header update indicator
- setting to turn update checks off

### Fixed

- hide the indicator at once"
[[ "$out" == "$expected" ]] || fail "0.2.0 section body mismatch; got:
$out"

# 2. The last section stops at EOF (no trailing blank lines).
out="$("$script" 0.1.0 "$cl")"
[[ "$out" == "### Added

- first release" ]] || fail "0.1.0 section body mismatch; got:
$out"

# 3. A version with no section fails (non-zero) and prints nothing to stdout.
rc=0
out="$("$script" 9.9.9 "$cl" 2>/dev/null)" || rc=$?
[[ "$rc" -ne 0 ]] || fail "missing section should exit non-zero"
[[ -z "$out" ]] || fail "missing section should print nothing to stdout"

# 4. An unreadable changelog fails cleanly.
rc=0
"$script" 0.2.0 "$d/nope.md" >/dev/null 2>&1 || rc=$?
[[ "$rc" -ne 0 ]] || fail "unreadable changelog should exit non-zero"

rm -rf "$d"
echo "all changelog-section tests passed"
