# Writing Help topics

The program's Help is a set of Markdown files in `app/cosmic/help/<area>/<topic>.md`. They are embedded at build time (`app/cosmic/build.rs` lists the files, `app/cosmic/src/help.rs` includes them), so a new file needs no registration. Design: `docs/Design/UI_SPEC.md` §45.

## The rule: original wording only

Every topic is written for this program, in your own words, from what the program does: `UI_SPEC.md`, `UI_COMMAND_MATRIX.md`, `KNOWN_LIMITATIONS.md`, `COMPATIBILITY.md`, the command registry (`app/cosmic/src/command.rs`), and the running app.

Ami Pro's own help files (`.HLP`, in `../AmiPro-Reference`) are a source of **facts** about how Ami Pro behaves, kept outside Git. Read them to check a fact; never copy, paraphrase closely, translate, or embed their sentences, headings, examples, or screenshots. Do not commit text, images, or tables extracted from them or from the user's guide. Where this program differs from Ami Pro, describe this program.

## A topic file

```text
---
id: styles
title: Styles and style sheets
area: document
keywords: style, style sheet, STY, paragraph style
see_also: text-formatting, outline-mode
context: command:Styles, command:CreateStyle, panel:StyleProperties
---
Opening paragraph: what the thing is, in one or two sentences. It is also the
result's summary in the search, so make it stand on its own.

## A heading

Paragraphs, lists (`-` or `1.`), **bold**, *italic*, `code`, and links to
other topics: [Text formatting](text-formatting).
```

- **File name = `id`**, lower-case letters, digits, and hyphens. The folder is the `area`: `start`, `document`, `objects`, `files`, or `reference` (`AREAS` in `help.rs`; add an area there and in `UI_SPEC.md` §45).
- **title:** a noun phrase, as the user would look for it.
- **keywords:** words the user may type that the text does not contain (old names, abbreviations, file extensions). Searched with the title and the text; a title or keyword hit ranks above a text hit.
- **see_also:** topic ids, listed at the end of the topic. Links in the text use `[label](topic-id)`; a link target must be a topic id (no web addresses).
- **context:** what F1 opens this topic for. `command:Name` is a `CommandId` of the registry (`Sort`, `Find`); `panel:Name` is a `RailPanel` (`Table`, `Updates`). Each context belongs to one topic. The first match in this order wins: the open drawer, Draw mode, the open rail panel, the selected frame's kind, the find bar, a table, Outline mode (`help_contexts` in `help_panel.rs`). A topic with no context is reached by the contents, search, and links.
- Do not repeat the title as a `#` heading; the body starts at `##`. Only `##` and `###` are used.

## Style

Short and plain. Say what the user does, with the menu path as it appears (Format › Page Layout…), the shortcut if the command has one (check `command.rs` or Help › Keyboard Shortcuts), and what happens. Name limits honestly and link to Known limitations. Do not describe a command the program does not have; do not promise. One screen of text is a good size; split a long topic.

## Checks

`cargo test --offline --manifest-path app/cosmic/Cargo.toml help` checks that every topic parses, ids are unique and match their file, every link and See also resolves, every context names a real command or panel, a context has one owner, and that every implemented command with a menu entry has a topic naming it **or** is on the exemption list `WITHOUT_HELP` in `help_panel.rs` (tests module). The exemption list is the to-do list: when you give a command a topic, take it off (the test fails until you do). A new menu command or rail panel needs a context or an exemption.
