#!/bin/sh
# Runs only the tests a change can affect (git diff vs HEAD, plus untracked).
#
# Usage: tools/test-changed.sh [--dry-run] [--text] [BASE_REV]
#   --dry-run  print the cargo commands, run nothing.
#   --text     also run the text-editor build when the full app run is chosen.
#   BASE_REV   compare against this revision instead of HEAD.
#
# Rules:
#   crates/document            -> whole core workspace (everything depends on it)
#   crates/<other>             -> that crate only (-p), plus the app's matching
#                                 scenarios are NOT run; use the app run if the
#                                 app consumes the change
#   app/cosmic/src/<m>.rs      -> app tests filtered by `<m>::` (and the family
#                                 prefix, e.g. table_*.rs -> `table_`)
#   app/cosmic/src/scenario_tests/<m>.rs -> `scenarios::<m>` only
#   shared app files (main.rs, command.rs, chrome.rs, page_surface*.rs,
#   page_layout.rs, Cargo.*, build.rs, help/)  -> full app run
#   docs, tools, *.md                      -> nothing
# AMIPRO_STYLES is unset (the style-sheet tests count sheets); --offline always.
set -u
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root" || exit 2
dry=0 text=0 base=HEAD
for a; do
	case $a in
	--dry-run) dry=1 ;;
	--text) text=1 ;;
	-*) echo "unknown option: $a" >&2; exit 2 ;;
	*) base=$a ;;
	esac
done
unset AMIPRO_STYLES
app=app/cosmic/Cargo.toml
files=$( { git diff --name-only "$base" --; git ls-files --others --exclude-standard; } | sort -u)

core_all=0 core_pkgs="" app_full=0 app_filters="" scen=""
add() { case " $2 " in *" $1 "*) echo "$2" ;; *) echo "$2 $1" ;; esac; }
for f in $files; do
	case $f in
	crates/document/*) core_all=1 ;;
	Cargo.toml|Cargo.lock) core_all=1 ;;
	crates/*/*)
		d=${f#crates/}; d=${d%%/*}
		pkg=$(sed -n 's/^name *= *"\(.*\)"/\1/p' "crates/$d/Cargo.toml" | head -1)
		core_pkgs=$(add "$pkg" "$core_pkgs") ;;
	app/cosmic/Cargo.*|app/cosmic/build.rs|app/cosmic/help/*) app_full=1 ;;
	app/cosmic/src/main.rs|app/cosmic/src/command.rs|app/cosmic/src/chrome.rs) app_full=1 ;;
	app/cosmic/src/page_surface*.rs|app/cosmic/src/page_layout.rs|app/cosmic/src/scenario_tests/mod.rs) app_full=1 ;;
	app/cosmic/src/scenario_tests/*.rs)
		m=${f##*/}; m=${m%.rs}
		scen=$(add "scenarios::$m" "$scen") ;;
	app/cosmic/src/*.rs)
		m=${f##*/}; m=${m%.rs}
		app_filters=$(add "$m::" "$app_filters")
		fam=${m%%_*}
		[ "$fam" != "$m" ] && app_filters=$(add "${fam}_" "$app_filters") ;;
	esac
done

run() {
	echo "+ $*"
	[ "$dry" = 1 ] || "$@"
}
rc=0
if [ "$core_all" = 1 ]; then
	run cargo test --offline --workspace || rc=1
else
	for p in $core_pkgs; do run cargo test --offline -p "$p" || rc=1; done
fi
if [ "$app_full" = 1 ]; then
	run cargo test --offline --manifest-path "$app" || rc=1
	[ "$text" = 1 ] && { run cargo test --offline --no-default-features --manifest-path "$app" || rc=1; }
elif [ -n "$app_filters$scen" ]; then
	# shellcheck disable=SC2086
	run cargo test --offline --manifest-path "$app" -- $app_filters $scen || rc=1
fi
[ -z "$core_pkgs$app_filters$scen" ] && [ "$core_all" = 0 ] && [ "$app_full" = 0 ] && echo "nothing to test"
exit $rc
