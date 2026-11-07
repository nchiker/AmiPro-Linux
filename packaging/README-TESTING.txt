AmiPro-Linux - test build
=========================

What this is
  A native word processor written in Rust for the COSMIC desktop. It aims to
  match Lotus Ami Pro 3.1 and reads and writes its SAM documents and STY style
  sheets. This is a test build: expect rough edges.

Requirements
  64-bit Linux (x86_64) with a graphical session; tested on Pop!_OS COSMIC.
  The program is linked only against the system C library and libxkbcommon
  (package libxkbcommon0, present on any desktop). It loads these at run time,
  and a stock Pop!_OS COSMIC has all of them: libwayland-client0, libEGL
  (libegl1), libvulkan1 (with mesa-vulkan-drivers), libx11-6, libxcb1,
  libxkbcommon-x11-0. If the program does not start or shows a blank window,
  install them:
      sudo apt install libxkbcommon0 libwayland-client0 libegl1 libvulkan1 \
                       mesa-vulkan-drivers libx11-6 libxcb1 libxkbcommon-x11-0
  Printing and Open/Save dialogs use the desktop portals (xdg-desktop-portal,
  already part of COSMIC).

Install (no root needed)
      tar xzf amipro-linux-*-x86_64.tar.gz
      cd amipro-linux-*-x86_64
      ./install.sh
  It puts the program in ~/.local/bin/amipro-linux, a menu entry, an icon and
  the .sam / .sty file types under ~/.local/share, and lists each path. Start it
  from the application menu ("AmiPro-Linux") or by running amipro-linux. Right-
  click a .SAM file > Open With > AmiPro-Linux (log out and in once if the
  entry does not appear). It is NOT made the default program for .SAM files.
  If you want that (optional):
      xdg-mime default amipro-linux.desktop application/x-amipro-document
      xdg-mime default amipro-linux.desktop application/x-amipro-style

Uninstall
      ./uninstall.sh
  removes exactly what install.sh added. Your settings and documents stay.

Where settings are kept
  ~/.config/cosmic/org.amipro_linux.AmiPro/ (delete it to reset the program).
  Nothing else is written outside the files you save.

IMPORTANT: work on COPIES of your documents
  Save writes over the .SAM file; the first time it keeps the old file as
  NAME.SAM.bak. Files imported from other formats (DOCX, ODT, RTF, Markdown,
  text) and STY files are never written over; Save asks for a new name. Still,
  please open copies of anything you care about.

What to try
  1. Open a .SAM file (File > Open, or Open With). Edit text, Save, Save As.
  2. File > Export PDF; also Export Markdown, DOCX, ODT, RTF.
  3. View > Outline mode: promote, demote, Move Up / Move Down.
  4. In a table: Table > Edit Formula...; Tools > Sort...
  5. Preferences > Folders: set the Documents folder and check Open/Save use it.
  6. Anything else you used in Ami Pro: styles, frames, pictures, headers,
     footnotes, find and replace, spelling, mail merge, master documents.

Style sheets (optional)
  No Ami Pro style sheets ship with the app. If you own some, point the
  environment variable AMIPRO_STYLES at a folder of .STY files, e.g.
      AMIPRO_STYLES=$HOME/AmiPro/STYLES amipro-linux

Reporting a problem
  Please send: what you did (steps), what you expected, what you saw, the text
  in the status bar at the bottom, the window size, the contents of VERSION.txt
  (in this folder), and screenshots. If a document is involved, say which
  one (send a copy only if you are happy to share it).

Known limits (not done yet)
  Chart editing, equations, OLE object editing, Page Setup, Fast Format,
  revision marking, formulas outside table cells, forms, inserted rulers, the
  status bar's Face and Point Size buttons, Quick Add. Layout differs from Ami
  Pro in a few rare cases (a very long word beside a frame; a Wrap around frame
  next to a column's first lines). Ctrl+Tab is taken by the desktop; use
  Ctrl+PageDown for Next Document.
