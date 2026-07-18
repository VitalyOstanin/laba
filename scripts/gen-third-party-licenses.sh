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

# npm production dependencies. The distributable is a Tauri app whose bundled
# binary embeds the built frontend (gui/), so the runtime npm packages ship too
# and their licenses need attribution alongside the Rust crates. Reads the
# resolved production tree from the installed node_modules (no network, no extra
# tooling beyond npm+jq). Skipped with a note if node_modules is absent.
npm_section=""
if command -v npm >/dev/null 2>&1 && [ -d "$ROOT/gui/node_modules" ]; then
  npm_rows="$(cd "$ROOT/gui" && npm ls --omit=dev --all --parseable 2>/dev/null | tail -n +2 \
    | while IFS= read -r p; do
        [ -f "$p/package.json" ] || continue
        jq -r '
          def lic:
            if (.license | type) == "string" then .license
            elif (.license | type) == "object" then (.license.type // "UNSPECIFIED")
            elif (.licenses | type) == "array" then ([.licenses[].type] | join(" OR "))
            else "UNSPECIFIED" end;
          "| \(.name) | \(.version) | \(lic) |"' "$p/package.json" 2>/dev/null
      done | sort -u)"
  npm_count="$(printf '%s\n' "$npm_rows" | grep -c '^| ')"
  npm_section="$(printf '## npm production dependencies\n\nRuntime npm packages bundled into the Tauri frontend (%s packages). The\ndev-only toolchain (build, lint, test) is excluded — it is not distributed.\n\n| Package | Version | License |\n| --- | --- | --- |\n%s' "$npm_count" "$npm_rows")"
else
  npm_section="## npm production dependencies

Not generated: \`gui/node_modules\` is absent (run \`npm ci\` in \`gui/\` first)."
fi

{
  echo "# Third-party licenses"
  echo
  echo "This project bundles third-party code from two ecosystems: Rust crates in"
  echo "the workspace binaries, and the npm packages of the Tauri frontend that ship"
  echo "inside the desktop bundle. The tables below list every third-party Rust crate"
  echo "in the resolved dependency graph ($count crates) and every production npm"
  echo "package, each with its version and SPDX license expression."
  echo
  echo "Regenerate with \`scripts/gen-third-party-licenses.sh\`. The dependency"
  echo "license allow-list is enforced separately in CI via \`deny.toml\`"
  echo "(\`cargo deny check licenses\`)."
  echo
  echo "$body"
  echo
  echo "$npm_section"
} > "$OUT"

echo "wrote $OUT ($count crates, ${npm_count:-0} npm packages)"
