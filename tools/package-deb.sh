#!/bin/sh
# Builds dist/amipro-linux_<version>_amd64.deb from the existing release binary
# (it does not build; run `cargo build --release -j1` in app/cosmic first).
# Version = $AMIPRO_VERSION, else <Cargo.toml version>+git<short rev>[.dirty]. Needs dpkg-deb; uses
# dpkg-shlibdeps for Depends (falls back to ldd + dpkg -S).
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo"
bin=app/cosmic/target/release/amipro-cosmic
[ -x "$bin" ] || { echo "missing $bin; build it first (cargo build --release -j1)" >&2; exit 1; }
newer=$(git ls-files app crates Cargo.toml Cargo.lock | grep -v -E '(_tests?\.rs|/tests/|/scenario_tests/)' | while read -r f; do
  [ "$f" -nt "$bin" ] && echo "$f"; done | head -3)
[ -z "$newer" ] || { echo "sources newer than the binary, rebuild first:" >&2; echo "$newer" >&2; exit 1; }
base=$(sed -n 's/^version *= *"\([0-9.]*\)".*/\1/p' app/cosmic/Cargo.toml | head -1)
dirty=""; git diff --quiet HEAD -- || dirty=".dirty"
version="$base+git$(git rev-parse --short HEAD)$dirty"
# A release names its own version (tools/release.sh): the updater expects
# amipro-linux_<tag without v>_amd64.deb.
version=${AMIPRO_VERSION:-$version}
work=$(mktemp -d "${TMPDIR:-/tmp}/amipro-deb.XXXXXX")
trap 'rm -rf "$work"' EXIT
root=$work/pkg
install -d -m 755 "$root/DEBIAN" "$root/usr/bin" "$root/usr/share/applications" \
  "$root/usr/share/mime/packages" "$root/usr/share/icons/hicolor/scalable/apps" \
  "$root/usr/share/doc/amipro-linux"
strip -o "$root/usr/bin/amipro-linux" "$bin"; chmod 755 "$root/usr/bin/amipro-linux"
install -m 644 packaging/amipro-linux.desktop "$root/usr/share/applications/amipro-linux.desktop"
install -m 644 packaging/amipro-linux-mime.xml "$root/usr/share/mime/packages/amipro-linux.xml"
install -m 644 packaging/org.amipro_linux.AmiPro.svg "$root/usr/share/icons/hicolor/scalable/apps/org.amipro_linux.AmiPro.svg"
install -m 644 docs/INSTALL.md "$root/usr/share/doc/amipro-linux/INSTALL.md"
cat > "$root/usr/share/doc/amipro-linux/copyright" <<C
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: AmiPro-Linux
Source: https://github.com/nchiker/AmiPro-Linux

Files: *
Copyright: the AmiPro-Linux authors
License: MPL-2.0
 This Source Code Form is subject to the terms of the Mozilla Public
 License, v. 2.0. If a copy of the MPL was not distributed with this
 file, You can obtain one at https://mozilla.org/MPL/2.0/.
 On Debian systems the full text is in /usr/share/common-licenses/MPL-2.0
 and in the project's LICENSE file. The statically linked Rust dependencies
 carry their own licences.
 .
 Ami Pro for Linux is not affiliated with IBM, HCL or Lotus. Lotus and Ami Pro are
 trademarks of their owners.
C
chmod 644 "$root/usr/share/doc/amipro-linux/copyright"
# Depends
deps=""
if command -v dpkg-shlibdeps >/dev/null 2>&1; then
  mkdir -p "$work/debian"
  printf 'Source: amipro-linux\n\nPackage: amipro-linux\nArchitecture: amd64\n' > "$work/debian/control"
  deps=$(cd "$work" && dpkg-shlibdeps -O --ignore-missing-info -e"$root/usr/bin/amipro-linux" 2>/dev/null | sed -n 's/^shlibs:Depends=//p')
fi
if [ -z "$deps" ]; then
  deps=$(ldd "$root/usr/bin/amipro-linux" | awk '/=>/ {print $3}' | while read -r l; do
    dpkg -S "$(readlink -f "$l")" 2>/dev/null | head -1 | cut -d: -f1; done | sort -u | paste -sd, - | sed 's/,/, /g')
fi
# Libraries the toolkit opens with dlopen (invisible to ldd/shlibdeps).
deps="$deps, libwayland-client0, libegl1, libvulkan1, libx11-6, libx11-xcb1"
size=$(du -sk "$root" | cut -f1)
cat > "$root/DEBIAN/control" <<C
Package: amipro-linux
Version: $version
Section: editors
Priority: optional
Architecture: amd64
Installed-Size: $size
Depends: $deps
Recommends: xdg-desktop-portal, xdg-desktop-portal-cosmic | xdg-desktop-portal-gtk
Maintainer: Charlie Day <nchiker35@duck.com>
Homepage: https://github.com/nchiker/AmiPro-Linux
Description: Word processor compatible with Lotus Ami Pro documents
 Page- and frame-based word processor for the COSMIC desktop that opens and
 saves Ami Pro SAM and STY files and exchanges DOCX, ODT, RTF, Markdown and
 PDF. Pre-release build; see /usr/share/doc/amipro-linux/INSTALL.md.
C
for s in postinst postrm; do
  cat > "$root/DEBIAN/$s" <<'S'
#!/bin/sh
set -e
case "$1" in configure|remove|purge|upgrade|abort-upgrade|abort-install|abort-remove|abort-configure|disappear)
  if command -v update-mime-database >/dev/null 2>&1; then update-mime-database /usr/share/mime >/dev/null 2>&1 || true; fi
  if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database -q /usr/share/applications >/dev/null 2>&1 || true; fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor >/dev/null 2>&1 || true; fi
  ;;
esac
exit 0
S
  chmod 755 "$root/DEBIAN/$s"
done
mkdir -p dist
out=dist/amipro-linux_${version}_amd64.deb
dpkg-deb --root-owner-group --build "$root" "$out" >/dev/null
ls -l "$out"; sha256sum "$out"
