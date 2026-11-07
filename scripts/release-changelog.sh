#!/bin/sh
# Usage: release-changelog.sh <version> <date>
#
# Moves the fragments in changelog.d/ into a new CHANGELOG.md section above the
# previous releases. Run from the repository root by `mise run release`.
set -eu

version=$1
date=$2

fragments=$(find changelog.d -name '*.md' | sort)
if [ -z "$fragments" ]; then
  echo "error: no changelog fragments in changelog.d/" >&2
  exit 1
fi

# `awk NF` drops blank lines and ends every line with a newline, so fragments
# written either way join into one list.
SECTION=$(printf '## %s - %s\n\n' "$version" "$date"; awk NF $fragments)
export SECTION

awk '
  !done && /^## / { print ENVIRON["SECTION"]; print ""; done = 1 }
  { print }
  END { if (!done) { print ""; print ENVIRON["SECTION"] } }
' CHANGELOG.md > CHANGELOG.md.new
mv CHANGELOG.md.new CHANGELOG.md
rm $fragments
