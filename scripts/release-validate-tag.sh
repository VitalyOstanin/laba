#!/usr/bin/env bash
# Validate tag format vX.Y.Z[-pre][+build] and echo the bare version.
set -euo pipefail
tag="${1:?usage: release-validate-tag.sh vX.Y.Z}"
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)*$ ]]; then
  echo "invalid tag: $tag" >&2
  exit 1
fi
echo "${tag#v}"
