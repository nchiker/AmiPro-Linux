#!/bin/sh
# Makes one frame fixture in original Ami Pro in the reference VM.
#
# Usage: tools/vm/frame-fixture.sh NAME ["MODIFY-ACTIONS"]
#        tools/vm/frame-fixture.sh --open NAME "MODIFY-ACTIONS"
#        tools/vm/frame-fixture.sh --finish NAME
#
# Creates a new _DEFAULT.STY document with two main paragraphs and one text
# frame (Frame > Create Frame: 2.00 x 1.50 in, 2.00 in down, 1.50 in from the
# left). If MODIFY-ACTIONS is given, opens Modify Frame Layout, sends those
# vmkey.py actions (for example "combo:alt+b key:space" for No wrap beside),
# screenshots the dialog as NAME-dlg, and presses OK. Then types
# "Frame text here." into the frame, saves L:\OUT\NAME.SAM, and closes it.
#
# Dialog mnemonics collide, so --open stops after the NAME-dlg screenshot.
# Check it, send corrections with vmkey.py if needed, then run --finish,
# which presses OK and continues as above.
#
# Requires Ami Pro running with no other document open and the transient
# share mounted as L: (docs/research/VM_LAB.md). Refuses to reuse a name,
# because Save As onto an existing file raises a prompt that swallows the
# following keystrokes.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
lab=${AMIPRO_VM_LAB:-$here/../../../AmiPro-Reference/vm-lab}
mode=all
case "$1" in
--open) mode=open; shift ;;
--finish) mode=finish; shift ;;
esac
name=$1
out="$lab/OUT/$name.SAM"
if [ -e "$out" ]; then
	echo "$out exists; choose a new fixture name" >&2
	exit 1
fi
key() { python3 -I "$here/vmkey.py" "$@"; }

if [ "$mode" != finish ]; then
	key combo:alt+f key:n wait:1.5 key:enter wait:2 \
		text:'Main text paragraph one, long enough to wrap around a frame placed on the page in Ami Pro layout mode.' \
		key:enter text:'Second main paragraph.' \
		combo:alt+r key:c wait:1.5 combo:alt+w text:2.00 combo:alt+h text:1.50 \
		combo:alt+d text:2.00 combo:alt+i text:1.50 key:enter wait:1.5
	if [ -n "${2:-}" ]; then
		key combo:alt+r key:m wait:2
		# shellcheck disable=SC2086 # the actions are separate vmkey.py arguments
		key $2 wait:0.5 shot:"$name-dlg"
		[ "$mode" = open ] && exit 0
		key key:enter wait:1.5
	fi
else
	key key:enter wait:1.5
fi
key key:enter wait:0.8 text:'Frame text here.' wait:0.3 key:esc key:esc wait:0.3 shot:"$name" \
	combo:alt+f key:a wait:1.2 text:"L:\\OUT\\$name.SAM" key:enter wait:3
if [ ! -s "$out" ]; then
	echo "save failed for $name; the document is still open in Ami Pro" >&2
	exit 1
fi
key combo:alt+f key:c wait:1.5
ls -la "$out"
