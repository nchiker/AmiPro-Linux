//! Password-protected documents (`UI_SPEC.md` §39, CM5;
//! `docs/research/COMMANDS_NOTES.md` §9): File › Open asks for a
//! protected file's password, as Ami Pro's Password Protect box (dialog
//! 1042); File › Password… sets the document's password with Ami Pro's
//! Enter and Verify (1042, 1043) or removes it. The password belongs to the
//! open document, not its content: it is not an undo step, it marks the
//! document changed, and every save of it as an Ami Pro document writes it
//! protected until the password is removed (CK12).

use std::path::PathBuf;

use cosmic::{Element, widget};

use crate::Message;

/// The password box's id: it takes the keyboard when the box opens.
pub const PASSWORD_FIELD: &str = "password-dialog-password";

/// Ami Pro's warning under Verify password (dialog 1043).
pub const WARNING: &str =
    "WARNING - the contents of protected files cannot be recovered if the password is lost";

#[derive(Clone, Debug, PartialEq)]
pub enum Purpose {
    /// Opening this protected file.
    Open(PathBuf),
    /// File › Password… for the active document.
    Set { protected: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PasswordDialog {
    pub purpose: Purpose,
    pub password: String,
    pub verify: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PasswordAction {
    Password(String),
    Verify(String),
    Submit,
    /// File › Password…: protection off.
    Remove,
    Cancel,
}

/// What the app does after an action.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Stay,
    Cancel,
    Open(PathBuf, String),
    Set(Option<String>),
}

impl PasswordDialog {
    pub fn open(path: PathBuf) -> Self {
        Self {
            purpose: Purpose::Open(path),
            password: String::new(),
            verify: String::new(),
            error: None,
        }
    }

    pub fn set(protected: bool) -> Self {
        Self {
            purpose: Purpose::Set { protected },
            password: String::new(),
            verify: String::new(),
            error: None,
        }
    }

    pub fn update(&mut self, action: PasswordAction) -> Outcome {
        // Ami Pro's box holds 14 characters (a 16-byte buffer).
        let limit = |text: String| -> String {
            text.chars()
                .take(amipro_formats::protect::PASSWORD_LIMIT)
                .collect()
        };
        match action {
            PasswordAction::Password(text) => {
                self.password = limit(text);
                self.error = None;
                Outcome::Stay
            }
            PasswordAction::Verify(text) => {
                self.verify = limit(text);
                self.error = None;
                Outcome::Stay
            }
            PasswordAction::Cancel => Outcome::Cancel,
            PasswordAction::Remove => match self.purpose {
                Purpose::Set { protected: true } => Outcome::Set(None),
                _ => Outcome::Stay,
            },
            PasswordAction::Submit => match &self.purpose {
                Purpose::Open(path) => Outcome::Open(path.clone(), self.password.clone()),
                Purpose::Set { .. } => {
                    if self.password.chars().any(|character| {
                        character.is_control() || u32::from(character) > 0xFF && !cp1252(character)
                    }) {
                        self.error = Some(
                            "A password holds only characters Ami Pro's files can store.".into(),
                        );
                        Outcome::Stay
                    } else if self.password != self.verify {
                        // Ami Pro's message 3179.
                        self.error = Some(
                            "Invalid Password Verification. Please reenter password and reverify."
                                .into(),
                        );
                        self.verify.clear();
                        Outcome::Stay
                    } else {
                        Outcome::Set(Some(self.password.clone()))
                    }
                }
            },
        }
    }
}

/// Whether `character` is one of Windows-1252's above Latin-1.
fn cp1252(character: char) -> bool {
    "€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ".contains(character)
}

pub fn dialog(state: &PasswordDialog) -> Element<'static, Message> {
    let send = |action: PasswordAction| Message::Password(action);
    let password = widget::secure_input("", state.password.clone(), None, true)
        .id(widget::Id::new(PASSWORD_FIELD))
        .on_input(move |text| Message::Password(PasswordAction::Password(text)))
        .on_submit(move |_| Message::Password(PasswordAction::Submit));
    let mut body = widget::column::with_capacity(6).spacing(8);
    let (title, primary) = match &state.purpose {
        Purpose::Open(path) => {
            body = body
                .push(widget::text::body(format!(
                    "{} is password protected.",
                    crate::file_name(path)
                )))
                .push(widget::text::body("Enter password:"))
                .push(password);
            ("Password Protect", "Open")
        }
        Purpose::Set { protected } => {
            let verify = widget::secure_input("", state.verify.clone(), None, true)
                .on_input(move |text| Message::Password(PasswordAction::Verify(text)))
                .on_submit(move |_| Message::Password(PasswordAction::Submit));
            body = body
                .push(widget::text::body(if *protected {
                    "The document is saved with a password. A new one replaces it."
                } else {
                    "Saves of this document as an Ami Pro document ask for this password to open them."
                }))
                .push(widget::text::body("Enter password:"))
                .push(password)
                .push(widget::text::body("Verify password:"))
                .push(verify)
                .push(widget::text::caption(WARNING));
            ("Password Protect", "Set Password")
        }
    };
    if let Some(error) = &state.error {
        body = body.push(widget::text::body(error.clone()));
    }
    let mut dialog = widget::dialog()
        .title(title)
        .control(body)
        .primary_action(widget::button::suggested(primary).on_press(send(PasswordAction::Submit)))
        .secondary_action(
            widget::button::standard("Cancel").on_press(send(PasswordAction::Cancel)),
        );
    if matches!(state.purpose, Purpose::Set { protected: true }) {
        dialog = dialog.tertiary_action(
            widget::button::destructive("Remove Password").on_press(send(PasswordAction::Remove)),
        );
    }
    dialog.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_needs_the_same_password_twice_and_open_takes_one() {
        let mut set = PasswordDialog::set(false);
        set.update(PasswordAction::Password("Secret".into()));
        set.update(PasswordAction::Verify("secret".into()));
        assert_eq!(set.update(PasswordAction::Submit), Outcome::Stay);
        assert!(
            set.error
                .as_deref()
                .unwrap()
                .starts_with("Invalid Password Verification")
        );
        assert!(set.verify.is_empty());
        set.update(PasswordAction::Verify("Secret".into()));
        assert_eq!(
            set.update(PasswordAction::Submit),
            Outcome::Set(Some("Secret".into()))
        );
        assert_eq!(
            set.update(PasswordAction::Remove),
            Outcome::Stay,
            "not protected"
        );
        let mut long = PasswordDialog::set(true);
        long.update(PasswordAction::Password("ABCDEFGHIJKLMNOP".into()));
        assert_eq!(long.password, "ABCDEFGHIJKLMN", "14 characters");
        assert_eq!(long.update(PasswordAction::Remove), Outcome::Set(None));
        let mut open = PasswordDialog::open("A.SAM".into());
        open.update(PasswordAction::Password("A".into()));
        assert_eq!(
            open.update(PasswordAction::Submit),
            Outcome::Open("A.SAM".into(), "A".into())
        );
    }
}
