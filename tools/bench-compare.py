#!/usr/bin/env python3
"""Applies the runbook's noise rule to a tools/bench-ab.sh summary.

Usage: python3 -I tools/bench-compare.py target/bench-ab/<time>/summary.txt

For every operation, B (the working tree) regresses against A (the base)
only if B's median is worse than A's by more than max(10%, 0.05 ms) in at
least two rounds, or B's p95 is worse by more than 25% in at least two
rounds (docs/PHASE2_RUNBOOK.md, section 4). Prints one line per operation
and a final verdict; exits 1 if any operation regresses.
"""

import re
import sys
from collections import defaultdict

SECTION = re.compile(r"^== (A|B)-(default|rich)-(\d+)$")
SCENARIO = re.compile(r"^(\S.*) \(line \d+\):$")
DEFAULT_OP = re.compile(r"^\s+([^:]+): total median ([\d.]+) ms, p95 ([\d.]+) ms")
RICH_PAIR = re.compile(r"([A-Za-z][A-Za-z -]*?) median ([\d.]+) ms, p95 ([\d.]+) ms")


def parse(path):
    # results[(editor, operation)][side][round] = (median, p95)
    results = defaultdict(lambda: defaultdict(dict))
    side = editor = round_number = scenario = None
    with open(path, encoding="utf-8") as summary:
        for line in summary:
            line = line.rstrip("\n")
            match = SECTION.match(line)
            if match:
                side, editor, round_number = match.group(1), match.group(2), int(match.group(3))
                scenario = None
                continue
            if side is None:
                continue
            match = SCENARIO.match(line)
            if match:
                scenario = match.group(1)
                continue
            match = DEFAULT_OP.match(line)
            if match and editor == "default":
                key = (editor, f"{scenario}: {match.group(1)}")
                results[key][side][round_number] = (float(match.group(2)), float(match.group(3)))
                continue
            if editor == "rich" and line.startswith("rich editor controller baseline"):
                for pair in RICH_PAIR.finditer(line.split(";", 1)[1]):
                    name = pair.group(1).strip()
                    key = (editor, name)
                    results[key][side][round_number] = (float(pair.group(2)), float(pair.group(3)))
    return results


def main():
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    results = parse(sys.argv[1])
    regressions = []
    for key in sorted(results):
        sides = results[key]
        rounds = sorted(set(sides["A"]) & set(sides["B"]))
        if not rounds:
            continue
        median_worse = p95_worse = 0
        cells = []
        for number in rounds:
            (a_median, a_p95), (b_median, b_p95) = sides["A"][number], sides["B"][number]
            if b_median - a_median > max(0.10 * a_median, 0.05):
                median_worse += 1
            if a_p95 > 0 and b_p95 > 1.25 * a_p95:
                p95_worse += 1
            cells.append(f"{a_median:.3f}/{b_median:.3f}")
        threshold = 2 if len(rounds) >= 3 else len(rounds)
        flags = []
        if median_worse >= threshold:
            flags.append("MEDIAN")
        if p95_worse >= threshold:
            flags.append("P95")
        verdict = "REGRESSION " + "+".join(flags) if flags else "ok"
        if flags:
            regressions.append(key)
        editor, operation = key
        print(f"{verdict:<22} {editor:<7} {operation:<40} A/B medians ms: {'  '.join(cells)}")
    print(f"verdict: {'REGRESSION in ' + str(len(regressions)) if regressions else 'no regression'}")
    return 1 if regressions else 0


if __name__ == "__main__":
    sys.exit(main())
