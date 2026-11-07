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

## Crash recovery

While you edit, the program keeps hidden copies of documents with unsaved
changes in `~/.local/share/amipro-linux/recovery/` (every 2 minutes by default;
Preferences › General turns it off or sets 1, 2, 5, or 10 minutes). It never
writes your own file. After a crash or power loss the next start opens the
"Recover documents" panel (also File › Recover Documents…): Recover opens a
copy as an unsaved tab, Discard deletes it. Copies are removed when you save,
when you close a document without saving, and when the program exits normally;
copies from sessions older than 30 days are deleted at start. Documents with a
password are not copied.

## Updates

Help › Check for Updates… looks at the project's GitHub Releases
(`https://github.com/nchiker/AmiPro-Linux/releases`) when you ask. If a newer
version exists it shows the release notes; Download and Install fetches the
package and `SHA256SUMS`, checks the package's SHA-256 (a mismatch is refused
and the file deleted), shows the command it will run, and on your confirmation
runs `pkexec apt-get install -y <file>` (one password prompt), then offers to
restart. Without `pkexec` or `apt-get` it shows the command for you to run. It
needs `curl`. Preferences › General can check once a day at start (off by
default; it only speaks up when a newer version exists). To update by hand
instead, download the `.deb` from the Releases page and run
`sudo apt install ./amipro-linux_<version>_amd64.deb`.

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

See `docs/KNOWN_LIMITATIONS.md` for what is not built, decided against,
or different from Ami Pro, and `FEATURE_PARITY.md` for the evidence per
feature. Notably: no accessibility nodes for page text (blocked by the pinned
toolkit), IME preedit not yet checked on a real input method, no macros, no
revision marking or Doc Compare, no live OLE editing, and no Window › New
View or vertical ruler. Runs C (table data) and D (objects) and the new
packaging, update, and crash-recovery features still await physical checks.

## Reporting problems

Open an issue in the project repository
(`https://github.com/nchiker/AmiPro-Linux`) or tell the
maintainer: give the package version (`dpkg -s amipro-linux | grep Version`),
what you did, and, if possible, a small document that shows it (not one
containing private or proprietary content).


---
Ami Pro for Linux is an independent project. It is not affiliated with, endorsed by, or sponsored by IBM, HCL, or Lotus. Lotus and Ami Pro are trademarks of their owners; they are used here only to name the file formats and the program this software is compatible with.
