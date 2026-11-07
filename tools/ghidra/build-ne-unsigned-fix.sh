#!/bin/sh
# Builds a Ghidra extension holding a copy of Ghidra's NE loader whose
# fixed-bundle entry segment number is read unsigned.
#
# Ghidra 12.1.3's NeLoader.processEntryTable indexes the segment table with a
# signed byte, so NE files with more than 127 segments fail to import
# ("Index -49 out of bounds for length 209" for AMIPRO.EXE, segment 208).
# The loader source is extracted from the installed Flatpak Ghidra and patched
# locally; no Ghidra source is stored in this repository.
#
# Usage: tools/ghidra/build-ne-unsigned-fix.sh SETTINGS_DIR
# Then import with:
#   flatpak run --filesystem=SETTINGS_DIR --filesystem=PROJECT_DIR \
#     --env=JAVA_TOOL_OPTIONS=-Dapplication.settingsdir=SETTINGS_DIR \
#     --command=/app/lib/ghidra/support/analyzeHeadless org.ghidra_sre.Ghidra \
#     PROJECT_DIR PROJECT_NAME -import AMIPRO.EXE -loader NeUnsignedLoader
set -eu

settings_dir=${1:?usage: build-ne-unsigned-fix.sh SETTINGS_DIR}
mkdir -p "$settings_dir"
settings_dir=$(cd "$settings_dir" && pwd)
app=org.ghidra_sre.Ghidra
ghidra=/app/lib/ghidra

in_ghidra() {
	flatpak run --filesystem="$settings_dir" --command=sh "$app" -c "$1"
}

properties=$(in_ghidra "cat $ghidra/Ghidra/application.properties")
version=$(printf '%s\n' "$properties" | sed -n 's/^application.version=//p')
release=$(printf '%s\n' "$properties" | sed -n 's/^application.release.name=//p')
# Ghidra prefixes the user name when the settings directory is outside $HOME.
case "$settings_dir/" in
"$HOME"/*) app_dir=ghidra ;;
*) app_dir="$(id -un)-ghidra" ;;
esac
extension="$settings_dir/$app_dir/ghidra_${version}_${release}/Extensions/NeUnsignedFix"

build="$settings_dir/.ne-unsigned-fix-build"
rm -rf "$build"
mkdir -p "$build/src" "$build/classes" "$extension/lib"

in_ghidra "cd '$build/src' && unzip -q -o $ghidra/Ghidra/Features/Base/lib/Base-src.zip \
	ghidra/app/util/opinion/NeLoader.java"
source="$build/src/ghidra/app/util/opinion/NeLoader.java"
patched="$build/src/ghidra/app/util/opinion/NeUnsignedLoader.java"
python3 -I - "$source" "$patched" <<'EOF'
import re
import sys

text = open(sys.argv[1]).read()
replacements = [
    (
        "seg = st.getSegments()[bundle.getType() - 1].getSegmentID();",
        "seg = st.getSegments()[Byte.toUnsignedInt(bundle.getType()) - 1].getSegmentID();",
    ),
    # Keep the processor-opinion lookup on the original loader name.
    (
        "QueryOpinionService.query(getName(),",
        "QueryOpinionService.query(NE_NAME,",
    ),
    (
        "\t\treturn NE_NAME;",
        '\t\treturn NE_NAME + " - unsigned entry segment fix";',
    ),
]
for old, new in replacements:
    if text.count(old) != 1:
        sys.exit(f"NeLoader.java changed upstream; expected one match for: {old}")
    text = text.replace(old, new)
open(sys.argv[2], "w").write(re.sub(r"\bNeLoader\b", "NeUnsignedLoader", text))
EOF
rm "$source"

in_ghidra "classpath=\$(find $ghidra/Ghidra -path '*/lib/*.jar' | tr '\n' ':') &&
	/app/jdk/bin/javac --release 21 -nowarn -cp \"\$classpath\" -d '$build/classes' '$patched' &&
	cd '$build/classes' && /app/jdk/bin/jar cf '$extension/lib/NeUnsignedFix.jar' ghidra"
cat >"$extension/extension.properties" <<EOF
name=NeUnsignedFix
description=NE loader with unsigned fixed-bundle entry segment numbers
author=AmiPro-Linux tools
createdOn=$(date +%Y-%m-%d)
version=$version
EOF
: >"$extension/Module.manifest"
rm -rf "$build"
echo "Installed $extension"
