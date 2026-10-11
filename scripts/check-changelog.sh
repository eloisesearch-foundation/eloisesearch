#!/usr/bin/env bash
# Validate CHANGELOG.md structure (Keep a Changelog).
#
#   - an "## [Unreleased]" section exists;
#   - each version section appears once;
#   - category headings are one of Added/Changed/Deprecated/Removed/Fixed/Security;
#   - no category heading repeats inside a section.
#
# Usage: scripts/check-changelog.sh [path/to/CHANGELOG.md]

set -euo pipefail

file="${1:-CHANGELOG.md}"
[[ -f "$file" ]] || { echo "check-changelog: $file not found" >&2; exit 1; }

awk -v file="$file" '
  function fail(msg) { printf "check-changelog: %s:%d: %s\n", file, NR, msg > "/dev/stderr"; bad = 1 }

  /^## \[/ {
    section = $0
    sub(/^## \[/, "", section); sub(/\].*$/, "", section)
    if (seen_section[section]++) fail("section [" section "] appears more than once")
    if (section == "Unreleased") has_unreleased = 1
    delete seen_cat
    next
  }

  /^### / {
    cat = substr($0, 5)
    if (section == "") fail("category \"" cat "\" outside a version section")
    if (cat !~ /^(Added|Changed|Deprecated|Removed|Fixed|Security)$/)
      fail("unknown category \"" cat "\" (use Added, Changed, Deprecated, Removed, Fixed or Security)")
    if (seen_cat[cat]++) fail("category \"" cat "\" repeated in section [" section "]")
  }

  END {
    if (!has_unreleased) { printf "check-changelog: %s: missing \"## [Unreleased]\" section\n", file > "/dev/stderr"; bad = 1 }
    if (bad) exit 1
    print "check-changelog: " file " ok"
  }
' "$file"
