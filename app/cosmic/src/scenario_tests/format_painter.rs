//! Format Painter, Copy Format, and Paste Format as a user works them
//! (`UI_SPEC.md` §49): real mouse drags on the page, the shortcuts as keys
//! through the window, and Esc through the window. (`commands.rs` covers
//! the modes themselves.)

use cosmic::iced::keyboard::{Key, Modifiers, key::Named};

use super::*;

/// A point on the first page `fraction` along paragraph `paragraph`'s first
/// line.
fn along(s: &mut Scenario, paragraph: usize, fraction: f32) -> cosmic::iced::Point {
    let main = s.main_flow();
    let (x, y) = s.app().pages.layout().pages[0]
        .lines
        .iter()
        .find(|line| line.flow == main && line.paragraph_index == paragraph)
        .map(|line| (line.x + line.width * fraction, line.top + line.height / 2.0))
        .unwrap_or_else(|| s.fail("layout", "no line for the paragraph"));
    page_point(&mut s.rt, 0, x, y)
}

/// A drag over paragraph `from.0` at `from.1` to paragraph `to.0` at `to.1`.
fn drag(s: &mut Scenario, what: &str, from: (usize, f32), to: (usize, f32), edits: bool) {
    let start = along(s, from.0, from.1);
    let end = along(s, to.0, to.1);
    let step = move |rt: &mut Runtime| {
        rt.drag(start, end);
    };
    if edits {
        s.edit(what, step);
    } else {
        s.no_change(what, step);
    }
}

fn marks(s: &Scenario, paragraph: usize, byte: usize) -> (Option<bool>, Option<bool>) {
    let format = s.document().main_flow().paragraphs[paragraph].format_at(byte);
    (format.bold, format.italic)
}

fn key(s: &mut Scenario, what: &str, key: Key, modifiers: Modifiers, edits: bool) {
    let step = move |rt: &mut Runtime| window_key(rt, key, modifiers);
    if edits {
        s.edit(what, step);
    } else {
        s.no_change(what, step);
    }
}

fn character(text: &str) -> Key {
    Key::Character(text.into())
}

#[test]
fn painting_with_the_mouse_and_the_shortcuts_from_one_shot_to_keep_on_and_esc() {
    let mut s = Scenario::new_document("painter-workflow");
    s.clean_folder();
    s.type_text("Source words in bold italic.");
    for paragraph in [
        "Second paragraph of plain text.",
        "Third paragraph of plain text.",
        "Fourth paragraph of plain text.",
        "Fifth paragraph of plain text.",
        "Sixth paragraph of plain text.",
    ] {
        s.enter();
        s.type_text(paragraph);
    }
    s.select_main((0, 0), (0, "Source words".len()));
    s.command(CommandId::Bold);
    s.command(CommandId::Italic);
    s.act("zoom 100%", |rt| {
        rt.send(Message::ZoomTo(100));
        rt.redraw();
    });
    let painted = (Some(true), Some(true));
    let plain = (None, None);

    // One-shot: select the source with the mouse, arm, and drag across two
    // paragraphs. One Undo takes both back (the step's check).
    drag(&mut s, "select in the source", (0, 0.02), (0, 0.25), false);
    s.command_act(CommandId::FormatPainter);
    assert!(s.app().fast_format.as_ref().is_some_and(|f| !f.sticky));
    drag(&mut s, "paint two paragraphs", (1, 0.0), (2, 0.99), true);
    assert_eq!(marks(&s, 1, 3), painted);
    assert_eq!(marks(&s, 2, 3), painted);
    assert_eq!(marks(&s, 3, 3), plain);
    assert!(s.app().fast_format.is_none(), "one selection only");
    drag(
        &mut s,
        "the next drag only selects",
        (3, 0.0),
        (3, 0.5),
        false,
    );

    // Ctrl+Alt+C and Ctrl+Alt+V, as keys, with the mouse left alone.
    drag(&mut s, "select in the source", (0, 0.02), (0, 0.25), false);
    key(
        &mut s,
        "Ctrl+Alt+C",
        character("c"),
        Modifiers::CTRL | Modifiers::ALT,
        false,
    );
    assert!(
        s.app().status.starts_with("Format copied: "),
        "{}",
        s.app().status
    );
    assert!(s.app().fast_format.is_none());
    drag(&mut s, "select the fourth", (3, 0.0), (3, 0.99), false);
    key(
        &mut s,
        "Ctrl+Alt+V",
        character("v"),
        Modifiers::CTRL | Modifiers::ALT,
        true,
    );
    assert_eq!(marks(&s, 3, 3), painted);
    assert_eq!(marks(&s, 4, 3), plain);

    // Ctrl+Shift+T keeps it on: two drags, a step each, then Esc.
    drag(&mut s, "select in the source", (0, 0.02), (0, 0.25), false);
    key(
        &mut s,
        "Ctrl+Shift+T",
        character("t"),
        Modifiers::CTRL | Modifiers::SHIFT,
        false,
    );
    assert!(s.app().fast_format.as_ref().is_some_and(|f| f.sticky));
    drag(&mut s, "paint the fifth", (4, 0.0), (4, 0.99), true);
    drag(&mut s, "paint the sixth", (5, 0.0), (5, 0.99), true);
    assert_eq!(marks(&s, 4, 3), painted);
    assert_eq!(marks(&s, 5, 3), painted);
    assert!(s.app().fast_format.is_some(), "still on");

    // One Undo takes back only the last selection painted.
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!((marks(&s, 4, 3), marks(&s, 5, 3)), (painted, plain));
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });

    key(
        &mut s,
        "Esc",
        Key::Named(Named::Escape),
        Modifiers::empty(),
        false,
    );
    assert!(s.app().fast_format.is_none(), "{}", s.app().status);
    drag(
        &mut s,
        "a drag after Esc paints nothing",
        (1, 0.0),
        (1, 0.5),
        false,
    );

    // The format stays copied: Paste Format still works after Esc.
    s.select_main((0, "Source words ".len()), (0, "Source words in".len()));
    s.command(CommandId::PasteFormat);
    assert_eq!(marks(&s, 0, "Source words i".len()), painted);
    let path = s.save_sam("PAINTFLOW.SAM");
    let reopened = s.reopen(&path);
    assert_eq!(marks(&reopened, 5, 3), painted);
}
