#!/bin/sh
# Runs the private corpus audit, extracting the corpus first if it is missing.
#
# Usage: tools/corpus.sh
#
# The corpus is $AMIPRO_CORPUS (default /tmp/amipro-phase1-corpus), outside
# Git. If it does not exist, the SAM and STY files are extracted read-only
# from $AMIPRO_ISO (default Reference/lotus-ami-pro-3.1.iso) with
# tools/reference_inventory.py. $AMIPRO_STYLES, if set, is the style-sheet
# path: directories (separated by `:`) searched for each SAM's STY after those
# beside it. The full audit output is printed. The exit
# status is non-zero if any SAM or STY file fails, if a SAM file does not
# round-trip through the native format, if an anchor mark names no frame, or
# if a control leaks into text.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
corpus=${AMIPRO_CORPUS:-/tmp/amipro-phase1-corpus}
iso=${AMIPRO_ISO:-$root/Reference/lotus-ami-pro-3.1.iso}
if [ ! -d "$corpus" ]; then
	if [ ! -f "$iso" ]; then
		echo "corpus $corpus is missing and the reference ISO $iso was not found" >&2
		exit 2
	fi
	python3 -I "$root/tools/reference_inventory.py" "$iso" \
		--extract-extension SAM --extract-extension STY --output-dir "$corpus"
fi
cd "$root"
output=$(cargo run --offline --quiet -p amipro-formats --example corpus_audit -- "$corpus")
printf '%s\n' "$output"
printf '%s\n' "$output" | awk '
	/^SAM files: / { files = $3 }
	/^Failed: / { failed = $2 }
	/^Exact native round trips: / { trips = $5 }
	/^STY failed: / { sty = $3 }
	/^Unresolved anchors: / { unresolved = $3 }
	/^Control leaks: / { leaks = $3 }
	/^(FAIL|NATIVE FAIL) / { bad++ }
	END {
		ok = files > 0 && failed == 0 && trips == files && sty == 0 && bad == 0 && unresolved == 0 && leaks == 0
		printf "corpus gate: %s (%d SAM, %d failed, %d round trips, %d STY failed, %d unresolved anchors, %d control leaks)\n",
			ok ? "PASS" : "FAIL", files, failed, trips, sty, unresolved, leaks
		exit ok ? 0 : 1
	}'
