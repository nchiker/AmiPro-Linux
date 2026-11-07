#!/bin/sh
# Builds dist/amipro-linux-<version>-x86_64.tar.gz from the existing release
# binary (it does not build). Version = 0.<minor from app/cosmic/Cargo.toml's
# 0.<minor>.x>-<git describe --always --dirty>, e.g. 0.1-f0a3294-dirty.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"
bin=app/cosmic/target/release/amipro-cosmic
[ -x "$bin" ] || { echo "missing $bin; build it first (cargo build --release)" >&2; exit 1; }
newer=$(git ls-files app crates Cargo.toml Cargo.lock | grep -v -E '(_tests?\.rs|/tests/|/scenario_tests/)' | while read -r f; do
  [ "$f" -nt "$bin" ] && echo "$f"; done | head -3)
[ -z "$newer" ] || { echo "sources newer than the binary, rebuild first:" >&2; echo "$newer" >&2; exit 1; }
minor=$(sed -n 's/^version *= *"0\.\([0-9]*\)\..*/\1/p' app/cosmic/Cargo.toml | head -1)
commit=$(git rev-parse --short HEAD)
version="0.${minor:-0}-$(git describe --always --dirty)"
name=amipro-linux-$version-x86_64
out=dist/$name
rm -rf "$out"; mkdir -p "$out"
strip -o "$out/amipro-linux" "$bin"
chmod 755 "$out/amipro-linux"
for f in install.sh uninstall.sh amipro-linux.desktop amipro-linux-mime.xml \
         org.amipro_linux.AmiPro.svg README-TESTING.txt; do
  cp packaging/$f "$out/$f"
done
chmod 755 "$out/install.sh" "$out/uninstall.sh"
cat > "$out/VERSION.txt" <<V
AmiPro-Linux $version
Build date: $(date -u -r "$bin" +%Y-%m-%dT%H:%MZ) (binary); packaged $(date -u +%Y-%m-%dT%H:%MZ)
Commit: $commit$(git diff --quiet HEAD -- || echo " (working tree had uncommitted changes)")
Features: default (page-surface), release profile
Target: x86_64 Linux
V
(cd "$out" && sha256sum amipro-linux install.sh uninstall.sh amipro-linux.desktop \
  amipro-linux-mime.xml org.amipro_linux.AmiPro.svg README-TESTING.txt VERSION.txt > CHECKSUMS.sha256)
tar -C dist -czf "dist/$name.tar.gz" "$name"
ls -l "dist/$name.tar.gz"; sha256sum "dist/$name.tar.gz"
