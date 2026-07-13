#!/usr/bin/env bash
# Regenerate THIRD-PARTY-LICENSES.md from the Cargo dependency graph.
#
# Lists every non-workspace crate in the resolved graph with its version and
# SPDX license expression, plus a per-license summary. Uses `cargo metadata`
# (no extra tooling) and `jq`. The list covers the full resolved graph,
# including build- and dev-dependencies, so it is a superset of what ships in a
# release binary — a safe over-approximation for license review.
#
# Usage: scripts/gen-third-party-licenses.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/THIRD-PARTY-LICENSES.md"

# gitea/crates network is not needed; avoid a proxy interfering with cargo.
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY

meta="$(cd "$ROOT" && cargo metadata --format-version 1)"

body="$(printf '%s' "$meta" | jq -r '
  [.packages[] | select(.name | test("^laba-") | not)]
    | sort_by(.name, .version) as $deps
  | ($deps | group_by(.license // "UNSPECIFIED")
      | map({lic: (.[0].license // "UNSPECIFIED"), n: length})
      | sort_by(-.n)) as $summary
  | "## License summary\n\n"
    + "| SPDX expression | Crates |\n| --- | ---: |\n"
    + ($summary | map("| \(.lic) | \(.n) |") | join("\n"))
    + "\n\n## Crates\n\n"
    + "| Crate | Version | License |\n| --- | --- | --- |\n"
    + ($deps | map("| \(.name) | \(.version) | \(.license // "UNSPECIFIED") |") | join("\n"))
')"

count="$(printf '%s' "$meta" | jq -r '[.packages[] | select(.name | test("^laba-") | not)] | length')"

{
  echo "# Third-party licenses"
  echo
  echo "This project bundles Rust crates from the ecosystem. The table below lists"
  echo "every third-party crate in the resolved dependency graph ($count crates) with"
  echo "its version and SPDX license expression."
  echo
  echo "Regenerate with \`scripts/gen-third-party-licenses.sh\`. The dependency"
  echo "license allow-list is enforced separately in CI via \`deny.toml\`"
  echo "(\`cargo deny check licenses\`)."
  echo
  echo "$body"
} > "$OUT"

echo "wrote $OUT ($count crates)"
