---
id: updates
title: Updates
area: files
keywords: update, upgrade, new version, release, install, checksum, github
see_also: getting-started, known-limitations
context: command:CheckForUpdates, panel:Updates
---
**Help › Check for Updates…** opens the Updates panel and asks the project's release page whether a newer version exists. It shows the version you have.

- **Up to date:** nothing to do.
- **Newer version:** the panel shows what is new. **Download and Install** downloads the package, checks it against the release's checksum list, and then shows the exact command it will run. **Install** runs it; you enter your password when the system asks. **Restart** starts the new version.
- **Failed:** the panel says why (no network, no release, a checksum that does not match) and offers Check again. Nothing is installed unless the checksum matches.

The program only checks when you ask, unless you turn on **Check for updates when the program starts** in Preferences › General (off by default; once a day at most). Only the project's own release downloads over HTTPS are used.

If the system has no way to install packages from the program, the package is still downloaded and checked, and the panel shows the command to run in a terminal.
