#!/bin/sh
# Interleaved release A/B benchmark of both editors: a base revision (A)
# against the working tree (B).
#
# Usage: tools/bench-ab.sh BASE_REV [ROUNDS]
#
# Runs the ignored release baselines default_editor_typing_performance_baseline
# (default build) and rich_editor_typing_performance_baseline
# (rich-editor-prototype). Both revisions must contain tests with these names.
# Rounds alternate A-then-B and B-then-A to cancel drift on this host (the
# CPU governor is powersave, and unrelated emulators often run). ROUNDS
# defaults to 3.
#
# The fixture is $AMIPRO_PERF_SAM (default Reference/SAM/README31.SAM).
# BASE_REV is checked out into a temporary worktree, which is removed on exit.
# Its release builds are kept in target/bench-ab/base so that later runs
# against the same base are incremental. Logs and summary.txt go to
# target/bench-ab/<timestamp>/.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
base=${1:?usage: tools/bench-ab.sh BASE_REV [ROUNDS]}
rounds=${2:-3}
AMIPRO_PERF_SAM=${AMIPRO_PERF_SAM:-$root/Reference/SAM/README31.SAM}
export AMIPRO_PERF_SAM
if [ ! -f "$AMIPRO_PERF_SAM" ]; then
	echo "fixture not found: $AMIPRO_PERF_SAM" >&2
	exit 2
fi
out=$root/target/bench-ab/$(date +%Y%m%d-%H%M%S)
mkdir -p "$out"
work=$(mktemp -d "${TMPDIR:-/tmp}/amipro-ab.XXXXXX")
git -C "$root" worktree add --quiet --detach "$work/base" "$base"
trap 'git -C "$root" worktree remove --force "$work/base"; rm -rf "$work"' EXIT

# build SOURCE_DIR TARGET_DIR LABEL [CARGO_FLAGS...]: prints the test binary.
build() {
	source_dir=$1
	target_dir=$2
	label=$3
	shift 3
	CARGO_TARGET_DIR=$target_dir cargo test --offline --release --no-run \
		--message-format=json --manifest-path "$source_dir/app/cosmic/Cargo.toml" "$@" \
		2>"$out/build-$label.log" |
		python3 -I -c '
import json, sys
paths = [m["executable"] for m in map(json.loads, sys.stdin)
         if m.get("reason") == "compiler-artifact" and m.get("executable")
         and m["profile"]["test"]]
print(paths[-1] if paths else "")'
}

echo "building (logs in $out)"
a_default=$(build "$work/base" "$root/target/bench-ab/base" A-default)
a_rich=$(build "$work/base" "$root/target/bench-ab/base" A-rich --features rich-editor-prototype)
b_default=$(build "$root" "$root/app/cosmic/target" B-default)
b_rich=$(build "$root" "$root/app/cosmic/target" B-rich --features rich-editor-prototype)
for binary in "$a_default" "$a_rich" "$b_default" "$b_rich"; do
	if [ ! -x "$binary" ]; then
		echo "a build failed; see $out/build-*.log" >&2
		exit 1
	fi
done

run() {
	label=$1
	binary=$2
	test_name=$3
	"$binary" "$test_name" --ignored --nocapture >"$out/$label.log" 2>&1
	{
		echo "== $label"
		grep -E 'median|paragraphs|\(line ' "$out/$label.log" || true
	} >>"$out/summary.txt"
}

round=1
while [ "$round" -le "$rounds" ]; do
	echo "round $round of $rounds"
	if [ $((round % 2)) = 1 ]; then order="A B"; else order="B A"; fi
	for side in $order; do
		if [ "$side" = A ]; then
			run "A-default-$round" "$a_default" default_editor_typing_performance_baseline
			run "A-rich-$round" "$a_rich" rich_editor_typing_performance_baseline
		else
			run "B-default-$round" "$b_default" default_editor_typing_performance_baseline
			run "B-rich-$round" "$b_rich" rich_editor_typing_performance_baseline
		fi
	done
	round=$((round + 1))
done
cat "$out/summary.txt"
echo "summary: $out/summary.txt"
