# Reference-Driven Development Policy

**Status:** Active from 2026-10-09. This policy supplements `SPEC.md`,
`PROJECT_PLAN.md`, and `FEATURE_PARITY.md`; it does not change the accepted
architecture or interrupt the approved interface milestone.

## Purpose and evidence order

Major compatibility work begins with a narrow statement of the Ami Pro 3.1
behavior to reproduce. Prefer evidence in this order:

1. Original manuals, help, file-format documentation, and reproducible behavior
   observed in the authorized Ami Pro 3.1 Windows XP VM.
2. Published specifications and independently developed, suitably licensed
   implementations of the same generic behavior.
3. Static analysis and decompilation of the privately held Ami Pro
   executables, DLLs, and filters. This is authorized as broadly as the work
   needs (user authorization, 2026-10-11); prefer whichever source answers a
   question most reliably.

Do not reconstruct proprietary source code. Never copy original code, resources,
artwork, fonts, dictionaries, or disassembly into the application repository.
Private binaries, Ghidra projects, screenshots containing private documents, and
raw investigation artifacts remain outside Git. Repository evidence consists of
independently written findings, hashes or identifiers where useful, test steps,
and nonproprietary fixtures.

## Required feature brief

Before substantial work on a major feature, add or update a row in
`docs/research/REFERENCE_FEATURE_MATRIX.md` and record:

- the exact user-visible behavior and edge cases to reproduce;
- original evidence already available and uncertainty that remains;
- focused comparison steps in the XP VM, which the agent may operate directly
  (user authorization, 2026-10-11; see `AGENTS.md`);
- open-source implementations or specifications worth studying, including the
  particular algorithm or interface boundary of interest;
- a reuse decision covering implementation effort, integration complexity,
  dependencies, license, performance, maintenance, and model ownership;
- questions that decompilation could answer more reliably than other sources;
- deterministic project tests and the separate original-compatibility test.

Research is timeboxed to the decision it supports. Do not repeat broad editor or
layout-engine audits. Revisit a rejected dependency only when a concrete fixture
or measured limitation crosses a reconsideration gate already documented in the
decision log.

## Verification and status

`FEATURE_PARITY.md` remains the authoritative feature/status matrix and must
distinguish `UNVERIFIED`, `PLANNED`, `IMPLEMENTED`, and `TESTED`. Acceptance text
must separately state whether behavior is model-tested, widget/integration-tested,
physically verified on COSMIC, or compatibility-tested against Ami Pro 3.1.
`IMPLEMENTED` never implies behavioral or compatibility acceptance.

Every completed research increment records reproducible commands or manual steps,
the evidence source/version, the decision, remaining uncertainty, and tests.
Unknown legacy structures remain diagnosed and must not be silently discarded.

## Current sequencing

Since 2026-10-11 the page, page-layout, and frame foundation is the next major
phase after UI-2 acceptance and the SAM import-fidelity fix (`PROJECT_PLAN.md`
Phase 2). Complete its feature brief in `docs/research/REFERENCE_FEATURE_MATRIX.md`
and author controlled fixtures in original Ami Pro before the model decision.
Pagination, tables, and complex style semantics follow the same rule.
