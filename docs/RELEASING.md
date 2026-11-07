# Releasing AmiPro-Linux

Releases are GitHub Releases of the public repository
(`https://github.com/nchiker/AmiPro-Linux`). The program's Help › Check for
Updates… reads the latest one (`releases/latest`, no token), so a release must
have exactly this shape:

- tag `vX.Y.Z` (semantic version; `X.Y.Z` is also the crate version in
  `app/cosmic/Cargo.toml`, which the program compares with the tag);
- asset `amipro-linux_X.Y.Z_amd64.deb`;
- asset `SHA256SUMS`, as `sha256sum` writes it, listing that package.

A release without a package, or without `SHA256SUMS`, is reported by the
program as an error and never installed. Pre-releases are not offered
(`releases/latest` skips them).

## Steps

1. Set `version` in `app/cosmic/Cargo.toml` to `X.Y.Z`, update the lock file
   (`cargo build --release -j1` in `app/cosmic` does), commit.
2. Build the binary: `cd app/cosmic && cargo build --release -j1`.
3. Write the release notes in a file (the program shows the first 20 lines,
   so put the summary first).
4. `tools/release.sh X.Y.Z NOTES.md`. It refuses when `Cargo.toml` says
   another version or the tree is dirty; it runs `tools/package-deb.sh` with
   that version, writes `dist/SHA256SUMS`, checks it, and prints the
   `gh release create vX.Y.Z dist/amipro-linux_X.Y.Z_amd64.deb dist/SHA256SUMS
   --title ... --notes-file NOTES.md` command. It publishes nothing.
5. Install the package on a test machine (`sudo apt install ./...deb`) and
   check it starts.
6. Run the printed `gh release create` command yourself, then in an older
   installed version try Help › Check for Updates…
   (`docs/Design/UI_ACCEPTANCE.md`, Updates).

Never put a Reference file, a corpus document, or any Ami Pro material in
`dist/`. Changing the asset names breaks the updater for installed versions;
change the program first and keep the old names for one release.
