#!/bin/sh
# Removes what install.sh installed. Usage: uninstall.sh [--prefix DIR]
set -eu
APPID=org.amipro_linux.AmiPro
root=${HOME:?}
scratch=0
while [ $# -gt 0 ]; do
  case $1 in
    --prefix) [ $# -ge 2 ] || { echo "--prefix needs a directory" >&2; exit 2; }
              root=$2; scratch=1; shift 2 ;;
    -h|--help) sed -n 2,2p "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
share=$root/.local/share
for f in "$root/.local/bin/amipro-linux" "$share/applications/amipro-linux.desktop" \
         "$share/icons/hicolor/scalable/apps/$APPID.svg" "$share/mime/packages/amipro-linux.xml"; do
  if [ -e "$f" ]; then rm -f "$f"; echo "removed: $f"; fi
done
# Remove directories only if empty and only in a scratch install.
if [ $scratch -eq 1 ]; then
  rm -rf "$share/mime/application" "$share/mime/"*.xml 2>/dev/null || true
  find "$root/.local" -depth -type d -empty -exec rmdir {} \; 2>/dev/null || true
else
  have() { command -v "$1" >/dev/null 2>&1; }
  have update-mime-database && update-mime-database "$share/mime" >/dev/null 2>&1 || true
  have update-desktop-database && update-desktop-database "$share/applications" >/dev/null 2>&1 || true
  have gtk-update-icon-cache && gtk-update-icon-cache -q -t "$share/icons/hicolor" >/dev/null 2>&1 || true
fi
echo "Left alone: your settings in ~/.config/cosmic/$APPID/ and all your documents"
echo "(including the Documents folder and any .SAM.bak files)."
