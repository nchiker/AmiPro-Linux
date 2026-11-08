#!/usr/bin/env python3
"""Fails if any Help topic repeats a run of N words from Ami Pro's help dump.

Usage: python3 -I tools/help_leak_check.py [N=7] [DUMP_DIR]
The dump (../AmiPro-Reference/help-dump/<name>/*.txt, made by tools/hlp_dump.py)
stays outside Git; without it the check says so and exits 0.
"""
import os, re, sys, glob

n = int(sys.argv[1]) if len(sys.argv) > 1 else 7
here = os.path.dirname(os.path.abspath(__file__))
dump = sys.argv[2] if len(sys.argv) > 2 else os.path.join(here, "..", "..", "AmiPro-Reference", "help-dump")
files = glob.glob(os.path.join(dump, "*", "*.txt"))
if not files:
    print("no help dump found; nothing to compare")
    sys.exit(0)

def words(text):
    return re.findall(r"[a-z0-9']+", text.lower())

grams = set()
for f in files:
    w = words(open(f, encoding="utf-8", errors="ignore").read())
    for i in range(len(w) - n + 1):
        grams.add(tuple(w[i:i + n]))

bad = 0
for f in sorted(glob.glob(os.path.join(here, "..", "app", "cosmic", "help", "**", "*.md"), recursive=True)):
    w = words(open(f, encoding="utf-8").read())
    hits = [" ".join(w[i:i + n]) for i in range(len(w) - n + 1) if tuple(w[i:i + n]) in grams]
    if hits:
        bad += 1
        print(f"{os.path.relpath(f, os.path.join(here, '..'))}: {len(hits)} shared {n}-word runs, e.g. '{hits[0]}'")
print(f"checked against {len(files)} dump files with N={n}: {bad} topic(s) share text")
sys.exit(1 if bad else 0)
