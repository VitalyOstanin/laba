#!/usr/bin/env bash
# Print the CHANGELOG.md body for one version, for use as GitHub release notes.
# Emits the lines between the `## [X.Y.Z] - DATE` header and the next `## [`
# header (or EOF), with leading and trailing blank lines trimmed. The header
# line itself is omitted (the release already shows the tag as its title).
# Exits non-zero if the section is absent or empty.
#
# Usage: changelog-section.sh X.Y.Z [CHANGELOG.md]
set -euo pipefail

version="${1:?usage: changelog-section.sh X.Y.Z [changelog]}"
file="${2:-CHANGELOG.md}"

test -r "$file" || { echo "changelog-section: cannot read $file" >&2; exit 1; }

# Capture the target section. Matching is literal (index at column 1), so dots
# in the version need no regex escaping. Capturing stops at the next H2 entry.
section="$(
  awk -v target="## [${version}] - " '
    /^## \[/ {
      if (index($0, target) == 1) { cap = 1; next }
      if (cap) exit
      next
    }
    cap { print }
  ' "$file"
)"

# Trim leading and trailing blank lines.
section="$(printf '%s\n' "$section" | awk '
  { lines[NR] = $0 }
  END {
    first = 0; last = 0
    for (i = 1; i <= NR; i++) if (lines[i] ~ /[^[:space:]]/) { if (!first) first = i; last = i }
    if (!first) exit
    for (i = first; i <= last; i++) print lines[i]
  }
')"

test -n "$section" || { echo "changelog-section: no section for ${version} in ${file}" >&2; exit 1; }
printf '%s\n' "$section"
