#!/bin/sh
# Prepares a release of AmiPro-Linux; it publishes nothing.
#
#   tools/release.sh 0.2.0 [NOTES_FILE]
#
# Builds dist/amipro-linux_0.2.0_amd64.deb with tools/package-deb.sh (from the
# existing release binary: build it first, `cargo build --release -j1` in
# app/cosmic), writes dist/SHA256SUMS, and prints the `gh release create`
# command to run yourself. The updater (Help > Check for Updates...) expects
# exactly these two assets, a tag `vX.Y.Z`, and a package named
# amipro-linux_X.Y.Z_amd64.deb. See docs/RELEASING.md.
set -eu
[ $# -ge 1 ] || { echo "usage: tools/release.sh VERSION [NOTES_FILE]" >&2; exit 2; }
version=${1#v}
notes=${2:-}
case $version in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "VERSION must look like 1.2.3 (or 1.2.3-rc.1), got: $1" >&2; exit 2 ;;
esac
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"
# The program compares its own Cargo.toml version with the release's tag, so
# they must agree or every install would offer the same update again.
crate=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' app/cosmic/Cargo.toml | head -1)
[ "$crate" = "$version" ] || {
  echo "app/cosmic/Cargo.toml says version $crate, not $version: set it, rebuild, commit, then release." >&2
  exit 1
}
git diff --quiet HEAD -- || { echo "the working tree has uncommitted changes; commit first." >&2; exit 1; }
[ -z "$notes" ] || [ -f "$notes" ] || { echo "no notes file: $notes" >&2; exit 1; }
AMIPRO_VERSION=$version tools/package-deb.sh
deb=dist/amipro-linux_${version}_amd64.deb
[ -f "$deb" ] || { echo "package-deb.sh did not write $deb" >&2; exit 1; }
(cd dist && sha256sum "amipro-linux_${version}_amd64.deb" > SHA256SUMS && sha256sum -c SHA256SUMS)
echo
echo "Ready (nothing published). To publish, review the files and run:"
if [ -n "$notes" ]; then
  echo "  gh release create v$version $deb dist/SHA256SUMS --title \"AmiPro-Linux $version\" --notes-file $notes"
else
  echo "  gh release create v$version $deb dist/SHA256SUMS --title \"AmiPro-Linux $version\" --notes-file NOTES.md"
fi
