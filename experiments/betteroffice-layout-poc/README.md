# BetterOffice layout POC

This crate is an isolated, non-production experiment. It projects an immutable
`amipro_document::Document` snapshot into BetterOffice's measured-block
contract. It is excluded from the root workspace and is not linked by the
COSMIC application or either production workspace.

Run the behavioral checks:

```sh
cargo test --manifest-path experiments/betteroffice-layout-poc/Cargo.toml
cargo clippy --manifest-path experiments/betteroffice-layout-poc/Cargo.toml --all-targets -- -D warnings
```

Run the release benchmark on a host with the Noto Serif system fonts:

```sh
cargo run --release --manifest-path experiments/betteroffice-layout-poc/Cargo.toml
```

`POC_MODE=layout-only` omits display-list construction and `POC_MODE=current`
measures only the existing estimator. `POC_ITERATIONS` controls BetterOffice
iterations. The benchmark reads system fonts but does not copy or redistribute
them.

See `docs/research/BETTEROFFICE_LAYOUT_POC.md` for results and the decision.
