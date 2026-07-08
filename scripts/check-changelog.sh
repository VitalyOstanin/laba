#!/usr/bin/env bash
# Fail unless CHANGELOG.md has a section for the given version.
set -euo pipefail
version="${1:?usage: check-changelog.sh X.Y.Z}"
if ! grep -qE "^## \[${version//./\\.}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md; then
  echo "CHANGELOG.md: no section for ${version}" >&2
  exit 1
fi
echo "CHANGELOG.md: found section for ${version}" >&2
