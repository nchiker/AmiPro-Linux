#!/bin/sh
# Crops and enlarges a reference-VM screenshot with ffmpeg.
# Usage: tools/vm/crop.sh NAME OUT WIDTH HEIGHT X Y [SCALE]
# Reads and writes SCREENS/NAME.png and SCREENS/OUT.png, where SCREENS is
# $AMIPRO_VM_LAB/screens (default ../AmiPro-Reference/vm-lab/screens).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
screens=${AMIPRO_VM_LAB:-$here/../../../AmiPro-Reference/vm-lab}/screens
scale=${7:-2}
ffmpeg -loglevel error -y -i "$screens/$1.png" \
	-vf "crop=$3:$4:$5:$6,scale=iw*$scale:ih*$scale:flags=neighbor" "$screens/$2.png"
