#!/bin/sh
# Per-user install of AmiPro-Linux. Usage: install.sh [--prefix DIR]
# Without --prefix the files go under $HOME (.local/...). With it, under DIR/
# using the same layout (a scratch install; no databases are updated then).
set -eu
APPID=org.amipro_linux.AmiPro
here=$(cd "$(dirname "$0")" && pwd)
root=${HOME:?}
scratch=0
while [ $# -gt 0 ]; do
  case $1 in
    --prefix) [ $# -ge 2 ] || { echo "--prefix needs a directory" >&2; exit 2; }
              root=$2; scratch=1; shift 2 ;;
    -h|--help) sed -n 2,4p "$0"; exit 0 ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
  esac
done
bin=$root/.local/bin
apps=$root/.local/share/applications
icons=$root/.local/share/icons/hicolor/scalable/apps
mime=$root/.local/share/mime
mkdir -p "$bin" "$apps" "$icons" "$mime/packages"
cp -f "$here/amipro-linux" "$bin/amipro-linux.new" && chmod 755 "$bin/amipro-linux.new" \
  && mv -f "$bin/amipro-linux.new" "$bin/amipro-linux"
echo "binary:        $bin/amipro-linux"
sed "s|^Exec=amipro-linux|Exec=$bin/amipro-linux|" "$here/amipro-linux.desktop" > "$apps/amipro-linux.desktop"
echo "desktop entry: $apps/amipro-linux.desktop"
cp -f "$here/$APPID.svg" "$icons/$APPID.svg"
echo "icon:          $icons/$APPID.svg"
cp -f "$here/amipro-linux-mime.xml" "$mime/packages/amipro-linux.xml"
echo "mime types:    $mime/packages/amipro-linux.xml (.sam, .sty)"
if [ $scratch -eq 0 ]; then
  have() { command -v "$1" >/dev/null 2>&1; }
  have update-mime-database && update-mime-database "$mime" >/dev/null 2>&1 && echo "updated:       mime database" || true
  have update-desktop-database && update-desktop-database "$apps" >/dev/null 2>&1 && echo "updated:       desktop database" || true
  have gtk-update-icon-cache && gtk-update-icon-cache -q -t "$root/.local/share/icons/hicolor" >/dev/null 2>&1 && echo "updated:       icon cache" || true
  case ":$PATH:" in *":$bin:"*) ;; *) echo "note: $bin is not on your PATH; use the application menu or the full path." ;; esac
else
  echo "(scratch install: databases not updated)"
fi
echo "Done. Not set as the default handler (optional: see README-TESTING.txt)."
