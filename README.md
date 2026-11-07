# Ami Pro for Linux

Native Rust + libcosmic word processor for Pop!_OS COSMIC, targeting the documented capabilities of Lotus Ami Pro 3.1 and original SAM/STY compatibility. The editor is usable (Phases 1–5 built; Phase 6 packaging, updates and crash recovery in progress; release v0.1.0 is published) but not complete: see `docs/KNOWN_LIMITATIONS.md`.

Ami Pro for Linux is an independent project. It is not affiliated with, endorsed by, or sponsored by IBM, HCL, or Lotus. Lotus and Ami Pro are trademarks of their owners; they are used here only to name the file formats and the program this software is compatible with.

Licence: MPL-2.0 (see `LICENSE`). Bundled third-party data: the en_US Hunspell dictionary (`app/cosmic/dictionaries/`, its notices in `en_US-COPYRIGHT.txt`). Thesauri are read from the system at run time.

## Files
- `AGENTS.md` — Codex autonomy, testing, licensing and phase gates.
- `SPEC.md` — product requirements and priority.
- `PROJECT_PLAN.md` — milestone sequence and exit tests.
- `ARCHITECTURE.md` — provisional Rust/libcosmic architecture.
- `FEATURE_PARITY.md` — original-feature evidence and implementation tracking.
- `docs/KNOWN_LIMITATIONS.md` — what is not built, decided against, or different from Ami Pro.
- `docs/INSTALL.md` — installing the `.deb` package.

## Recommended filesystem layout
```text
~/Projects/
  ami-pro-linux/       # this Git repository and the Markdown files
  AmiPro-Reference/    # private original software and documents (NOT in Git)
    iso/               # standalone Ami Pro ISO; optionally SmartSuite ISO
    manuals/
    original-documents/
    analysis/
```
Reference folder can be elsewhere: set `AMIPRO_REFERENCE_DIR=/absolute/path` before launching Codex. Avoid committing proprietary source files, ISO images or documents.

## First Codex instruction
Read `AGENTS.md`, `SPEC.md`, `PROJECT_PLAN.md`, `ARCHITECTURE.md`, and `FEATURE_PARITY.md`. Begin Phase 0 autonomously. Inventory the private reference ISO read-only, verify its Ami Pro version, perform the timeboxed Rust/libcosmic architecture evaluation, document evidence and choose an implementation approach. Then proceed to the first runnable COSMIC vertical slice. Stop only for an actual blocker requiring my action, and provide precise instructions when that happens.

## Build instructions

The independent core has no third-party dependencies:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The COSMIC shell follows the upstream libcosmic application template and pins the verified libcosmic 1.0.0 Git revision. Build or run it with:

```sh
cargo run --manifest-path app/cosmic/Cargo.toml --locked
```

This is the page editing surface: pages, frames, headers, footers, and
tables drawn and edited as laid out. The earlier plain text editor stays
available with `--no-default-features`.

The native binary builds successfully. Interactive editing and visual behavior still require verification in a reachable COSMIC graphical session. See `PROJECT_STATUS.md` for the exact distinction between compiled functionality and pending compatibility/UI checks.
