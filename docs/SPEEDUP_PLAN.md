# Speeding up the rest of the project (proposal, 2026-11-04)

The user's order of 2026-11-04, item 3: the project is near its end, but
progress has slowed; how does the remaining queue get done faster? This is
a proposal for the user's decision. **Adopted in full by the user on
2026-11-04** (`DECISIONS.md`, SU1–SU4). Run D is to build full editors,
not the recommended scope (SU2), and run E is not planned (SU3).

## What slows a run now (measured)

- **Reading before working.** A session reads about 350 KB of documents
  before it starts work: `CLAUDE.md` alone is 126 KB, and Claude Code
  loads it into every turn. `docs/SESSION_HANDOFF.md` is 137 KB, mostly
  history; `PROJECT_PLAN.md` is 59 KB; and `PROJECT_STATUS.md` is 77 KB.
  Most of this repeats the run records (`docs/*_PROGRESS.md`) that already
  hold it.
- **The exit's paperwork.** Each run updates 12 documents at its exit
  (recipe step 12), and several of those restate each other. The record
  lines in `CLAUDE.md` alone run to thousands of words.
- **Rework after the physical check.** The merge run needed two rework
  sessions after the user's first use. Every defect found then was one a
  whole-task test would have caught. The scenario suite (2026-11-04)
  closes this gap: in one session three lower-cost agents wrote 90
  scenarios over every area. They found 14 defects that the 600 unit
  tests had missed, among them lines lost on empty table cells, five false
  loss lists, and the caret lost after Undo outside the main text. All 14
  were fixed the same session.
- **The full recipe for every run.** Every run maps the feature in the
  decompile, makes VM fixtures one property at a time, and opens twins in
  Ami Pro, even when the run only adds menus and dialogs over structures
  the app already reads and writes.
- **One run at a time, and one decision stop each.** Each run stops once
  for its decisions, then runs to its exit, then waits for a physical
  check.

## The proposal, ranked by what it saves

1. **Cut the start-up reading to about 40 KB.** `CLAUDE.md` becomes a page
   of current state of at most 15 KB: the layout, how to build and test,
   and the "watch for this" notes. Its history moves to `docs/HISTORY.md`,
   which is not loaded. `docs/SESSION_HANDOFF.md` keeps only the current
   handoff (at most 10 KB), and the rest moves to `docs/HANDOFF_ARCHIVE.md`.
   A session then reads `AGENTS.md`, `CLAUDE.md`, the handoff, and the
   run's row in the playbook. *Saves:* tokens and time on every turn of
   every session. *Cost:* one session of editing, delegated, then reviewed.
2. **The scenario suite as the gate before every physical check.** Each
   run adds its scenarios to `app/cosmic/src/scenario_tests/` (written by a
   lower-cost agent from the run's checklist, then reviewed), and they
   must pass before the user's check. *Saves:* the rework sessions.
3. **Merge the nine remaining runs into five:**
   - **A. "Commands"** = templates (15) + the remaining commands (16) +
     views (21). All three are menus and dialogs over existing structures,
     so one light run covers them.
   - **B. Outline mode** (17).
   - **C. Table data** (20): formulas, Quick Add, forms, and the open
     merge point (numbers in table cells).
   - **D. Objects** = charts (18) + equations (19) + OLE and image
     processing (22). Recommended scope: draw what the files hold, keep
     it byte for byte, and edit chart data in a table. Leave equation and
     OLE editing out: equations occur in no file, charts in 34 user files (7 distinct
     records), and OLE objects in 17 (corrected 2026-11-06; the earlier
     "4 and 4" were table-cell hints). Decision SU2 later chose full editors.
   - **E. Revisions** (23): no file in either corpus has revision marking
     on. Recommended: read, show, and keep `<:V…>` marks, plus Doc Compare
     as a plain comparison. Otherwise, list it as not planned.
4. **Two recipe weights.** *Full* (decompile, VM fixtures, twins) applies
   only where a run writes a SAM structure the app has not written before.
   *Light* (spec, decisions, scenarios, one twin) applies to the rest
   (runs A and B). Recommended: A and B light; C, D, and E full.
5. **Less paperwork at the exit.** A run keeps its progress file, its row
   in `FEATURE_PARITY.md`, the playbook's queue, `UI_SPEC.md`'s section,
   and the handoff. `UI_COMMAND_MATRIX.md` would be generated from
   `command.rs` by a test. `PROJECT_STATUS.md` and `PROJECT_PLAN.md` would
   get one line per run. A lower-cost agent would make the remaining
   updates from the progress file.
6. **Ask the decisions for two runs at once**, so that the second run
   starts without a stop (for example, A and B in one round of questions,
   then C and D).
7. **Optional: free disk space for parallel work.** `app/cosmic/target`
   is 52 GB and the disk has 30 GB free. Clearing stale release and
   benchmark builds would make room for one git worktree, so that the
   core and the interface parts of a run could be built by two agents at
   once. This saves the least, so it comes last.

## What it adds up to

Nine runs of about one session each, plus a rework session after about
every second run, become five runs and the start-up cut. That is about
six sessions instead of twelve to fourteen. The open points (accessibility
of the page surface, the IME preedit check, the UI-1 and UI-2 checklists)
could go in one closing session after run E.
