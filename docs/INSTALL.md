# Installing AmiPro-Linux (Pop!_OS / COSMIC, amd64)

AmiPro-Linux is a pre-release. The Debian package installs one binary and the
desktop, MIME, and icon files system-wide. It contains no Ami Pro files,
sample documents, or other proprietary material.

## Build the package (on the build machine)

```sh
cd app/cosmic && cargo build --release -j1 && cd ../..
tools/package-deb.sh        # writes dist/amipro-linux_<version>_amd64.deb
```

The version is the `app/cosmic/Cargo.toml` version plus `+git<short rev>`
(`.dirty` if the tree has uncommitted changes). The script refuses to run if
sources are newer than the release binary.

## Install

```sh
sudo apt install ./amipro-linux_<version>_amd64.deb
```

`apt` pulls in the shared libraries the binary needs. The post-install script
refreshes the MIME, desktop, and icon databases. Start it from the application
launcher ("AmiPro-Linux") or run `amipro-linux [file.sam]`. It registers
`.sam` and `.sty` as Ami Pro types but does not take over as default handler;
set that in Files (Open With) if you want it.

## Uninstall

```sh
sudo apt remove amipro-linux      # or: sudo apt purge amipro-linux
```

Your documents are never touched. Settings (below) stay until you delete them.

## Where things live

- Binary: `/usr/bin/amipro-linux`; desktop entry
  `/usr/share/applications/amipro-linux.desktop`; MIME definition
  `/usr/share/mime/packages/amipro-linux.xml`; icon
  `/usr/share/icons/hicolor/scalable/apps/org.amipro_linux.AmiPro.svg`.
- Settings: COSMIC config, `~/.config/cosmic/org.amipro_linux.AmiPro/`.
- Documents: where you save them. Saving over an existing `.SAM` keeps
  `NAME.SAM.bak` the first time. Imported files and STY files are never
  written over.

## Known limitations

See `FEATURE_PARITY.md` (evidence per feature) and the "Known open items" in
`CLAUDE.md`. Notably: no accessibility nodes for page text (blocked by the
pinned toolkit), IME preedit not yet checked on a real input method, no chart
editing, Page Setup, Fast Format, or macros, and no revision marking. Run C
(table data) and the closing pass still await physical checks.

## Reporting problems

Open an issue in the project repository
(`https://github.com/nchiker/AmiPro-Linux`, currently private) or tell the
maintainer: give the package version (`dpkg -s amipro-linux | grep Version`),
what you did, and, if possible, a small document that shows it (not one
containing private or proprietary content).


---
Ami Pro for Linux is an independent project. It is not affiliated with, endorsed by, or sponsored by IBM, HCL, or Lotus. Lotus and Ami Pro are trademarks of their owners; they are used here only to name the file formats and the program this software is compatible with.
