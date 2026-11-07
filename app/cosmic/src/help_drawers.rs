//! Help › Keyboard Shortcuts and Help › Compatibility and Fidelity
//! (`UI_SPEC.md` §39, CM12): two side drawers beside About. The shortcuts
//! are generated from the command registry, so the list can never disagree
//! with the menus; the compatibility list is the loss check Save As SAM
//! runs, shown without saving.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::command::{self, CommandId, Implementation, MENU_LAYOUTS, MenuRoot, MenuSlot};

#[derive(Clone, Debug, PartialEq)]
pub enum HelpDrawer {
    Shortcuts {
        filter: String,
    },
    Compatibility {
        /// The session the list was made for.
        session: u64,
        /// The document's tab label when checked.
        name: String,
        /// What saving as SAM would change, or why the check failed.
        losses: Result<Vec<String>, String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum HelpAction {
    Filter(String),
    /// Runs the loss check again for the active document.
    CheckAgain,
    Close,
}

impl HelpDrawer {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Shortcuts { .. } => "Keyboard Shortcuts",
            Self::Compatibility { .. } => "Compatibility and Fidelity",
        }
    }
}

/// A shortcut as the menus show it (`Ctrl+Shift+N`).
pub fn shortcut_label(id: CommandId) -> Option<String> {
    command::metadata(id)
        .shortcut
        .map(|shortcut| shortcut.key_bind().to_string().replace(" + ", "+"))
}

/// Every implemented command with a shortcut, by menu in menu order:
/// (menu, [(label, shortcut)]). With a filter only the entries whose label,
/// shortcut, or menu contain it (any case).
pub fn shortcut_rows(filter: &str) -> Vec<(MenuRoot, Vec<(&'static str, String)>)> {
    let filter = filter.trim().to_lowercase();
    MENU_LAYOUTS
        .iter()
        .filter_map(|(root, slots)| {
            let rows: Vec<_> = MenuSlot::commands(slots)
                .into_iter()
                .filter(|id| command::metadata(*id).implementation == Implementation::Implemented)
                .filter_map(|id| {
                    let label = command::metadata(id).label;
                    shortcut_label(id).map(|shortcut| (label, shortcut))
                })
                .filter(|(label, shortcut)| {
                    filter.is_empty()
                        || label.to_lowercase().contains(&filter)
                        || shortcut.to_lowercase().contains(&filter)
                        || root.label().to_lowercase().contains(&filter)
                })
                .collect();
            (!rows.is_empty()).then_some((*root, rows))
        })
        .collect()
}

fn send(action: HelpAction) -> Message {
    Message::HelpDrawer(action)
}

pub fn view(drawer: &HelpDrawer, active_session: u64) -> Element<'_, Message> {
    match drawer {
        HelpDrawer::Shortcuts { filter } => shortcuts_view(filter),
        HelpDrawer::Compatibility {
            session,
            name,
            losses,
        } => compatibility_view(*session == active_session, name, losses),
    }
}

fn shortcuts_view(filter: &str) -> Element<'_, Message> {
    let mut children: Vec<Element<'_, Message>> = vec![
        widget::search_input("Find a command or key", filter.to_owned())
            .on_input(|text| send(HelpAction::Filter(text)))
            .on_clear(send(HelpAction::Filter(String::new())))
            .width(Length::Fill)
            .into(),
    ];
    let groups = shortcut_rows(filter);
    if groups.is_empty() {
        children.push(widget::text::body("No shortcut matches.").into());
    }
    for (root, rows) in groups {
        children.push(widget::text::heading(root.label()).into());
        for (label, shortcut) in rows {
            children.push(
                widget::row::with_children(vec![
                    widget::text::body(label.trim_end_matches('…'))
                        .width(Length::Fill)
                        .into(),
                    widget::text::body(shortcut).into(),
                ])
                .spacing(8)
                .into(),
            );
        }
    }
    children.push(
        widget::text::caption(
            "On the page the arrow keys, Home, End, Page Up and Page Down move the caret; \
             with Shift they select, with Ctrl they move by word or paragraph.",
        )
        .into(),
    );
    widget::column::with_children(children)
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn compatibility_view<'a>(
    current: bool,
    name: &'a str,
    losses: &'a Result<Vec<String>, String>,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = Vec::new();
    children.push(widget::text::heading(name).into());
    match losses {
        Ok(losses) if losses.is_empty() => children.push(
            widget::text::body("Ami Pro 3.1 can store everything in this document exactly.").into(),
        ),
        Ok(losses) => {
            children.push(
                widget::text::body(
                    "Ami Pro 3.1 cannot store everything in this document exactly. \
                     Saving as an Ami Pro document changes:",
                )
                .into(),
            );
            for loss in losses {
                children.push(widget::text::body(format!("• {loss}")).into());
            }
        }
        Err(error) => children
            .push(widget::text::body(format!("The check could not be made: {error}")).into()),
    }
    children.push(
        widget::text::caption(if current {
            "Checked without saving. Check again after editing."
        } else {
            "This list is for another document. Check again for the one shown."
        })
        .into(),
    );
    children.push(
        widget::button::standard("Check Again")
            .on_press(send(HelpAction::CheckAgain))
            .into(),
    );
    widget::column::with_children(children)
        .spacing(8)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shortcut_list_comes_from_the_registry_in_menu_order() {
        let groups = shortcut_rows("");
        let roots: Vec<_> = groups.iter().map(|(root, _)| *root).collect();
        assert_eq!(roots.first(), Some(&MenuRoot::File));
        let file = &groups[0].1;
        assert_eq!(file[0], ("New Document", "Ctrl+N".to_owned()));
        // Every implemented command with a shortcut is listed once.
        let listed: usize = groups.iter().map(|(_, rows)| rows.len()).sum();
        let expected = command::COMMANDS
            .iter()
            .filter(|c| c.shortcut.is_some() && c.implementation == Implementation::Implemented)
            .count();
        assert_eq!(listed, expected);
    }

    #[test]
    fn the_shortcut_filter_matches_labels_keys_and_menus_in_any_case() {
        let bold = shortcut_rows("bold");
        assert!(
            bold.iter()
                .flat_map(|(_, rows)| rows)
                .any(|(label, shortcut)| *label == "Bold" && shortcut == "Ctrl+B")
        );
        assert!(
            shortcut_rows("CTRL+B")
                .iter()
                .flat_map(|(_, rows)| rows)
                .any(|(label, _)| *label == "Bold")
        );
        assert!(shortcut_rows("no such command anywhere").is_empty());
        let window = shortcut_rows("window");
        assert!(window.iter().any(|(root, _)| *root == MenuRoot::Window));
    }
}
