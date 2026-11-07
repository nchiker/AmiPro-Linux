#!/bin/sh
# Runs the project's verification gate and prints one line per step.
#
# Usage: tools/verify.sh [--quick] [--release] [--corpus]
#   (default)  fmt, strict Clippy, and tests for the core workspace and for
#              the COSMIC app: default (the page surface), the text editor
#              (`--no-default-features`), and all features.
#   --quick    the core workspace only.
#   --release  also `--locked --release` builds of the app (default and all
#              features).
#   --corpus   also the private corpus audit (tools/corpus.sh).
#
# Full logs are in target/verify/. The exit status is non-zero if any step
# fails. The summary counts are summed from every `test result:` line.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root" || exit 2
logs=target/verify
mkdir -p "$logs"
quick=0
release=0
corpus=0
for option; do
	case $option in
	--quick) quick=1 ;;
	--release) release=1 ;;
	--corpus) corpus=1 ;;
	*)
		echo "unknown option: $option" >&2
		exit 2
		;;
	esac
done
app=app/cosmic/Cargo.toml
failed=0

step() {
	name=$1
	shift
	log=$logs/$name.log
	if "$@" >"$log" 2>&1; then
		status=PASS
	else
		status=FAIL
		failed=1
	fi
	counts=$(awk '/^test result:/ { p += $4; f += $6; i += $8; n++ }
		END { if (n) printf "%d passed, %d failed, %d ignored", p, f, i }' "$log")
	printf '%s %-16s %s\n' "$status" "$name" "${counts:-}"
}

step core-fmt cargo fmt --all -- --check
step core-clippy cargo clippy --offline --workspace --all-targets -- -D warnings
step core-test cargo test --offline --workspace
if [ "$quick" = 0 ]; then
	step app-fmt cargo fmt --manifest-path "$app" -- --check
	step app-clippy cargo clippy --offline --manifest-path "$app" --all-targets -- -D warnings
	step app-clippy-text cargo clippy --offline --manifest-path "$app" --no-default-features --all-targets -- -D warnings
	step app-clippy-all cargo clippy --offline --manifest-path "$app" --all-features --all-targets -- -D warnings
	step app-test cargo test --offline --manifest-path "$app"
	step app-test-text cargo test --offline --no-default-features --manifest-path "$app"
	step app-test-all cargo test --offline --all-features --manifest-path "$app"
fi
if [ "$release" = 1 ]; then
	step app-release cargo build --offline --locked --release --manifest-path "$app"
	step app-release-all cargo build --offline --locked --release --all-features --manifest-path "$app"
fi
if [ "$corpus" = 1 ]; then
	step corpus-audit sh tools/corpus.sh
fi
echo "logs: $root/$logs"
exit "$failed"
