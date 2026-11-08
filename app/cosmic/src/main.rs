use std::collections::HashMap;
use std::ops::{Deref, DerefMut, RangeInclusive};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "page-surface")]
mod character_dialogs;
#[cfg(feature = "page-surface")]
mod chart_commands;
#[cfg(feature = "page-surface")]
mod chart_panel;
mod chrome;
mod clock;
#[cfg(feature = "page-surface")]
mod colour_control;
#[cfg(feature = "page-surface")]
mod colours;
mod command;
#[cfg(feature = "page-surface")]
mod contents_commands;
#[cfg(feature = "page-surface")]
mod contents_panel;
#[cfg(feature = "page-surface")]
mod data_file;
#[cfg_attr(
    feature = "rich-editor-prototype",
    allow(
        dead_code,
        reason = "the accepted editor is retained for non-prototype builds"
    )
)]
mod document_highlighter;
#[cfg(feature = "page-surface")]
mod document_properties;
#[cfg(feature = "page-surface")]
mod draw_commands;
#[cfg(feature = "page-surface")]
mod draw_files;
#[cfg(feature = "page-surface")]
mod draw_mode;
#[cfg(feature = "page-surface")]
mod drawing_properties;
#[cfg(feature = "page-surface")]
mod envelope_panel;
#[cfg(feature = "page-surface")]
mod equation_commands;
#[cfg(feature = "page-surface")]
mod equation_panel;
#[cfg(feature = "page-surface")]
mod export;
#[cfg(feature = "page-surface")]
mod export_folder;
#[cfg(feature = "page-surface")]
mod fast_format;
#[cfg(feature = "page-surface")]
mod field_commands;
#[cfg(feature = "page-surface")]
mod fields_panel;
mod file_dialogs;
mod file_workflow;
#[cfg(feature = "page-surface")]
mod find_bar;
#[cfg(feature = "page-surface")]
mod find_set;
#[cfg(feature = "page-surface")]
mod frame_editing;
#[cfg(feature = "page-surface")]
mod frame_inspector;
#[cfg(feature = "page-surface")]
mod glossary_commands;
#[cfg(feature = "page-surface")]
mod glossary_panel;
#[cfg(feature = "page-surface")]
mod grammar;
#[cfg(feature = "page-surface")]
mod help;
mod help_drawers;
#[cfg(feature = "page-surface")]
mod help_panel;
mod import;
#[cfg(feature = "page-surface")]
mod import_folder;
#[cfg(feature = "page-surface")]
mod import_report;
#[cfg(test)]
mod layout_comparison;
#[cfg(feature = "page-surface")]
mod lengths;
#[cfg(feature = "page-surface")]
mod links_panel;
#[cfg(feature = "page-surface")]
mod master;
#[cfg(feature = "page-surface")]
mod master_commands;
#[cfg(feature = "page-surface")]
mod master_panel;
#[cfg(feature = "page-surface")]
mod merge;
#[cfg(feature = "page-surface")]
mod merge_commands;
#[cfg(feature = "page-surface")]
mod merge_panel;
#[cfg(feature = "page-surface")]
mod metafile_render;
mod new_document_dialog;
#[cfg(feature = "page-surface")]
mod outline_navigator;
#[cfg(feature = "page-surface")]
mod outline_view;
#[cfg(feature = "page-surface")]
mod page_commands;
#[cfg(feature = "page-surface")]
mod page_dialogs;
#[cfg(feature = "page-surface")]
mod page_editing;
mod page_layout;
#[cfg(feature = "page-surface")]
mod page_layout_properties;
mod page_preview;
#[cfg(feature = "page-surface")]
mod page_surface;
#[cfg(all(test, feature = "page-surface"))]
mod page_surface_app_tests;
#[cfg(feature = "page-surface")]
mod page_view;
#[cfg(feature = "page-surface")]
mod paragraph_properties;
mod password_dialog;
#[cfg(feature = "page-surface")]
mod paste_special;
#[cfg(feature = "page-surface")]
mod pdf;
#[cfg(feature = "page-surface")]
mod pdf_pictures;
#[cfg(feature = "page-surface")]
mod picture_insert;
#[cfg(feature = "page-surface")]
mod picture_inspector;
#[cfg(feature = "page-surface")]
mod pictures;
mod preferences_dialog;
#[cfg(feature = "page-surface")]
mod print;
#[cfg(feature = "page-surface")]
mod print_panel;
#[cfg(feature = "page-surface")]
mod print_portal;
#[cfg(feature = "page-surface")]
mod proofing_commands;
#[cfg(feature = "page-surface")]
mod proofing_panel;
#[cfg(all(test, feature = "page-surface"))]
mod rail_panel_tests;
#[cfg(feature = "page-surface")]
mod rail_panels;
#[cfg(feature = "page-surface")]
mod recovery;
#[cfg(feature = "page-surface")]
mod recovery_commands;
#[cfg(feature = "page-surface")]
mod recovery_panel;
#[cfg(feature = "page-surface")]
mod reference_commands;
#[cfg(feature = "page-surface")]
mod references_panel;
mod revert_saved;
#[cfg(feature = "page-surface")]
mod sort_commands;
#[cfg(feature = "page-surface")]
mod sort_panel;
#[cfg(feature = "page-surface")]
mod special_character;
#[cfg(feature = "page-surface")]
mod spelling;
#[cfg(feature = "page-surface")]
mod table_commands;
#[cfg(feature = "page-surface")]
mod table_editing;
#[cfg(feature = "page-surface")]
mod table_formula;
#[cfg(feature = "page-surface")]
mod table_properties;
#[cfg(feature = "page-surface")]
mod updates;
#[cfg(feature = "page-surface")]
mod updates_commands;
#[cfg(feature = "page-surface")]
mod updates_panel;
#[cfg(feature = "page-surface")]
mod view_modes;
// Until the right-click menus (RU.6) use every gesture.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the derived COSMIC Text layout is dormant until the custom page widget consumes it"
    )
)]
mod rich_buffer;
mod rich_editor_prototype;
mod rich_paragraph;
#[cfg(feature = "page-surface")]
#[cfg_attr(not(test), allow(dead_code))]
mod ruler;
#[cfg(feature = "page-surface")]
mod ruler_view;
mod style_dialogs;
#[cfg(feature = "page-surface")]
mod style_properties;
mod workspace_geometry;

use amipro_document::{
    Alignment, Caps, CharacterFormat, Document, Paragraph, ParagraphStyle, StyleId,
};
use amipro_editing::{CharacterAttribute, History};
use chrome::{
    ToolbarEdit, ToolbarEditor, ToolbarItem, action_bar_layout, action_icon_name, command_tooltip,
    status_bar_model,
};
use command::{COMMANDS, CommandId, EnableRule, Implementation, MENU_LAYOUTS, MenuSlot};
use cosmic::app::context_drawer::{self, ContextDrawer};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::advanced::text::{Ellipsize, EllipsizeHeightLimit, LineHeight, Wrapping};
use cosmic::iced::widget::scrollable::{Direction, Scrollbar};
use cosmic::iced::{Event, Length, Limits, Subscription, Task, event, keyboard};
use cosmic::prelude::*;
use cosmic::widget::RcElementWrapper;
use cosmic::widget::about::About;
use cosmic::widget::menu::action::MenuAction;
use cosmic::widget::menu::{self, ItemHeight, ItemWidth, KeyBind};
use cosmic::widget::{self, segmented_button, text_editor};
use document_highlighter::{
    DocumentHighlightSettings, DocumentHighlighter, default_document_font,
    document_highlight_format,
};
use file_workflow::{
    DocumentLocation, OverwriteConfirmation, SaveFormat, SaveRequest, SaveSelection,
    classify_save_selection, save_native_document, style_sheet_destination,
    suggested_style_sheet_name, write_new_file,
};
#[cfg(feature = "page-surface")]
use frame_editing::{FrameAction, FrameSelection};
#[cfg(feature = "page-surface")]
use page_editing::{PageAction, PageSelection};
#[cfg(feature = "page-surface")]
use page_view::{PageView, Zoom};
#[cfg(feature = "page-surface")]
use rail_panels::RailPanel;
#[cfg(feature = "rich-editor-prototype")]
use rich_editor_prototype::{Action as PrototypeAction, Selection as PrototypeSelection};
use workspace_geometry::PageViewGeometry;
#[cfg(not(feature = "rich-editor-prototype"))]
use workspace_geometry::SCREEN_PIXELS_PER_POINT;

const APP_ID: &str = "org.amipro_linux.AmiPro";

// Approved Revision 2 chrome geometry (`docs/Design/UI_SPEC.md` section 4).
const ACTION_BAR_HEIGHT: f32 = 40.0;
const TAB_STRIP_HEIGHT: f32 = 44.0;
const STATUS_BAR_HEIGHT: f32 = 28.0;
const CHROME_HORIZONTAL_PADDING: u16 = 8;

fn main() -> cosmic::iced::Result {
    DropTrace::install();
    let settings = cosmic::app::Settings::default()
        .size_limits(Limits::NONE.min_width(900.0).min_height(640.0));
    cosmic::app::run::<AmiProApp>(settings, ())
}

/// A Save As SAM the loss check stopped (decision B): where it goes and
/// what would not be written exactly.
#[derive(Clone, Debug, PartialEq)]
struct PendingSamSave {
    path: PathBuf,
    losses: Vec<String>,
    saved_at: u32,
    /// Written over an existing file (Save, or Save As after Replace).
    replace: bool,
}

/// A save that was refused or failed: a dialog says why and offers to
/// choose another name (the status bar alone was easy to miss; user,
/// 2026-10-13).
#[derive(Clone, Debug, PartialEq)]
struct SaveProblem {
    title: String,
    body: String,
    retry: SaveRetry,
}

/// What Choose Another Name… opens again.
#[derive(Clone, Debug, PartialEq)]
enum SaveRetry {
    SaveAs,
    StyleSheet { description: String, contents: bool },
}

impl SaveProblem {
    fn save_as(error: &file_workflow::SaveSafetyError) -> Self {
        use file_workflow::SaveSafetyError;
        let (title, body) = match error {
            SaveSafetyError::StyleSheetTarget => (
                "Not a document name".to_owned(),
                "Style sheets are saved with Format › Styles › Save as a Style Sheet. Choose \
                 a name ending in .amipro or .SAM."
                    .to_owned(),
            ),
            SaveSafetyError::MissingFileName | SaveSafetyError::LegacyTarget => (
                "Choose a file name".to_owned(),
                format!("The document was not saved: {error}."),
            ),
        };
        Self {
            title,
            body,
            retry: SaveRetry::SaveAs,
        }
    }

    fn write_failed(path: &std::path::Path, error: &dyn std::fmt::Display) -> Self {
        Self {
            title: format!("Could not save {}", file_name(path)),
            body: format!("{error}. The document was not saved; choose another place or name."),
            retry: SaveRetry::SaveAs,
        }
    }

    /// The document cannot be written as it is (a value holding a data
    /// file's delimiter): nothing was written; the value needs changing.
    fn not_written(path: &std::path::Path, error: &str) -> Self {
        Self {
            title: format!("Could not save {}", file_name(path)),
            body: format!("{error}. Nothing was written; change the value and save again."),
            retry: SaveRetry::SaveAs,
        }
    }

    fn style_sheet(
        error: &file_workflow::StyleSheetSaveError,
        description: &str,
        contents: bool,
    ) -> Self {
        use file_workflow::StyleSheetSaveError;
        let (title, body) = match error {
            StyleSheetSaveError::Exists(path) => (
                format!("{} already exists", file_name(path)),
                "Style sheets are saved only to new files, so an existing sheet is never \
                 replaced. Choose another name."
                    .to_owned(),
            ),
            StyleSheetSaveError::NotAStyleSheetName => (
                "Not a style sheet name".to_owned(),
                "A style sheet's name ends in .sty. Choose another name.".to_owned(),
            ),
            StyleSheetSaveError::MissingFileName => (
                "Choose a file name".to_owned(),
                "The style sheet was not saved: a file name is required.".to_owned(),
            ),
            StyleSheetSaveError::Write(error) => (
                "Could not save the style sheet".to_owned(),
                format!("{error}. Choose another place or name."),
            ),
        };
        Self {
            title,
            body,
            retry: SaveRetry::StyleSheet {
                description: description.to_owned(),
                contents,
            },
        }
    }
}

/// Whether `path` names an Ami Pro document (`.sam` in any case).
fn is_sam_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The status bar in picture mode (`UI_SPEC.md` §25, P6).
#[cfg(feature = "page-surface")]
const PICTURE_MODE_NOTE: &str =
    "Drag the picture or use the arrow keys (Ctrl for larger steps); Esc returns";

/// What an undo step keeps of where the user was: the hidden editor's
/// cursor (the main text) and, on the page surface, the page selection in
/// whichever flow it is (a header, a footer, a frame, a cell), so that Undo
/// and Redo put the caret back where each step was made or left it.
#[derive(Clone, Debug)]
struct HistoryCaret {
    cursor: text_editor::Cursor,
    #[cfg(feature = "page-surface")]
    page: PageSelection,
    /// The selected frames: Redo of Insert › Frame… selects the frame again.
    #[cfg(feature = "page-surface")]
    frames: Option<FrameSelection>,
}

/// How long a pause between keys ends a run of typing (SU4).
#[cfg(feature = "page-surface")]
const TYPING_PAUSE: std::time::Duration = std::time::Duration::from_secs(2);

/// What a run of key presses does: keys of one kind join one undo step.
#[cfg(feature = "page-surface")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypingKind {
    Insert,
    Backspace,
    Delete,
}

#[cfg(feature = "page-surface")]
impl TypingKind {
    fn of(action: &PageAction) -> Option<Self> {
        match action {
            PageAction::Insert(_) => Some(Self::Insert),
            PageAction::Backspace => Some(Self::Backspace),
            PageAction::Delete => Some(Self::Delete),
            _ => None,
        }
    }
}

/// A run of typing on the page surface (SU4): its keys undo as one step,
/// until the caret moves, a command runs, another kind of key is pressed,
/// or there is a pause of [`TYPING_PAUSE`].
#[cfg(feature = "page-surface")]
#[derive(Clone, Copy, Debug)]
struct TypingRun {
    kind: TypingKind,
    /// The caret after the run's last key.
    at: amipro_editing::TextPosition,
    /// The history's change count after the run's last key: anything else
    /// that changed the history since ends the run.
    changes: u64,
    last: std::time::Instant,
}

struct DocumentSession {
    id: u64,
    rich_layout_generation: u64,
    /// The password the document opens with (CM5): saves as an Ami Pro
    /// document write it protected (`amipro_formats::protect`).
    password: Option<String>,
    document: Document,
    editor: text_editor::Content,
    location: DocumentLocation,
    status: String,
    /// Previous and undone document roots, each with the editor cursor.
    history: History<HistoryCaret>,
    overwrite_confirmation: OverwriteConfirmation,
    /// Save As SAM waiting for the user's answer: what Ami Pro cannot store
    /// exactly (decision B).
    sam_save: Option<PendingSamSave>,
    /// A save that was refused or failed, shown in a dialog until answered.
    save_problem: Option<SaveProblem>,
    dirty: bool,
    clean_document: Document,
    /// When this session opened the document: a save adds the minutes
    /// since to its editing time, as Ami Pro does (`[desc]` value 9).
    opened_at: std::time::Instant,
    /// The time of this session's last save, in Ami Pro's time base.
    saved_at: Option<u32>,
    scroll_offset: cosmic::iced::widget::scrollable::AbsoluteOffset,
    /// The document's pages, laid out again after every change.
    pages: page_layout::SessionPages,
    /// The main text's word count, refreshed after every message.
    words: chrome::WordCount,
    /// The page surface's selection, in any flow.
    #[cfg(feature = "page-surface")]
    page_selection: PageSelection,
    /// The selected frames (`UI_SPEC.md` §11); while there are any, there is
    /// no caret.
    #[cfg(feature = "page-surface")]
    frame_selection: Option<FrameSelection>,
    /// When the last frame key press ran and the undo depth after it:
    /// presses within a second join one undo step.
    #[cfg(feature = "page-surface")]
    frame_keys: Option<(std::time::Instant, usize)>,
    /// The run of typing the last key press made or joined (SU4).
    #[cfg(feature = "page-surface")]
    typing_run: Option<TypingRun>,
    /// The table cell the caret edited and may still be in: when the caret
    /// leaves it, a number typed there is rewritten as Ami Pro stores it
    /// (TD.5, `rewrite_number`).
    #[cfg(feature = "page-surface")]
    number_pending: Option<amipro_document::FlowId>,
    /// The selected frames wait for the user to confirm their deletion.
    #[cfg(feature = "page-surface")]
    pending_frame_delete: bool,
    /// What the last import changed or left out, for the Import Report
    /// (IM8).
    import_report: Option<import::Report>,
    /// The Import Report shows in the rail.
    #[cfg(feature = "page-surface")]
    import_report_shown: bool,
    /// The Go To dialog, while it is open.
    #[cfg(feature = "page-surface")]
    go_to: Option<GoToState>,
    /// The find bar, while it shows (`UI_SPEC.md` §27).
    #[cfg(feature = "page-surface")]
    find: Option<find_bar::FindBar>,
    /// Every field's shown text, for Field Shading.
    #[cfg(feature = "page-surface")]
    field_shading: page_surface::Highlights,
    /// Every field, as of the last change (`fields::all_fields`).
    #[cfg(feature = "page-surface")]
    fields: Vec<amipro_editing::fields::FieldAt>,
    /// Format › Fast Format's picked-up format while the mode is on (CM4).
    #[cfg(feature = "page-surface")]
    fast_format: Option<fast_format::FastFormat>,
    /// A field's prompt waiting for an answer (Query$, Decide).
    #[cfg(feature = "page-surface")]
    field_prompt: Option<field_commands::FieldPrompt>,
    /// Ami Pro's Print Options for this document (`UI_SPEC.md` §29).
    #[cfg(feature = "page-surface")]
    print_options: print::PrintOptions,
    /// The Print panel in the rail, while it shows (the pages then show as
    /// they print).
    #[cfg(feature = "page-surface")]
    print_panel: Option<print_panel::PrintPanel>,
    /// File › Export ▸ waiting for the user's answer: what the format
    /// cannot hold (`UI_SPEC.md` §30, EX7).
    #[cfg(feature = "page-surface")]
    export_pending: Option<export::Prepared>,
    /// The file the last export wrote, which the status bar's Open opens
    /// (EX10).
    #[cfg(feature = "page-surface")]
    exported: Option<PathBuf>,
    /// File › Import Folder… in the rail (`UI_SPEC.md` §31, IM11).
    #[cfg(feature = "page-surface")]
    import_folder: Option<import_folder::ImportFolder>,
    /// File › Export ▸ Folder… in the rail (`UI_SPEC.md` §30, EX6).
    #[cfg(feature = "page-surface")]
    export_folder: Option<export_folder::ExportFolder>,
    /// Save as a Style Sheet's description, while its dialog is open.
    save_sheet: Option<String>,
    /// Save as a Style Sheet's With contents (Ami Pro's default: on).
    save_sheet_contents: bool,
    /// Format › Styles' dialogs, while open.
    create_style: Option<style_dialogs::CreateStyle>,
    style_management: Option<style_dialogs::StyleManagement>,
    use_sheet: Option<style_dialogs::UseStyleSheet>,
    /// The style being renamed from the Styles panel, and the name typed.
    rename_style: Option<(StyleId, String)>,
    /// Style Properties in the rail, while it shows (`UI_SPEC.md` §15).
    #[cfg(feature = "page-surface")]
    style_properties: Option<style_properties::StyleProperties>,
    /// Format › Paragraph…'s panel (`UI_SPEC.md` §39, CM3).
    #[cfg(feature = "page-surface")]
    paragraph_properties: Option<paragraph_properties::ParagraphProperties>,
    /// Page Layout Properties in the rail, while it shows (`UI_SPEC.md`
    /// §18). It and Style Properties never show together: opening one
    /// closes the other.
    #[cfg(feature = "page-surface")]
    page_layout_properties: Option<page_layout_properties::PageLayoutProperties>,
    /// Table Properties in the rail (`UI_SPEC.md` §32), while the caret is
    /// in a table.
    #[cfg(feature = "page-surface")]
    table_properties: Option<table_properties::TableProperties>,
    /// The Fields panel and Document Properties in the rail (`UI_SPEC.md`
    /// §28); one rail panel shows at a time.
    #[cfg(feature = "page-surface")]
    fields_panel: Option<fields_panel::FieldsPanel>,
    /// The Notes, Bookmarks, or Footnotes panel (`UI_SPEC.md` §34).
    #[cfg(feature = "page-surface")]
    references: Option<references_panel::ReferencePanel>,
    /// The Contents & Index panel (`UI_SPEC.md` §36).
    #[cfg(feature = "page-surface")]
    contents: Option<contents_panel::ContentsPanel>,
    /// The Master Document panel (`UI_SPEC.md` §37, MS1).
    #[cfg(feature = "page-surface")]
    master_panel: Option<master_panel::MasterPanel>,
    /// The Merge panel (`UI_SPEC.md` §38, MG1).
    #[cfg(feature = "page-surface")]
    merge_panel: Option<merge_panel::MergePanel>,
    /// While the Merge panel's preview shows a record: the document with
    /// its fields (MG5; display only).
    #[cfg(feature = "page-surface")]
    merge_preview: Option<merge_commands::MergePreview>,
    /// Merge, view & print: the record shown and what was printed (MG4).
    #[cfg(feature = "page-surface")]
    merge_viewing: Option<merge_commands::MergeViewing>,
    /// A merge data file's records as a table (MR1).
    #[cfg(feature = "page-surface")]
    data_form: Option<data_file::DataForm>,
    /// View & Print's records to print, while the print dialog is asked.
    #[cfg(feature = "page-surface")]
    viewing_job: Option<Vec<Document>>,
    /// The Envelope panel (`UI_SPEC.md` §38, MG8).
    #[cfg(feature = "page-surface")]
    envelope_panel: Option<envelope_panel::EnvelopePanel>,
    /// A master document's files and their numbers in the set (MS3).
    #[cfg(feature = "page-surface")]
    master_set: Option<master::MasterSet>,
    /// While this document is a file of an open master: the numbers it
    /// continues from in the set, over its own stored ones (MS3).
    #[cfg(feature = "page-surface")]
    live_sequence: Option<master_commands::LiveSequence>,
    /// A master in this member's folder that lists it (MS4), once looked
    /// for.
    #[cfg(feature = "page-surface")]
    master_offer: Option<Option<PathBuf>>,
    /// Treat as Single Document was chosen in this tab: an open master
    /// does not number it (Ami Pro's Yes, until the set is worked again).
    #[cfg(feature = "page-surface")]
    treated_single: bool,
    /// On a master: Print and Export PDF take this document only, not the
    /// whole set (MS6; the set is the default).
    #[cfg(feature = "page-surface")]
    print_this_only: bool,
    /// A set's TOC made in this tab: each row's file and heading (MS5).
    #[cfg(feature = "page-surface")]
    set_links: Vec<(PathBuf, amipro_editing::TextPosition)>,
    /// This tab is a master's TOC or index: saving it names it in the
    /// master's `[master]` (MS5).
    #[cfg(feature = "page-surface")]
    output_of: Option<(u64, master_commands::OutputKind)>,
    /// The Glossary panel (`UI_SPEC.md` §36, TX7).
    #[cfg(feature = "page-surface")]
    glossary: Option<glossary_panel::GlossaryPanel>,
    /// The Proofing panel (`UI_SPEC.md` §35).
    #[cfg(feature = "page-surface")]
    proofing: Option<proofing_panel::ProofingPanel>,
    /// Skip All and Ignore All: words skipped for the session in this
    /// document (SP4).
    #[cfg(feature = "page-surface")]
    spell_ignored: amipro_editing::proofing::spelling::Ignored,
    /// Spelling as you type (SP2): the words underlined, the document they
    /// were found in (or are being found in), the idle check's generation,
    /// and each paragraph text's problems, by the text's hash.
    #[cfg(feature = "page-surface")]
    spell_marks: page_surface::Highlights,
    #[cfg(feature = "page-surface")]
    grammar_marks: page_surface::Highlights,
    #[cfg(feature = "page-surface")]
    spell_root: Option<Document>,
    #[cfg(feature = "page-surface")]
    spell_generation: u64,
    #[cfg(feature = "page-surface")]
    spell_cache: std::collections::HashMap<u64, Vec<std::ops::Range<usize>>>,
    /// The underlined word the right button pressed, with its suggestions
    /// (and synonyms), for the right-click menu.
    #[cfg(feature = "page-surface")]
    context_word: Option<proofing_panel::ContextWord>,
    /// Each paragraph text's grammar problems, by the text's hash and the
    /// set.
    #[cfg(feature = "page-surface")]
    grammar_cache: std::collections::HashMap<(u64, String), Vec<grammar::GrammarProblem>>,
    #[cfg(feature = "page-surface")]
    document_properties: Option<document_properties::DocumentProperties>,
    /// Whether the last right press on the page was in a margin, where the
    /// right-click menu offers Page Layout… first.
    #[cfg(feature = "page-surface")]
    context_in_margin: bool,
    /// While a ruler marker is dragged: its page and position (page twips),
    /// for the guide line down the page (`UI_SPEC.md` §19).
    #[cfg(feature = "page-surface")]
    ruler_guide: Option<(usize, f32)>,
    /// Whether the status message is the ruler's hover text.
    #[cfg(feature = "page-surface")]
    ruler_status: bool,
    /// What the last right press on the ruler was on, for its menu.
    #[cfg(feature = "page-surface")]
    ruler_context: Option<(Option<ruler::Marker>, ruler_view::Part)>,
    /// The kind and leader of the tab a click on the ruler adds.
    #[cfg(feature = "page-surface")]
    ruler_tab: (amipro_document::TabKind, amipro_document::TabLeader),
    /// While the ruler has the keyboard (Edit › Go To Ruler): its line's
    /// position, page twips.
    #[cfg(feature = "page-surface")]
    ruler_cursor: Option<f32>,
    /// Insert › Floating Header/Footer… and Page Number…, while open.
    #[cfg(feature = "page-surface")]
    floating_dialog: Option<page_dialogs::FloatingDialog>,
    #[cfg(feature = "page-surface")]
    page_number_dialog: Option<page_dialogs::PageNumberDialog>,
    /// Insert › Frame…, while open (`UI_SPEC.md` §20).
    #[cfg(feature = "page-surface")]
    create_frame_dialog: Option<page_dialogs::CreateFrameDialog>,
    /// Insert › Table…'s dialog (`UI_SPEC.md` §32).
    #[cfg(feature = "page-surface")]
    insert_table_dialog: Option<table_commands::InsertTableDialog>,
    /// Insert › Bullet…, while open (`UI_SPEC.md` §21).
    #[cfg(feature = "page-surface")]
    bullet_dialog: Option<page_dialogs::BulletDialog>,
    /// Format › Styles ▸ Outline Numbering…, while open (`UI_SPEC.md` §22).
    #[cfg(feature = "page-surface")]
    outline_numbering: Option<page_dialogs::OutlineNumberingDialog>,
    /// Insert › Draw Frame, or the dialog's Draw on Page: the next press on
    /// a page draws a frame (`UI_SPEC.md` §20).
    #[cfg(feature = "page-surface")]
    drawing_frame: bool,
    /// Formatting chosen at the caret for the text typed next.
    #[cfg(feature = "page-surface")]
    typing: Option<page_editing::TypingFormat>,
    /// The document's pictures, decoded once (`UI_SPEC.md` §25).
    #[cfg(feature = "page-surface")]
    pictures: pictures::Pictures,
    /// Picture mode on this selected picture frame (P6).
    #[cfg(feature = "page-surface")]
    picture_mode: Option<amipro_document::FrameId>,
    /// Draw mode on a drawing frame (`UI_SPEC.md` §33).
    #[cfg(feature = "page-surface")]
    draw: Option<draw_mode::DrawMode>,
    /// Drawing Properties' typed values (`UI_SPEC.md` §33).
    #[cfg(feature = "page-surface")]
    draw_properties: drawing_properties::DrawingProperties,
    /// The Special Effects and Font dialogs, while open.
    #[cfg(feature = "page-surface")]
    special_effects: Option<character_dialogs::SpecialEffects>,
    #[cfg(feature = "page-surface")]
    font_dialog: Option<character_dialogs::FontDialog>,
    /// The open colour panel (`UI_SPEC.md` §23), and, while a drag in its
    /// picker goes on, the undo depth its first change left, so that the
    /// drag's changes are one undo step (C3).
    #[cfg(feature = "page-surface")]
    colour_panel: Option<colour_control::Panel>,
    #[cfg(feature = "page-surface")]
    colour_drag: Option<usize>,
    /// The Frame Properties inspector's typed values not yet applied, for
    /// `inspector_frame`, and the last value it refused with the reason.
    #[cfg(feature = "page-surface")]
    inspector_drafts: std::collections::BTreeMap<frame_inspector::Field, String>,
    #[cfg(feature = "page-surface")]
    inspector_frame: Option<amipro_document::FrameId>,
    #[cfg(feature = "page-surface")]
    inspector_error: Option<(frame_inspector::Field, String)>,
    #[cfg(feature = "page-surface")]
    zoom: Zoom,
    /// View › Facing Pages (CM8): pages in pairs; kept per tab.
    #[cfg(feature = "page-surface")]
    facing: bool,
    /// View › Draft Mode (CM9): the pages' width in twips while it is on.
    #[cfg(feature = "page-surface")]
    draft: Option<amipro_document::Twips>,
    /// View › Outline Mode (OM1): the pages' width in twips while it is on,
    /// and each main paragraph's icon from the last layout.
    #[cfg(feature = "page-surface")]
    outline: Option<amipro_document::Twips>,
    #[cfg(feature = "page-surface")]
    outline_icons: Vec<outline_view::OutlineIcon>,
    /// What Outline mode shows (OB.4): per tab, never written (OM2).
    #[cfg(feature = "page-surface")]
    outline_state: outline_view::OutlineState,
    /// Each main paragraph hidden in Outline mode, from the last layout.
    #[cfg(feature = "page-surface")]
    outline_hidden: Vec<bool>,
    /// The display copy Outline mode lays out and draws (its styles may be
    /// the outline styles).
    #[cfg(feature = "page-surface")]
    outline_copy: Option<Document>,
    /// The Outline navigator in the rail (OB.6), when it is open.
    #[cfg(feature = "page-surface")]
    outline_navigator: Option<outline_navigator::OutlineNavigator>,
    /// Tools › Charting… in the rail (DO.8), when it is open.
    #[cfg(feature = "page-surface")]
    chart_panel: Option<chart_panel::ChartPanel>,
    /// Tools › Equations… (DO.9): the equation being edited in place, with
    /// its template palette in the rail.
    #[cfg(feature = "page-surface")]
    equation_panel: Option<equation_panel::EquationPanel>,
    /// Chart frames last edited with Drawing: a double-click opens Draw
    /// mode on them, not the panel, until Charting changes them again.
    #[cfg(feature = "page-surface")]
    chart_drawing: std::collections::BTreeSet<amipro_document::FrameId>,
    /// The frame a Replace question waits to fill with a chart, while the
    /// clipboard is read.
    #[cfg(feature = "page-surface")]
    chart_replace: Option<amipro_document::FrameId>,
    /// Edit › Links… in the rail (DO.3), when it is open.
    #[cfg(feature = "page-surface")]
    links_panel: Option<links_panel::LinksPanel>,
    /// Help › Check for Updates… in the rail (P6.2), when it is open.
    #[cfg(feature = "page-surface")]
    updates_panel: Option<updates_panel::UpdatesPanel>,
    /// File › Recover Documents… in the rail (P6.3), when it is open.
    #[cfg(feature = "page-surface")]
    recovery_panel: Option<recovery_panel::RecoveryPanel>,
    /// Help in the rail (P6.4), when it is open.
    #[cfg(feature = "page-surface")]
    help_panel: Option<help_panel::HelpPanel>,
    /// Opened from a recovery copy and not saved since: the tab says so.
    #[cfg(feature = "page-surface")]
    recovered: bool,
    /// Edit › Paste Special… in the rail (DO.4), when it is open.
    #[cfg(feature = "page-surface")]
    paste_special: Option<paste_special::PasteSpecial>,
    /// The Sort panel in the rail (TD.8), and the settings of the last
    /// sort, which the next opening shows.
    #[cfg(feature = "page-surface")]
    sort_panel: Option<sort_panel::SortPanel>,
    #[cfg(feature = "page-surface")]
    sort_form: sort_panel::Form,
    /// Use Outline Styles, as the app's preference has it.
    #[cfg(feature = "page-surface")]
    outline_styles: bool,
    /// The zoom before View › Full Page, which choosing it again restores.
    #[cfg(feature = "page-surface")]
    zoom_before_full_page: Option<Zoom>,
    /// The visible part of the page workspace.
    #[cfg(feature = "page-surface")]
    viewport: cosmic::iced::Size,
    #[cfg(feature = "rich-editor-prototype")]
    prototype_selection: PrototypeSelection,
}

/// A new document: Ami Pro's Doc Info created now (`1058:0120`).
fn new_document() -> Document {
    let mut document = Document::default();
    *document.info_mut() = amipro_document::DocumentInfo::new(clock::ami_pro_now());
    document
}

impl DocumentSession {
    /// In Outline mode, a caret in a hidden paragraph goes to the nearest
    /// shown one, forward first when `forward` (its start), else back (its
    /// end), as Ami Pro's caret never stays in hidden text.
    #[cfg(feature = "page-surface")]
    fn leave_hidden_text(&mut self, forward: bool) {
        if self.outline.is_none() {
            return;
        }
        let caret = self.page_selection.focus;
        if caret.flow != self.document.main_flow_id()
            || !self
                .outline_hidden
                .get(caret.paragraph)
                .copied()
                .unwrap_or(false)
        {
            return;
        }
        let Some(paragraph) =
            outline_view::shown_near(&self.outline_hidden, caret.paragraph, forward)
        else {
            return;
        };
        let byte = if paragraph > caret.paragraph {
            0
        } else {
            self.document.paragraphs()[paragraph].text.len()
        };
        let at = amipro_editing::TextPosition {
            flow: caret.flow,
            paragraph,
            byte,
        };
        let page = self.page_selection.page;
        self.page_selection = PageSelection::caret(at, page);
        self.sync_editor_to_page();
    }
    /// Where the user is now, as an undo step keeps it.
    fn history_caret(&self) -> HistoryCaret {
        HistoryCaret {
            cursor: self.editor.cursor(),
            #[cfg(feature = "page-surface")]
            page: self.page_selection,
            #[cfg(feature = "page-surface")]
            frames: self.frame_selection.clone(),
        }
    }

    fn new(id: u64, document: Document, location: DocumentLocation, status: String) -> Self {
        let clean_document = document.clone();
        let editor = text_editor::Content::with_text(&document.plain_text());
        let pages = page_layout::SessionPages::default();
        let mut words = chrome::WordCount::default();
        words.refresh(&document);
        #[cfg(feature = "page-surface")]
        let page_selection = PageSelection::start(&document);
        let mut session = Self {
            id,
            rich_layout_generation: 0,
            password: None,
            document,
            editor,
            location,
            status,
            history: History::new(),
            overwrite_confirmation: OverwriteConfirmation::default(),
            sam_save: None,
            save_problem: None,
            dirty: false,
            clean_document,
            opened_at: std::time::Instant::now(),
            saved_at: None,
            scroll_offset: cosmic::iced::widget::scrollable::AbsoluteOffset::default(),
            #[cfg(feature = "page-surface")]
            page_selection,
            #[cfg(feature = "page-surface")]
            frame_selection: None,
            #[cfg(feature = "page-surface")]
            frame_keys: None,
            #[cfg(feature = "page-surface")]
            typing_run: None,
            #[cfg(feature = "page-surface")]
            number_pending: None,
            #[cfg(feature = "page-surface")]
            pending_frame_delete: false,
            #[cfg(feature = "page-surface")]
            go_to: None,
            #[cfg(feature = "page-surface")]
            find: None,
            #[cfg(feature = "page-surface")]
            field_shading: page_surface::Highlights::new(),
            #[cfg(feature = "page-surface")]
            fields: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format: None,
            #[cfg(feature = "page-surface")]
            field_prompt: None,
            #[cfg(feature = "page-surface")]
            print_options: print::PrintOptions::default(),
            #[cfg(feature = "page-surface")]
            print_panel: None,
            #[cfg(feature = "page-surface")]
            export_pending: None,
            #[cfg(feature = "page-surface")]
            exported: None,
            #[cfg(feature = "page-surface")]
            import_folder: None,
            #[cfg(feature = "page-surface")]
            export_folder: None,
            save_sheet: None,
            save_sheet_contents: true,
            create_style: None,
            style_management: None,
            use_sheet: None,
            rename_style: None,
            #[cfg(feature = "page-surface")]
            style_properties: None,
            #[cfg(feature = "page-surface")]
            paragraph_properties: None,
            #[cfg(feature = "page-surface")]
            page_layout_properties: None,
            #[cfg(feature = "page-surface")]
            table_properties: None,
            #[cfg(feature = "page-surface")]
            fields_panel: None,
            #[cfg(feature = "page-surface")]
            references: None,
            #[cfg(feature = "page-surface")]
            contents: None,
            #[cfg(feature = "page-surface")]
            master_panel: None,
            #[cfg(feature = "page-surface")]
            merge_panel: None,
            #[cfg(feature = "page-surface")]
            merge_preview: None,
            #[cfg(feature = "page-surface")]
            merge_viewing: None,
            #[cfg(feature = "page-surface")]
            data_form: None,
            #[cfg(feature = "page-surface")]
            viewing_job: None,
            #[cfg(feature = "page-surface")]
            envelope_panel: None,
            #[cfg(feature = "page-surface")]
            master_set: None,
            #[cfg(feature = "page-surface")]
            live_sequence: None,
            #[cfg(feature = "page-surface")]
            master_offer: None,
            #[cfg(feature = "page-surface")]
            treated_single: false,
            #[cfg(feature = "page-surface")]
            print_this_only: false,
            #[cfg(feature = "page-surface")]
            set_links: Vec::new(),
            #[cfg(feature = "page-surface")]
            output_of: None,
            #[cfg(feature = "page-surface")]
            glossary: None,
            #[cfg(feature = "page-surface")]
            proofing: None,
            #[cfg(feature = "page-surface")]
            spell_ignored: Default::default(),
            #[cfg(feature = "page-surface")]
            spell_marks: Default::default(),
            #[cfg(feature = "page-surface")]
            grammar_marks: Default::default(),
            #[cfg(feature = "page-surface")]
            spell_root: None,
            #[cfg(feature = "page-surface")]
            spell_generation: 0,
            #[cfg(feature = "page-surface")]
            spell_cache: Default::default(),
            #[cfg(feature = "page-surface")]
            context_word: None,
            #[cfg(feature = "page-surface")]
            grammar_cache: Default::default(),
            #[cfg(feature = "page-surface")]
            document_properties: None,
            #[cfg(feature = "page-surface")]
            context_in_margin: false,
            #[cfg(feature = "page-surface")]
            ruler_guide: None,
            #[cfg(feature = "page-surface")]
            ruler_status: false,
            #[cfg(feature = "page-surface")]
            ruler_context: None,
            #[cfg(feature = "page-surface")]
            ruler_tab: (
                amipro_document::TabKind::Left,
                amipro_document::TabLeader::None,
            ),
            #[cfg(feature = "page-surface")]
            ruler_cursor: None,
            import_report: None,
            #[cfg(feature = "page-surface")]
            import_report_shown: false,
            #[cfg(feature = "page-surface")]
            floating_dialog: None,
            #[cfg(feature = "page-surface")]
            page_number_dialog: None,
            #[cfg(feature = "page-surface")]
            create_frame_dialog: None,
            #[cfg(feature = "page-surface")]
            insert_table_dialog: None,
            #[cfg(feature = "page-surface")]
            bullet_dialog: None,
            #[cfg(feature = "page-surface")]
            outline_numbering: None,
            #[cfg(feature = "page-surface")]
            drawing_frame: false,
            #[cfg(feature = "page-surface")]
            typing: None,
            #[cfg(feature = "page-surface")]
            pictures: pictures::Pictures::default(),
            #[cfg(feature = "page-surface")]
            picture_mode: None,
            #[cfg(feature = "page-surface")]
            draw: None,
            #[cfg(feature = "page-surface")]
            draw_properties: drawing_properties::DrawingProperties::default(),
            #[cfg(feature = "page-surface")]
            special_effects: None,
            #[cfg(feature = "page-surface")]
            font_dialog: None,
            #[cfg(feature = "page-surface")]
            colour_panel: None,
            #[cfg(feature = "page-surface")]
            colour_drag: None,
            #[cfg(feature = "page-surface")]
            inspector_drafts: std::collections::BTreeMap::new(),
            #[cfg(feature = "page-surface")]
            inspector_frame: None,
            #[cfg(feature = "page-surface")]
            inspector_error: None,
            #[cfg(feature = "page-surface")]
            zoom: Zoom::default(),
            #[cfg(feature = "page-surface")]
            facing: false,
            #[cfg(feature = "page-surface")]
            draft: None,
            #[cfg(feature = "page-surface")]
            outline: None,
            #[cfg(feature = "page-surface")]
            outline_icons: Vec::new(),
            #[cfg(feature = "page-surface")]
            outline_state: outline_view::OutlineState::default(),
            #[cfg(feature = "page-surface")]
            outline_hidden: Vec::new(),
            #[cfg(feature = "page-surface")]
            outline_copy: None,
            #[cfg(feature = "page-surface")]
            outline_navigator: None,
            #[cfg(feature = "page-surface")]
            chart_panel: None,
            #[cfg(feature = "page-surface")]
            equation_panel: None,
            #[cfg(feature = "page-surface")]
            chart_drawing: std::collections::BTreeSet::new(),
            #[cfg(feature = "page-surface")]
            chart_replace: None,
            #[cfg(feature = "page-surface")]
            links_panel: None,
            #[cfg(feature = "page-surface")]
            updates_panel: None,
            #[cfg(feature = "page-surface")]
            recovery_panel: None,
            #[cfg(feature = "page-surface")]
            help_panel: None,
            #[cfg(feature = "page-surface")]
            recovered: false,
            #[cfg(feature = "page-surface")]
            paste_special: None,
            #[cfg(feature = "page-surface")]
            sort_panel: None,
            #[cfg(feature = "page-surface")]
            sort_form: sort_panel::Form::default(),
            #[cfg(feature = "page-surface")]
            outline_styles: false,
            #[cfg(feature = "page-surface")]
            zoom_before_full_page: None,
            #[cfg(feature = "page-surface")]
            viewport: cosmic::iced::Size::ZERO,
            pages,
            words,
            #[cfg(feature = "rich-editor-prototype")]
            prototype_selection: PrototypeSelection::caret(0),
        };
        session.refresh_rich_layout();
        session.rich_layout_generation = 0;
        session
    }

    fn display_name(&self) -> String {
        let name = self.location_name();
        #[cfg(feature = "page-surface")]
        if self.recovered {
            return format!("{name} (recovered)");
        }
        name
    }

    fn location_name(&self) -> String {
        match &self.location {
            DocumentLocation::Untitled => self.document.title.clone(),
            DocumentLocation::LegacySource(path)
            | DocumentLocation::Sam(path)
            | DocumentLocation::Native(path) => path
                .file_name()
                .and_then(|name| name.to_str())
                .map_or_else(|| self.document.title.clone(), str::to_owned),
        }
    }

    fn tab_label(&self) -> String {
        let name = self.display_name();
        if self.dirty {
            format!("{name} •")
        } else {
            name
        }
    }

    /// Turns the default editor's text change into one command on the main
    /// flow, executed through the history. Where the command's rules differ
    /// from what the editor did (a pasted U+FFFC becomes U+FFFD; deleting a
    /// placeholder deletes what its object owns), the editor is brought back
    /// to the model.
    ///
    /// `single_delete` marks a Backspace or Delete without a selection. When
    /// it removed exactly one object placeholder, the edit becomes the
    /// single-character delete command, so a frame anchor stays (V05).
    fn reconcile_standard_edit(&mut self, cursor_before: text_editor::Cursor, single_delete: bool) {
        let mut text = self.editor.text();
        if text.contains('\r') {
            text = text.replace("\r\n", "\n").replace('\r', "\n");
        }
        let caret = line_text_offset(&text, self.editor.cursor().position);
        let Some(mut command) = amipro_editing::plain_text_edit(&self.document, &text, caret)
        else {
            return;
        };
        // Only text holding U+FFFC, a single delete of a placeholder, or a
        // rejected edit can leave the model different from what the editor
        // shows.
        let mut may_differ = matches!(
            &command,
            amipro_editing::Command::ReplaceText { text, .. }
                if text.contains(amipro_document::OBJECT_REPLACEMENT_CHARACTER)
        );
        if single_delete
            && let amipro_editing::Command::ReplaceText { start, end, text } = &command
            && text.is_empty()
            && start.paragraph == end.paragraph
            && self.document.paragraphs()[start.paragraph]
                .text
                .get(start.byte..end.byte)
                == Some(amipro_document::OBJECT_REPLACEMENT_CHARACTER.encode_utf8(&mut [0; 3]))
        {
            command = amipro_editing::Command::DeleteCharacter {
                at: *start,
                forward: true,
            };
            may_differ = true;
        }
        let mut caret = self.history_caret();
        caret.cursor = cursor_before;
        match self.history.execute(&mut self.document, caret, &command) {
            Ok(executed) if !executed.report.is_empty() => {
                self.refresh_rich_layout();
                self.status = "Modified".into();
                self.dirty = true;
            }
            Ok(_) => {}
            Err(error) => {
                self.status = format!("Edit failed: {error}");
                may_differ = true;
            }
        }
        let model_text = if may_differ {
            self.document.plain_text()
        } else {
            return;
        };
        if model_text != text {
            let cursor = self.editor.cursor();
            synchronize_editor_plain_text(&mut self.editor, &text, &model_text);
            if line_text_offset(&model_text, cursor.position).is_some() {
                self.editor.move_to(text_editor::Cursor {
                    position: cursor.position,
                    selection: None,
                });
            }
        }
    }

    /// Whether a drag in a colour panel's picker goes on and its first
    /// change is the last undo step, so the next change joins it (C3).
    fn continues_colour_drag(&self) -> bool {
        #[cfg(feature = "page-surface")]
        return self.colour_drag == Some(self.history.undo_len());
        #[cfg(not(feature = "page-surface"))]
        false
    }

    /// Runs a command that changes the document but not its text (styles,
    /// the style sheet) as one undo step; returns whether it changed it.
    fn execute_document_command(&mut self, command: &amipro_editing::Command) -> bool {
        #[cfg(feature = "page-surface")]
        if self.preview_refuses_edit() {
            return false;
        }
        let cursor = self.history_caret();
        let result = if self.continues_colour_drag() {
            self.history.amend(&mut self.document, cursor, command)
        } else {
            self.history.execute(&mut self.document, cursor, command)
        };
        match result {
            Ok(executed) if !executed.report.is_empty() => {
                self.refresh_rich_layout();
                self.dirty = true;
                true
            }
            Ok(_) => false,
            Err(error) => {
                self.status = format!("Edit failed: {error}");
                false
            }
        }
    }

    /// Writes the document's styles and page layout to a new style sheet at
    /// `selected` (decision D3: never over an existing file), then links
    /// the document to it, or detaches an untitled one, as Ami Pro does
    /// (VM fixtures SD1, SD2, SS2).
    fn save_style_sheet(
        &mut self,
        selected: &std::path::Path,
        description: &str,
        contents: bool,
    ) -> bool {
        let saved_at = clock::ami_pro_now();
        let written = style_sheet_destination(selected).and_then(|path| {
            let sheet = amipro_formats::style_sheet(
                &self.document,
                &amipro_formats::SheetOptions {
                    contents,
                    description: description.to_owned(),
                    saved_at,
                },
            );
            write_new_file(&path, &sheet).map(|()| path)
        });
        match written {
            Ok(path) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
                let link = match self.location {
                    DocumentLocation::Untitled => None,
                    _ => name,
                };
                self.execute_document_command(&amipro_editing::Command::SavedAsStyleSheet {
                    name: link,
                });
                self.status = format!("Saved style sheet {}", path.display());
                true
            }
            Err(error) => {
                self.status = format!("Save as a Style Sheet failed: {error}");
                self.save_problem = Some(SaveProblem::style_sheet(&error, description, contents));
                false
            }
        }
    }

    /// Records the current root and cursor as the state before an edit.
    fn push_undo(&mut self) {
        let cursor = self.history_caret();
        self.history.record(&mut self.document, cursor);
    }

    /// Marks derived rich layout stale after a semantic change and lays out
    /// the pages again (incrementally). The feature-gated surface reshapes
    /// only changed paragraphs on its next layout; the default editor derives
    /// its display from `Document` in `view`.
    fn refresh_rich_layout(&mut self) {
        self.rich_layout_generation = self
            .rich_layout_generation
            .checked_add(1)
            .expect("rich layout generation overflowed");
        self.pages.set_field_context(self.field_context());
        #[cfg(feature = "page-surface")]
        self.refresh_field_views();
        #[cfg(feature = "page-surface")]
        if let Some(width) = self.outline {
            self.outline_state.retain(&self.document);
            let hidden = self.outline_state.hidden_flags(&self.document);
            let outline =
                outline_view::outline_document(&self.document, width, &hidden, self.outline_styles);
            self.pages.update(&outline);
            self.outline_icons = outline_view::icons(&self.document, &hidden);
            self.outline_hidden = hidden;
            self.outline_copy = Some(outline);
            return;
        }
        #[cfg(feature = "page-surface")]
        if let Some(width) = self.draft {
            let draft = view_modes::draft_document(&self.document, width);
            self.pages.update(&draft);
            return;
        }
        self.pages.update(&self.document);
    }

    /// Draft's page width for this tab's window and zoom.
    #[cfg(feature = "page-surface")]
    fn draft_width(&self) -> amipro_document::Twips {
        let percent = match self.zoom {
            Zoom::Percent(percent) => f32::from(percent),
            _ => 100.0,
        };
        let pixels =
            (self.shown_viewport().width - 2.0 * page_view::PADDING - PAGE_SCROLLBAR).max(1.0);
        (pixels / (page_view::ACTUAL_SIZE * percent / 100.0)) as amipro_document::Twips
    }

    /// Writes a prepared export and says so; the status bar then offers to
    /// open the file (EX10).
    #[cfg(feature = "page-surface")]
    fn finish_export(&mut self, prepared: &export::Prepared) {
        match export::write(prepared) {
            Ok(exported) => {
                self.status = exported.message();
                self.exported = Some(exported.path);
            }
            Err(error) => self.status = format!("Export failed: {error}"),
        }
    }

    /// What a folder's import reports, in its panel and the status bar.
    #[cfg(feature = "page-surface")]
    fn import_folder_progress(&mut self, progress: import_folder::Progress) {
        let Some(panel) = self.import_folder.as_mut() else {
            return;
        };
        match progress {
            import_folder::Progress::Started { total } => {
                if let Some(running) = panel.running.as_mut() {
                    running.total = total;
                }
            }
            import_folder::Progress::Done { done } => {
                if let Some(running) = panel.running.as_mut() {
                    running.done = done;
                }
            }
            import_folder::Progress::Finished(result) => {
                panel.running = None;
                match result {
                    Ok(finished) => {
                        self.status = finished.message();
                        panel.finished = Some(finished);
                    }
                    Err(error) => self.status = format!("Import Folder failed: {error}"),
                }
            }
        }
    }

    /// What a folder's export reports, in its panel and the status bar.
    #[cfg(feature = "page-surface")]
    fn export_folder_progress(&mut self, progress: export_folder::Progress) {
        let Some(panel) = self.export_folder.as_mut() else {
            return;
        };
        match progress {
            export_folder::Progress::Started { total } => {
                if let Some(running) = panel.running.as_mut() {
                    running.total = total;
                }
            }
            export_folder::Progress::Done { done } => {
                if let Some(running) = panel.running.as_mut() {
                    running.done = done;
                }
            }
            export_folder::Progress::Finished(result) => {
                panel.running = None;
                match result {
                    Ok(finished) => {
                        self.status = finished.message();
                        panel.finished = Some(finished);
                    }
                    Err(error) => self.status = format!("Export Folder failed: {error}"),
                }
            }
        }
    }

    /// What the document's dates and Doc Info entries show now: the clock to
    /// the minute, the file, this session's save and minutes, and the page
    /// count of the last layout.
    fn field_context(&self) -> amipro_document::FieldContext {
        let path = match &self.location {
            DocumentLocation::Untitled => None,
            DocumentLocation::LegacySource(path)
            | DocumentLocation::Sam(path)
            | DocumentLocation::Native(path) => Some(path),
        };
        amipro_document::FieldContext {
            now: amipro_document::WallClock {
                second: 0,
                ..clock::local_now()
            },
            file_name: path
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            directory: path
                .and_then(|path| path.parent())
                .map(|folder| folder.display().to_string())
                .unwrap_or_default(),
            saved: self.saved_at,
            session_minutes: clock::editing_minutes(self.opened_at.elapsed()),
            pages: u32::try_from(self.pages.page_count()).unwrap_or(u32::MAX),
            ..amipro_document::FieldContext::default()
        }
    }

    /// The caret's page and the page count, for the status bar.
    fn page_position(&self) -> (u32, usize) {
        let position = self.editor.cursor().position;
        (
            self.pages.page_of(position.line, position.column),
            self.pages.page_count(),
        )
    }
}

#[cfg(feature = "page-surface")]
impl DocumentSession {
    /// The document's own pages, for print and PDF: the shown layout in
    /// Layout mode; in Draft or Outline mode, which lay out a display copy,
    /// the document laid out afresh (found by the pre-handoff check of run
    /// B: those modes printed their display pages).
    fn output_layout(&self) -> amipro_layout::DocumentLayout {
        if self.draft.is_none() && self.outline.is_none() {
            return self.pages.layout().clone();
        }
        let mut pages = page_layout::SessionPages::default();
        pages.set_field_context(self.field_context());
        pages.update(&self.document);
        pages.layout().clone()
    }

    /// The page transform for the last known workspace size.
    fn page_view(&self) -> PageView {
        PageView::arranged(
            self.pages.layout(),
            self.zoom,
            self.shown_viewport(),
            self.facing,
        )
    }

    /// The workspace's visible size (before the first event, a typical
    /// one).
    fn shown_viewport(&self) -> cosmic::iced::Size {
        if self.viewport.width > 0.0 {
            self.viewport
        } else {
            cosmic::iced::Size::new(1_000.0, 800.0)
        }
    }

    /// The page at the workspace's centre, or the nearest one: Ami Pro's
    /// current page, where Create Frame puts a frame (FC9: the page in
    /// view, not the caret's).
    #[cfg(feature = "page-surface")]
    fn page_in_view(&self) -> u32 {
        let view = self.page_view();
        let viewport = self.shown_viewport();
        let x = self.scroll_offset.x + viewport.width / 2.0;
        let y = self.scroll_offset.y + viewport.height / 2.0;
        let distance = |rect: &cosmic::iced::Rectangle| {
            let dx = (rect.x - x).max(x - (rect.x + rect.width)).max(0.0);
            let dy = (rect.y - y).max(y - (rect.y + rect.height)).max(0.0);
            dx * dx + dy * dy
        };
        view.pages
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
            .map_or(1, |(index, _)| index as u32 + 1)
    }

    /// Page `page`'s paper in twips.
    #[cfg(feature = "page-surface")]
    fn paper_of(&self, page: u32) -> (amipro_document::Twips, amipro_document::Twips) {
        self.pages
            .layout()
            .pages
            .get(page.saturating_sub(1) as usize)
            .map_or((12_240, 15_840), |page| {
                (page.width.round() as i32, page.height.round() as i32)
            })
    }

    /// Runs a text command from the page surface through history.
    /// Runs `command` as one undo step; returns whether it changed the
    /// document.
    fn execute_page_command(&mut self, command: &amipro_editing::Command) -> bool {
        self.run_page_command(command, false)
    }

    /// Runs `command` as a new undo step, or with `join` as part of the
    /// last one (a key of a run of typing); returns whether it changed the
    /// document.
    fn run_page_command(&mut self, command: &amipro_editing::Command, join: bool) -> bool {
        #[cfg(feature = "page-surface")]
        if self.preview_refuses_edit() {
            return false;
        }
        let previous = self.document.clone();
        let cursor = self.history_caret();
        let result = if join {
            self.history.amend(&mut self.document, cursor, command)
        } else {
            self.history.execute(&mut self.document, cursor, command)
        };
        match result {
            Ok(executed) if !executed.report.is_empty() => {
                let caret = executed
                    .caret
                    .unwrap_or_else(|| self.page_selection.ordered().0);
                self.page_selection = page_editing::valid_selection(
                    &self.document,
                    PageSelection::caret(caret, self.page_selection.page),
                );
                self.refresh_rich_layout();
                synchronize_editor_text(&mut self.editor, &previous, &self.document);
                self.sync_editor_to_page();
                self.status = "Modified".into();
                self.dirty = true;
                true
            }
            Ok(_) => false,
            Err(error) => {
                self.status = format!("Edit failed: {error}");
                false
            }
        }
    }

    /// Runs `commands` as one undo step, from the first that changes the
    /// document.
    #[cfg(feature = "page-surface")]
    fn execute_page_commands(&mut self, commands: &[amipro_editing::Command]) {
        let mut started = false;
        for command in commands {
            if started {
                self.amend_page_command(command);
            } else {
                started = self.execute_page_command(command);
            }
        }
    }

    /// Applies `command` as part of the last undo step (the formatting of
    /// text just typed).
    #[cfg(feature = "page-surface")]
    fn amend_page_command(&mut self, command: &amipro_editing::Command) {
        let previous = self.document.clone();
        let cursor = self.history_caret();
        match self.history.amend(&mut self.document, cursor, command) {
            Ok(executed) if !executed.report.is_empty() => {
                self.page_selection =
                    page_editing::valid_selection(&self.document, self.page_selection);
                self.refresh_rich_layout();
                synchronize_editor_text(&mut self.editor, &previous, &self.document);
                self.sync_editor_to_page();
            }
            Ok(_) => {}
            Err(error) => self.status = format!("Edit failed: {error}"),
        }
    }

    /// The formatting chosen for typing at the caret, while the caret stays
    /// where it was chosen and nothing else changed the document.
    #[cfg(feature = "page-surface")]
    fn typing_format(&self) -> Option<&page_editing::TypingFormat> {
        self.typing.as_ref().filter(|typing| {
            self.frame_selection.is_none()
                && self.page_selection.is_caret()
                && self.page_selection.focus == typing.at
                && self.history.undo_len() == typing.depth
        })
    }

    /// Whether a key of `kind` joins the run of typing before it (SU4): the
    /// same kind of key, the caret where the run left it, nothing else in
    /// the history since, no formatting chosen at the caret, no pause.
    #[cfg(feature = "page-surface")]
    fn continues_typing_run(&self, kind: TypingKind) -> bool {
        self.typing_run.is_some_and(|run| {
            run.kind == kind
                && self.frame_selection.is_none()
                && self.page_selection.is_caret()
                && self.page_selection.focus == run.at
                && self.history.changes() == run.changes
                && self.typing_format().is_none()
                && run.last.elapsed() < TYPING_PAUSE
        })
    }

    /// Writes the number typed in a cell the way Ami Pro stores it (TD.5,
    /// VM TDT1–TDT4): `%.*f` at the style's decimals, with the right
    /// alignment code, the stored text then standing for the number the
    /// style shows. Text (a comma, `$`, `%`, a space) and formula cells stay
    /// as typed. It joins the typing run it follows when nothing came
    /// between, else it is a step of its own; the caret stays where it is.
    #[cfg(feature = "page-surface")]
    fn rewrite_number(&mut self, flow: amipro_document::FlowId, joins: bool) {
        use amipro_document::CellContent;
        // A merge data file's values are text for the merge (a zip code
        // keeps its leading zero): TNK-TD5c.
        if self.data_form.is_some() {
            return;
        }
        let Some((frame, row, column)) = amipro_editing::tables::cell_of_flow(&self.document, flow)
        else {
            return;
        };
        let Some(amipro_document::FrameContent::Table(table)) =
            self.document.frame(frame).map(|frame| &frame.content)
        else {
            return;
        };
        let Some(cell) = table.cells.get(row * table.columns.len() + column) else {
            return;
        };
        if cell.formula.is_some()
            || cell.content != CellContent::Text(flow)
            || cell.format.protected
        {
            return;
        }
        let Some(content) = self.document.flow(flow) else {
            return;
        };
        let [paragraph] = content.paragraphs.as_slice() else {
            return;
        };
        if !paragraph.objects.is_empty() || !paragraph.marks.is_empty() {
            return;
        }
        let Some(style) = self.document.styles().get(&paragraph.style) else {
            return;
        };
        let format = &style.kept.number_format;
        let Some(value) = format.scan(&paragraph.text) else {
            return;
        };
        let stored = format.stored_text(value);
        let mut commands = Vec::new();
        if stored != paragraph.text {
            let at = |byte| amipro_editing::TextPosition {
                flow,
                paragraph: 0,
                byte,
            };
            commands.push(amipro_editing::Command::ReplaceText {
                start: at(0),
                end: at(paragraph.text.len()),
                text: stored,
            });
        }
        if paragraph.format.alignment.is_none() {
            commands.push(amipro_editing::Command::SetParagraphAlignment {
                flow,
                first: 0,
                last: 0,
                alignment: Some(amipro_document::Alignment::Right),
            });
        }
        if commands.is_empty() {
            return;
        }
        let selection = self.page_selection;
        let mut started = joins;
        for command in &commands {
            if started {
                self.amend_page_command(command);
            } else {
                started = self.execute_page_command(command);
            }
        }
        self.typing_run = None;
        self.page_selection = page_editing::valid_selection(&self.document, selection);
        self.sync_editor_to_page();
    }

    /// After a message: a cell the caret edited and has left gets its
    /// number rewritten ([`DocumentSession::rewrite_number`]); a cell the
    /// message edited becomes the one to rewrite when the caret leaves.
    #[cfg(feature = "page-surface")]
    fn settle_numbers(
        &mut self,
        (id, run, undo, before, text): (
            u64,
            Option<u64>,
            usize,
            amipro_document::FlowId,
            Option<String>,
        ),
    ) {
        let session = self;
        let selection = session.page_selection;
        let in_flow = selection.focus.flow;
        let changed = session.id == id
            && in_flow == before
            && session
                .document
                .flow(in_flow)
                .and_then(|content| content.paragraphs.first().map(|first| &first.text))
                != text.as_ref();
        let inside = selection.anchor.flow == in_flow && session.frame_selection.is_none();
        if let Some(pending) = session.number_pending
            && (!inside || in_flow != pending)
        {
            session.number_pending = None;
            // It joins the typing run it follows, or the one step the
            // message itself made (Tab adding a row).
            let joins =
                run == Some(session.history.changes()) || session.history.undo_len() == undo + 1;
            session.rewrite_number(pending, joins);
            // The formulas that read the cell follow, in the same step; the
            // number rewrite came first, since its text is what they read
            // (TD.7, TN5).
            let joins = joins || session.history.undo_len() > undo;
            session.recalculate_cell(pending, joins);
        }
        if changed
            && inside
            && amipro_editing::tables::cell_of_flow(&session.document, in_flow).is_some()
        {
            session.number_pending = Some(in_flow);
        }
    }

    /// Ages the run of typing past the pause, as if the user had waited.
    #[cfg(all(test, feature = "page-surface"))]
    fn age_typing_run(&mut self) {
        if let Some(run) = self.typing_run.as_mut() {
            run.last = std::time::Instant::now()
                .checked_sub(TYPING_PAUSE)
                .unwrap_or(run.last);
        }
    }

    /// The typing format at the caret, started afresh if the old one lapsed.
    #[cfg(feature = "page-surface")]
    fn typing_format_mut(&mut self) -> &mut page_editing::TypingFormat {
        if self.typing_format().is_none() {
            self.typing = Some(page_editing::TypingFormat::new(
                self.page_selection.focus,
                self.history.undo_len(),
            ));
        }
        self.typing.as_mut().expect("just set")
    }

    /// Whether `attribute` is on over the page selection, or at the caret
    /// (the typing format's choice, else either neighbouring character's).
    #[cfg(feature = "page-surface")]
    fn page_attribute_state(&self, attribute: CharacterAttribute) -> ToggleState {
        if self.frame_selection.is_some() {
            return ToggleState::Unavailable;
        }
        let selection = self.page_selection;
        if selection.is_caret() {
            let on = self
                .typing_format()
                .and_then(|typing| typing.attribute(attribute))
                .unwrap_or_else(|| {
                    page_editing::caret_attribute(&self.document, selection.focus, attribute)
                });
            return if on {
                ToggleState::On
            } else {
                ToggleState::Off
            };
        }
        let (start, end) = selection.ordered();
        let Some(flow) = self.document.flow(start.flow) else {
            return ToggleState::Unavailable;
        };
        range_character_state(
            &self.document,
            &flow.paragraphs,
            (start.paragraph, start.byte),
            (end.paragraph, end.byte),
            attribute,
        )
    }

    /// The face, size, and colour the Font dialog starts with: the typing
    /// choice at a caret, else the text's at the selection's start (at a
    /// caret, the character before it, or after it at a paragraph start;
    /// in an empty paragraph, its end format).
    #[cfg(feature = "page-surface")]
    fn selection_font(&self) -> (String, f32, amipro_document::Colour) {
        let (format, style) = self.selection_format();
        let mut font = (
            format
                .font_family
                .clone()
                .unwrap_or_else(|| style.font_family.clone()),
            format.font_size_points.unwrap_or(style.font_size_points),
            format.attributes(&style).colour,
        );
        if let Some(choice) = self.typing_format().and_then(|typing| typing.font.as_ref()) {
            font = (
                choice
                    .family
                    .clone()
                    .unwrap_or_else(|| style.font_family.clone()),
                choice.size.unwrap_or(style.font_size_points),
                choice.colour.unwrap_or(style.attributes.colour),
            );
        }
        font
    }

    /// The overstrike character the Special Effects dialog starts with: the
    /// typing choice at a caret, else the text's (as `selection_font`).
    #[cfg(feature = "page-surface")]
    fn selection_overstrike(&self) -> Option<char> {
        let chosen = self.typing_format().and_then(|typing| {
            typing
                .attributes
                .iter()
                .rev()
                .find_map(|(attribute, on)| match attribute {
                    CharacterAttribute::Overstrike(character) => Some(on.then_some(*character)),
                    _ => None,
                })
        });
        chosen.unwrap_or_else(|| {
            let (format, style) = self.selection_format();
            format.attributes(&style).overstrike
        })
    }

    /// The direct format at the selection's start and its paragraph's
    /// style: at a caret, the character before it, or after it at a
    /// paragraph start; in an empty paragraph, its end format.
    #[cfg(feature = "page-surface")]
    fn selection_format(&self) -> (CharacterFormat, amipro_document::ParagraphStyle) {
        let (start, _) = self.page_selection.ordered();
        let paragraph = self
            .document
            .flow(start.flow)
            .and_then(|flow| flow.paragraph(start.paragraph));
        let style = paragraph
            .and_then(|paragraph| self.document.styles().get(&paragraph.style))
            .or_else(|| self.document.styles().values().next())
            .expect("a document has a style");
        let byte = paragraph.and_then(|paragraph| {
            let text = &paragraph.text;
            let at = start.byte.min(text.len());
            if self.page_selection.is_caret() {
                text[..at]
                    .chars()
                    .next_back()
                    .map(|character| at - character.len_utf8())
                    .or((at < text.len()).then_some(at))
            } else {
                Some(at)
            }
        });
        // In an empty paragraph, its end format: what typing there takes.
        let plain = CharacterFormat::default();
        let format = match (paragraph, byte) {
            (Some(paragraph), Some(byte)) => paragraph.format_at(byte),
            (Some(paragraph), None) => &paragraph.end_format,
            (None, _) => &plain,
        };
        (format.clone(), style.clone())
    }

    /// Switches attributes over the page selection as one undo step, or at
    /// a caret for the text typed next.
    #[cfg(feature = "page-surface")]
    fn apply_attribute_changes(&mut self, changes: &[(CharacterAttribute, bool)]) {
        if changes.is_empty() || self.frame_selection.is_some() {
            return;
        }
        let selection = self.page_selection;
        if selection.is_caret() {
            let typing = self.typing_format_mut();
            for (attribute, on) in changes {
                typing.set(*attribute, *on);
            }
            self.status = "Special effects for typing".into();
            return;
        }
        let (start, end) = selection.ordered();
        let commands: Vec<_> = changes
            .iter()
            .map(
                |(attribute, on)| amipro_editing::Command::SetCharacterAttribute {
                    start,
                    end,
                    attribute: *attribute,
                    on: *on,
                },
            )
            .collect();
        self.execute_page_commands(&commands);
        self.page_selection = page_editing::valid_selection(&self.document, selection);
        self.sync_editor_to_page();
        self.status = "Special effects applied".into();
    }

    /// Sets the font over the page selection, or at a caret for the text
    /// typed next.
    #[cfg(feature = "page-surface")]
    /// The status bar's Face or Point Size: only the part chosen changes, on
    /// a selection or in the typing format at a caret.
    fn apply_font_part(&mut self, choice: page_editing::FontChoice) {
        if self.frame_selection.is_some() {
            return;
        }
        let selection = self.page_selection;
        if selection.is_caret() {
            let font = &mut self.typing_format_mut().font;
            let held = font.get_or_insert_with(page_editing::FontChoice::default);
            if choice.family.is_some() {
                held.family = choice.family;
            }
            if choice.size.is_some() {
                held.size = choice.size;
            }
            if choice.colour.is_some() {
                held.colour = choice.colour;
            }
            self.status = "Font for typing".into();
            return;
        }
        let (start, end) = selection.ordered();
        self.execute_page_command(&amipro_editing::Command::SetFontParts {
            start,
            end,
            family: choice.family,
            size: choice.size,
            colour: choice.colour,
        });
        self.page_selection = page_editing::valid_selection(&self.document, selection);
        self.sync_editor_to_page();
        self.status = "Font applied".into();
    }

    fn apply_font(&mut self, choice: page_editing::FontChoice) {
        if self.frame_selection.is_some() {
            return;
        }
        let selection = self.page_selection;
        if selection.is_caret() {
            self.typing_format_mut().font = Some(choice);
            self.status = "Font for typing".into();
            return;
        }
        let (start, end) = selection.ordered();
        self.execute_page_command(&amipro_editing::Command::SetFont {
            start,
            end,
            family: choice.family,
            size: choice.size,
            colour: choice.colour,
        });
        self.page_selection = page_editing::valid_selection(&self.document, selection);
        self.sync_editor_to_page();
        self.status = "Font applied".into();
    }

    /// Runs a frame gesture's commands as one undo step. A key press within
    /// a second of the previous one, with no other step between, joins its
    /// step.
    /// The scaling that sizes the picture in `frame` with its frame resized
    /// from `from` to `to` (Shift and a corner handle; P6): Custom, its
    /// size as drawn now scaled as the frame is.
    #[cfg(feature = "page-surface")]
    fn picture_scaled_with_frame(
        &self,
        frame: amipro_document::FrameId,
        page: &amipro_layout::PageBox,
        from: amipro_document::Rect,
        to: amipro_document::Rect,
    ) -> Option<amipro_document::PictureScaling> {
        let amipro_document::FrameContent::Picture(picture) = &self.document.frame(frame)?.content
        else {
            return None;
        };
        let frame_box = page
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)?;
        let area = page_surface::picture_area(frame_box);
        let area = amipro_document::Size {
            width: area.width.round() as i32,
            height: area.height.round() as i32,
        };
        let drawn = match self.picture_natural(frame) {
            Some(natural) => picture.scaling.drawn_size(natural, area),
            None => picture.scaling.size,
        };
        let scale = |value: i32, after: i32, before: i32| {
            (i64::from(value) * i64::from(after) / i64::from(before.max(1)))
                .clamp(1, i64::from(amipro_document::PICTURE_MAX)) as i32
        };
        Some(amipro_document::PictureScaling {
            sizing: amipro_document::PictureSizing::Custom,
            size: amipro_document::Size {
                width: scale(drawn.width, to.width(), from.width()),
                height: scale(drawn.height, to.height(), from.height()),
            },
            ..picture.scaling
        })
    }

    /// The natural size of the picture in `frame`, when a reader gives one.
    #[cfg(feature = "page-surface")]
    fn picture_natural(&self, frame: amipro_document::FrameId) -> Option<amipro_document::Size> {
        let amipro_document::FrameContent::Picture(picture) = &self.document.frame(frame)?.content
        else {
            return None;
        };
        self.pictures
            .natural_size(&self.document, picture, self.location.folder())
    }

    fn execute_frame_commands(
        &mut self,
        commands: &[amipro_editing::Command],
        key: bool,
    ) -> Result<(), String> {
        #[cfg(feature = "page-surface")]
        if self.preview_refuses_edit() {
            return Ok(());
        }
        let continuing =
            key && self.frame_keys.is_some_and(|(at, depth)| {
                at.elapsed() < std::time::Duration::from_secs(1) && depth == self.history.undo_len()
            }) || self.continues_colour_drag();
        let previous = self.document.clone();
        let cursor = self.history_caret();
        let mut started = continuing;
        let mut refused = Ok(());
        for command in commands {
            let result = if started {
                self.history
                    .amend(&mut self.document, cursor.clone(), command)
            } else {
                self.history
                    .execute(&mut self.document, cursor.clone(), command)
            };
            match result {
                Ok(executed) => started |= !executed.report.is_empty(),
                Err(error) => {
                    self.status = format!("Frame change refused: {error}");
                    refused = Err(error.to_owned());
                    break;
                }
            }
        }
        self.frame_keys = key.then(|| (std::time::Instant::now(), self.history.undo_len()));
        if self.document != previous {
            self.refresh_rich_layout();
            synchronize_editor_text(&mut self.editor, &previous, &self.document);
            self.frame_selection = self
                .frame_selection
                .as_ref()
                .and_then(|selection| selection.valid(&self.document));
            self.page_selection =
                page_editing::valid_selection(&self.document, self.page_selection);
            if refused.is_ok() {
                self.status = "Modified".into();
            }
            self.dirty = true;
        }
        refused
    }

    /// The frame the inspector shows and its page: a single selected frame.
    fn inspected_frame(
        &self,
    ) -> Option<(
        amipro_document::FrameId,
        std::sync::Arc<amipro_layout::PageBox>,
    )> {
        let selection = self.frame_selection.as_ref()?;
        let frame = selection.single(&self.document)?;
        Some((frame, self.page_box(selection.page)?))
    }

    /// The laid-out page numbered `number`.
    fn page_box(&self, number: u32) -> Option<std::sync::Arc<amipro_layout::PageBox>> {
        self.pages
            .layout()
            .pages
            .iter()
            .find(|page| page.number == number)
            .cloned()
    }

    /// The hidden text editor follows a main-text selection, so the action
    /// bar, the Styles panel, and history read the same selection.
    fn sync_editor_to_page(&mut self) {
        let selection = self.page_selection;
        if selection.flow() != self.document.main_flow_id() {
            return;
        }
        let position = |position: amipro_editing::TextPosition| text_editor::Position {
            line: position.paragraph,
            column: position.byte,
        };
        // The pinned editor keeps its old selection when given none (the
        // style field then read the old range); a move clears it.
        if selection.is_caret() && self.editor.cursor().selection.is_some() {
            self.editor
                .perform(text_editor::Action::Move(text_editor::Motion::Left));
        }
        self.editor.move_to(text_editor::Cursor {
            position: position(selection.focus),
            selection: (!selection.is_caret()).then(|| position(selection.anchor)),
        });
    }

    /// After undo or redo: the page selection the step kept, in any flow,
    /// made valid.
    fn restore_page_selection(&mut self, caret: &HistoryCaret) {
        self.page_selection = page_editing::valid_selection(&self.document, caret.page);
    }

    /// Scrolls so that the caret's line is visible.
    fn scroll_to_caret(&self) -> Task<cosmic::Action<Message>> {
        if self.viewport.height <= 0.0 {
            return Task::none();
        }
        let view = self.page_view();
        let Some(line) = page_surface::caret_line_rect(
            &self.document,
            self.pages.layout(),
            &view,
            self.page_selection,
        ) else {
            return Task::none();
        };
        let top = self.scroll_offset.y;
        let bottom = top + self.viewport.height;
        let y = if line.y < top {
            line.y - 24.0
        } else if line.y + line.height > bottom {
            line.y + line.height - self.viewport.height + 24.0
        } else {
            return Task::none();
        };
        let id = workspace_scroll_id();
        let offset = cosmic::iced::widget::scrollable::AbsoluteOffset {
            x: Some(self.scroll_offset.x),
            y: Some(y.max(0.0)),
        };
        scroll_workspace(id, offset)
    }

    /// The page the caret is on, from 1.
    fn caret_page_number(&self) -> u32 {
        page_surface::caret_page(&self.document, self.pages.layout(), self.page_selection)
    }

    fn page_count(&self) -> u32 {
        self.pages.layout().pages.len().max(1) as u32
    }

    /// Puts the caret at the top of `page`'s text and scrolls the page's top
    /// into view (Ami Pro's page arrows and Go To; a page without main text
    /// gives way to the nearest one in the direction moved).
    fn go_to_page(&mut self, page: u32, forward: bool) -> Task<cosmic::Action<Message>> {
        let Some((position, landed)) =
            page_surface::page_top(&self.document, self.pages.layout(), page, forward)
        else {
            return Task::none();
        };
        self.frame_selection = None;
        self.page_selection = PageSelection::caret(position, landed);
        self.sync_editor_to_page();
        self.status = format!("Page {landed}");
        let view = self.page_view();
        trace_scroll(|| {
            format!(
                "go to page {page} -> {landed}: viewport {:?}, page rect {:?}, offset now {:?}",
                self.viewport,
                view.pages.get(landed as usize - 1),
                self.scroll_offset
            )
        });
        let Some(rect) = view.pages.get(landed as usize - 1) else {
            return Task::none();
        };
        let id = workspace_scroll_id();
        let offset = cosmic::iced::widget::scrollable::AbsoluteOffset {
            x: Some(self.scroll_offset.x),
            y: Some((rect.y - 24.0).max(0.0)),
        };
        scroll_workspace(id, offset)
    }
}

/// Edit › Go To (Ami Pro's Go To dialog, opened by Ctrl+G or the status
/// bar's page number): the page typed and the reason a value was refused.
#[cfg(feature = "page-surface")]
#[derive(Clone, Debug, Default, PartialEq)]
struct GoToState {
    page: String,
    error: Option<String>,
    /// Next item: which kind Previous and Next go to
    /// ([`reference_commands::GO_TO_ITEMS`]; Ami Pro's Next item list).
    item: usize,
}

/// What the Go To dialog asks for.
#[cfg(feature = "page-surface")]
#[derive(Clone, Debug)]
enum GoToAction {
    Input(String),
    Go,
    First,
    Last,
    NextFrame,
    /// Next item's kind, and a step to the previous or next one (NR4).
    Item(usize),
    Step(bool),
    Cancel,
}

/// What Frame Properties' New frames buttons ask for (decision F2).
#[cfg(feature = "page-surface")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NewFrameDefault {
    /// Make Default for New Frames: the inspected frame's settings.
    Make,
    /// Restore Ami Pro's Default.
    Restore,
}

/// What the Rename Style dialog asks for.
#[derive(Clone, Debug)]
enum RenameAction {
    Name(String),
    Rename,
    Cancel,
}

/// What the Save as a Style Sheet dialog asks for.
#[derive(Clone, Debug)]
enum SaveSheetAction {
    Description(String),
    /// With contents: the main text, page frames, header and footer text,
    /// and pictures go into the sheet (`UI_SPEC.md` §14).
    Contents(bool),
    /// Choose the file (the portal's save dialog) and write it.
    Choose,
    Cancel,
}

/// Appends a line to the file named by `AMIPRO_TRACE_DROP`, when set: a
/// diagnostic for paste and drop on the user's desktop (2026-10-18), off
/// by default, with libcosmic's own drop and clipboard traces
/// ([`DropTrace`]).
#[cfg(feature = "page-surface")]
fn trace_drop(line: impl FnOnce() -> String) {
    if let Some(path) = std::env::var_os("AMIPRO_TRACE_DROP") {
        append_trace(&path, line());
    }
}

/// Passes the traces of libcosmic's drop target and the runtime's
/// clipboard to the `AMIPRO_TRACE_DROP` file.
struct DropTrace(std::ffi::OsString);

impl DropTrace {
    const TARGETS: [&str; 3] = [
        "libcosmic::widget::dnd_destination",
        "iced::winit::clipboard",
        "iced::winit::program",
    ];

    fn install() {
        if let Some(path) = std::env::var_os("AMIPRO_TRACE_DROP")
            && log::set_boxed_logger(Box::new(Self(path))).is_ok()
        {
            log::set_max_level(log::LevelFilter::Trace);
        }
    }
}

impl log::Log for DropTrace {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        Self::TARGETS
            .iter()
            .any(|target| metadata.target().starts_with(target))
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = record.args().to_string();
        // The runtime traces every event; only drops are wanted.
        if record.target().starts_with("iced::winit::program") && !line.starts_with("Dnd") {
            return;
        }
        append_trace(&self.0, format!("{}: {line}", record.target()));
    }

    fn flush(&self) {}
}

fn append_trace(path: &std::ffi::OsStr, line: String) {
    use std::io::Write as _;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{:?} {line}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default(),
        );
    }
}

/// Appends a line to the file named by `AMIPRO_TRACE_SCROLL`, when set: a
/// diagnostic for scrolling on the user's desktop (2026-10-12), off by
/// default.
fn trace_scroll(line: impl FnOnce() -> String) {
    if let Some(path) = std::env::var_os("AMIPRO_TRACE_SCROLL") {
        append_trace(&path, line());
    }
}

/// Scrolls a document's workspace, then repaints. The pinned runtime
/// (libcosmic `dbc62f2`, iced winit `run_action`) applies a widget operation
/// without requesting a redraw, and skips the frame when no message or
/// event is pending, so a scroll alone stayed invisible until the next
/// input (user, 2026-10-12: the page arrows moved the caret but not the
/// view). A no-op message after it makes the runtime update and redraw.
/// The page workspace's scrollable, one id for every tab. The pinned
/// libcosmic's tree diff keeps the id a scrollable first had at its place in
/// the widget tree, so a per-tab id never reached a tab opened later: its
/// scroll operations matched nothing until a resize rebuilt the tree. The
/// tabs share the scrollable's state, and `follow_active_session` restores
/// each tab's own offset.
const WORKSPACE_SCROLL: &str = "document-scroll";

fn workspace_scroll_id() -> cosmic::iced::widget::Id {
    cosmic::iced::widget::Id::new(WORKSPACE_SCROLL)
}

fn scroll_workspace(
    id: cosmic::iced::widget::Id,
    offset: cosmic::iced::widget::scrollable::AbsoluteOffset<Option<f32>>,
) -> Task<cosmic::Action<Message>> {
    trace_scroll(|| format!("scroll_to {id:?} {offset:?}"));
    cosmic::iced::widget::scrollable::scroll_to::<Message>(id, offset)
        .chain(Task::done(Message::Repaint))
        .map(cosmic::Action::from)
}

/// Native tab entity data linking a tab to its `DocumentSession::id`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TabSession(u64);

/// What the shared page workspace last showed (see [`WORKSPACE_SCROLL`]).
#[derive(Clone, Debug, Default)]
struct WorkspaceState {
    /// The session whose pages the workspace scrollable holds.
    shown_session: Option<u64>,
    /// The page surface's visible size, the same for every tab; a tab that
    /// has not received an event yet takes it from here.
    #[cfg(feature = "page-surface")]
    viewport: cosmic::iced::Size,
}

/// The context rail's state beyond the docked panel's visibility
/// (`UI_SPEC.md` §5 "Styles panel"; gaps R3 and R6).
#[derive(Clone, Debug, Default)]
struct RailState {
    /// The Styles filter text.
    style_filter: String,
    /// The window's width once a resize event has reported it.
    window_width: Option<f32>,
    /// Below [`RAIL_OVERLAY_WIDTH`] the rail opens over the workspace; it
    /// starts closed there.
    overlay_open: bool,
    /// The Styles panel row the right button last pressed, which the
    /// panel's one right-click menu acts on; none once an entry is chosen
    /// or the menu closes.
    context_style: Option<StyleId>,
}

/// The user's interface choices that outlive a session: the toolbar and
/// whether it shows (`UI_SPEC.md` §12).
#[derive(Clone, Debug, PartialEq)]
struct Preferences {
    toolbar: Vec<ToolbarItem>,
    toolbar_visible: bool,
    /// View › Ruler (`UI_SPEC.md` §19).
    ruler_visible: bool,
    /// View › Status Bar: shown by default; hidden, it still appears while
    /// the message is a failure, so an error is never concealed.
    status_bar_visible: bool,
    /// Ami Pro's style-sheet path (Tools › User Setup › Paths): folders
    /// searched for a document's style sheet and listed by Use Another
    /// Style Sheet. A sheet chosen with Browse… adds its folder.
    style_folders: Vec<PathBuf>,
    /// The last frame made, which Insert › Frame… offers next (Ami Pro's
    /// AMIPRO.INI `frmcrt`; `UI_SPEC.md` §20).
    last_frame: Option<amipro_editing::frame_creation::FrameGeometry>,
    /// What new frames take: Ami Pro's default frame until Make Default
    /// for New Frames (Ami Pro's `FrDefs`; decision F2).
    frame_defaults: amipro_editing::frame_creation::FrameDefaults,
    /// The last bullet Insert › Bullet… put in, which it offers next
    /// (decision B4).
    last_bullet: Option<char>,
    /// The size Insert › Table… made last, which it offers next (Ami Pro
    /// offers 5 × 5).
    insert_table: Option<(usize, usize)>,
    /// Recent colours, newest first (`UI_SPEC.md` §23, C4), as
    /// `COLORREF`s; `colours::Recent` keeps the list's rules.
    recent_colours: Vec<u32>,
    /// Find & Replace's options, Ami Pro's by default and kept between
    /// sessions (`UI_SPEC.md` §27, F2).
    find_options: amipro_editing::find::FindOptions,
    /// Recent searches, newest first (F4).
    recent_searches: Vec<String>,
    /// View › Field Shading (`UI_SPEC.md` §28, FL6): on by default.
    field_shading: bool,
    /// View › Notes and View › Bookmarks (`UI_SPEC.md` §34, NR1, NR5).
    show_notes: bool,
    show_bookmarks: bool,
    /// The initials and colour new notes take (Ami Pro's User Setup).
    note_initials: String,
    note_colour: u32,
    /// Generate Index joins pages into ranges (TX5; off: Ami Pro's form).
    index_ranges: bool,
    /// The glossary data file (Ami Pro's `AMIPRO.INI` `Glossary=`).
    glossary_file: String,
    /// Complete glossary names as you type (TX7, off by default).
    complete_glossary: bool,
    /// Ami Pro's Spell Check options as its `SpellOpts` bits
    /// (`UI_SPEC.md` §35; default 0: Ami Pro's).
    spell_options: u32,
    /// Tools › Check Spelling as You Type (SP2): on by default.
    spell_as_you_type: bool,
    /// Grammar underlines as you type (SP8): off by default.
    grammar_as_you_type: bool,
    /// Ami Pro's Grammar Check options as its `GrammarOpts` bits (default
    /// 13: statistics, other streams, explanations).
    grammar_options: u32,
    /// The language of new documents (Ami Pro's `dictlang`, SP6): a
    /// locale; empty is American.
    default_language: String,
    /// Power fields saved with a name (Ami Pro's AMIPRO.INI `[Fields]`):
    /// name and code.
    saved_fields: Vec<(String, String)>,
    /// The PDF's additions (`UI_SPEC.md` §29, PO4).
    #[cfg(feature = "page-surface")]
    pdf_additions: print::PdfAdditions,
    /// The folder the last export went to, where the next one starts
    /// (`UI_SPEC.md` §30, EX10).
    export_folder: Option<PathBuf>,
    /// Preferences › Folders › Documents (DF1): where the file choosers
    /// start when the document has no folder of its own; none until set.
    documents_folder: Option<PathBuf>,
    /// Export Folder's last folder of documents and its format (EX10).
    export_from: Option<PathBuf>,
    export_format: String,
    /// Import Options' last choice (IM2): the style option, the text's line
    /// ends, Keep style names.
    import_styles: String,
    import_line_ends: String,
    import_keep_names: bool,
    /// Import Folder's last folders (IM11).
    import_from: Option<PathBuf>,
    import_to: Option<PathBuf>,
    /// Print Envelope's options (Ami Pro's `AMIPRO2.INI` `[AmiEnv]`, MG8).
    #[cfg(feature = "page-surface")]
    envelope: envelope_panel::EnvelopeOptions,
    /// Return addresses saved by name (Ami Pro's `[AdrsBook]`): name and
    /// lines.
    return_addresses: Vec<(String, String)>,
    /// Undo levels each document keeps (Preferences › General, CM6).
    undo_levels: usize,
    /// Saving over an Ami Pro document keeps the original as
    /// `NAME.SAM.bak` the first time (user, 2026-11-04; CM6).
    keep_backup: bool,
    /// How many recent files the File menu lists, and the files, newest
    /// first (Ami Pro's File menu list).
    recent_count: usize,
    recent_files: Vec<PathBuf>,
    /// File › New…'s sheet when none was used yet (Ami Pro's `defstyle`),
    /// and the sheets it made documents from, newest first.
    default_sheet: String,
    recent_sheets: Vec<String>,
    /// Insert › Special Character…'s recent characters, newest first.
    recent_characters: Vec<char>,
    /// View › Custom Zoom's percentage (Ami Pro's `viewlevel`, default 91).
    custom_zoom: u16,
    /// View › Show ▸'s marks (Ami Pro's View Preferences; CM10): Tabs &
    /// Returns and Column Guides off, Margins in Colour and Pictures on.
    tabs_returns: bool,
    column_guides: bool,
    margins_in_colour: bool,
    show_pictures: bool,
    /// View › Show ▸ Table row/column headings: letters over and numbers
    /// beside a page table's cells (TD.7, TN9).
    table_addresses: bool,
    /// View › Outline ▸ Use Outline Styles (OB.5): Outline mode shows each
    /// level in `_OUTLINE.STY`'s font; the app's, as Ami Pro's INI bit.
    outline_styles: bool,
    /// Preferences › General › Check for updates when the program starts
    /// (P6.2; off until the user turns it on), and the time (Unix seconds)
    /// of the last check, so that it runs once a day at most.
    check_updates_on_start: bool,
    last_update_check: u64,
    /// Preferences › General › Keep recovery copies while editing (P6.3),
    /// and how often they are written, in minutes.
    keep_recovery: bool,
    recovery_minutes: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            toolbar: chrome::standard_toolbar(),
            toolbar_visible: true,
            ruler_visible: true,
            status_bar_visible: true,
            style_folders: Vec::new(),
            last_frame: None,
            frame_defaults: amipro_editing::frame_creation::FrameDefaults::AMI_PRO,
            last_bullet: None,
            insert_table: None,
            recent_colours: Vec::new(),
            find_options: amipro_editing::find::FindOptions::default(),
            recent_searches: Vec::new(),
            field_shading: true,
            show_notes: true,
            show_bookmarks: false,
            note_initials: std::env::var("USER")
                .ok()
                .map(|name| references_initials(&name))
                .filter(|initials| !initials.is_empty())
                .unwrap_or_else(|| "AP".into()),
            index_ranges: false,
            glossary_file: String::new(),
            complete_glossary: false,
            note_colour: amipro_document::NoteInfo::YELLOW,
            spell_options: 0,
            spell_as_you_type: true,
            grammar_as_you_type: false,
            grammar_options: 13,
            default_language: String::new(),
            saved_fields: Vec::new(),
            #[cfg(feature = "page-surface")]
            pdf_additions: print::PdfAdditions::default(),
            export_folder: None,
            documents_folder: None,
            export_from: None,
            export_format: "docx".into(),
            import_styles: "convert".into(),
            import_line_ends: "paragraphs".into(),
            import_keep_names: false,
            import_from: None,
            import_to: None,
            #[cfg(feature = "page-surface")]
            envelope: envelope_panel::EnvelopeOptions::default(),
            return_addresses: Vec::new(),
            undo_levels: amipro_editing::History::<()>::DEFAULT_LIMIT,
            keep_backup: true,
            recent_count: 4,
            recent_files: Vec::new(),
            default_sheet: "_DEFAULT.STY".into(),
            recent_sheets: Vec::new(),
            recent_characters: Vec::new(),
            custom_zoom: 91,
            tabs_returns: false,
            column_guides: false,
            margins_in_colour: true,
            show_pictures: true,
            table_addresses: false,
            outline_styles: false,
            check_updates_on_start: false,
            last_update_check: 0,
            keep_recovery: true,
            recovery_minutes: preferences_dialog::RECOVERY_DEFAULT,
        }
    }
}

/// The preferences' place in the user's COSMIC configuration
/// (`~/.config/cosmic/<APP_ID>/v1/`; `UI_SPEC.md` §12, gap R5), with what was
/// last written there. Tests run without one, or with one in a scratch
/// directory.
struct Settings {
    config: cosmic::cosmic_config::Config,
    /// The preferences and the docked rail's visibility as last saved.
    saved: (Preferences, bool),
}

const SETTINGS_VERSION: u64 = 1;

impl Settings {
    /// Reads the preferences and the docked rail's visibility (when saved);
    /// anything missing or unreadable keeps its default.
    fn load(config: cosmic::cosmic_config::Config) -> (Self, Preferences, Option<bool>) {
        use cosmic::cosmic_config::ConfigGet;
        let defaults = Preferences::default();
        let preferences = Preferences {
            toolbar: config
                .get::<Vec<String>>("toolbar")
                .map_or(defaults.toolbar, |keys| chrome::toolbar_from_keys(&keys)),
            toolbar_visible: config
                .get("toolbar_visible")
                .unwrap_or(defaults.toolbar_visible),
            ruler_visible: config
                .get("ruler_visible")
                .unwrap_or(defaults.ruler_visible),
            status_bar_visible: config
                .get("status_bar_visible")
                .unwrap_or(defaults.status_bar_visible),
            style_folders: config
                .get::<Vec<String>>("style_folders")
                .map_or(defaults.style_folders, |folders| {
                    folders.into_iter().map(PathBuf::from).collect()
                }),
            last_frame: config
                .get::<Vec<i64>>("last_frame")
                .ok()
                .and_then(|numbers| frame_geometry_from(&numbers))
                .or(defaults.last_frame),
            frame_defaults: config
                .get::<Vec<i64>>("frame_defaults")
                .ok()
                .and_then(|numbers| {
                    amipro_editing::frame_creation::FrameDefaults::from_numbers(&numbers)
                })
                .unwrap_or(defaults.frame_defaults),
            last_bullet: config
                .get::<u32>("last_bullet")
                .ok()
                .and_then(char::from_u32)
                .filter(|code| amipro_document::is_bullet_code(*code))
                .or(defaults.last_bullet),
            insert_table: config
                .get::<Vec<u32>>("insert_table")
                .ok()
                .and_then(|size| match size.as_slice() {
                    [columns @ 1..=250, rows @ 1..=4000] => {
                        Some((*columns as usize, *rows as usize))
                    }
                    _ => None,
                })
                .or(defaults.insert_table),
            recent_colours: config
                .get::<Vec<u32>>("recent_colours")
                .unwrap_or(defaults.recent_colours),
            find_options: config
                .get::<Vec<i64>>("find_options")
                .ok()
                .and_then(|numbers| find_options_from(&numbers))
                .unwrap_or(defaults.find_options),
            recent_searches: config
                .get::<Vec<String>>("recent_searches")
                .unwrap_or(defaults.recent_searches),
            field_shading: config
                .get("field_shading")
                .unwrap_or(defaults.field_shading),
            show_notes: config.get("show_notes").unwrap_or(defaults.show_notes),
            show_bookmarks: config
                .get("show_bookmarks")
                .unwrap_or(defaults.show_bookmarks),
            note_initials: config
                .get("note_initials")
                .unwrap_or(defaults.note_initials),
            note_colour: config.get("note_colour").unwrap_or(defaults.note_colour),
            index_ranges: config.get("index_ranges").unwrap_or(defaults.index_ranges),
            glossary_file: config
                .get("glossary_file")
                .unwrap_or(defaults.glossary_file),
            complete_glossary: config
                .get("complete_glossary")
                .unwrap_or(defaults.complete_glossary),
            spell_options: config
                .get("spell_options")
                .unwrap_or(defaults.spell_options),
            spell_as_you_type: config
                .get("spell_as_you_type")
                .unwrap_or(defaults.spell_as_you_type),
            grammar_as_you_type: config
                .get("grammar_as_you_type")
                .unwrap_or(defaults.grammar_as_you_type),
            grammar_options: config
                .get("grammar_options")
                .unwrap_or(defaults.grammar_options),
            default_language: config
                .get("default_language")
                .unwrap_or(defaults.default_language),
            saved_fields: config.get::<Vec<String>>("saved_fields").map_or(
                defaults.saved_fields,
                |fields| {
                    fields
                        .iter()
                        .filter_map(|field| field.split_once('\u{1F}'))
                        .map(|(name, code)| (name.to_owned(), code.to_owned()))
                        .collect()
                },
            ),
            #[cfg(feature = "page-surface")]
            pdf_additions: print::PdfAdditions {
                bookmarks: config
                    .get("pdf_bookmarks")
                    .unwrap_or(defaults.pdf_additions.bookmarks),
                properties: config
                    .get("pdf_properties")
                    .unwrap_or(defaults.pdf_additions.properties),
                archival: config
                    .get("pdf_archival")
                    .unwrap_or(defaults.pdf_additions.archival),
            },
            export_folder: config
                .get::<String>("export_folder")
                .ok()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            documents_folder: config
                .get::<String>("documents_folder")
                .ok()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            export_from: config
                .get::<String>("export_from")
                .ok()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            export_format: config
                .get::<String>("export_format")
                .unwrap_or(defaults.export_format),
            import_styles: config
                .get::<String>("import_styles")
                .unwrap_or(defaults.import_styles),
            import_line_ends: config
                .get::<String>("import_line_ends")
                .unwrap_or(defaults.import_line_ends),
            import_keep_names: config
                .get::<bool>("import_keep_names")
                .unwrap_or(defaults.import_keep_names),
            import_from: config
                .get::<String>("import_from")
                .ok()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            import_to: config
                .get::<String>("import_to")
                .ok()
                .filter(|folder| !folder.is_empty())
                .map(PathBuf::from),
            #[cfg(feature = "page-surface")]
            envelope: config
                .get::<Vec<i64>>("envelope")
                .ok()
                .and_then(|numbers| {
                    envelope_panel::EnvelopeOptions::from_numbers(
                        &numbers,
                        config.get::<String>("envelope_return").unwrap_or_default(),
                    )
                })
                .unwrap_or(defaults.envelope),
            return_addresses: config.get::<Vec<String>>("return_addresses").map_or(
                defaults.return_addresses,
                |saved| {
                    saved
                        .iter()
                        .filter_map(|entry| entry.split_once('\u{1F}'))
                        .map(|(name, lines)| (name.to_owned(), lines.to_owned()))
                        .collect()
                },
            ),
            undo_levels: config
                .get::<usize>("undo_levels")
                .ok()
                .filter(|levels| preferences_dialog::UNDO_LEVELS.contains(levels))
                .unwrap_or(defaults.undo_levels),
            keep_backup: config.get("keep_backup").unwrap_or(defaults.keep_backup),
            recent_count: config
                .get::<usize>("recent_count")
                .ok()
                .filter(|count| preferences_dialog::RECENT_FILES.contains(count))
                .unwrap_or(defaults.recent_count),
            recent_files: config
                .get::<Vec<String>>("recent_files")
                .map_or(defaults.recent_files, |files| {
                    files.into_iter().map(PathBuf::from).collect()
                }),
            default_sheet: config
                .get::<String>("default_sheet")
                .ok()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(defaults.default_sheet),
            recent_sheets: config
                .get::<Vec<String>>("recent_sheets")
                .unwrap_or(defaults.recent_sheets),
            recent_characters: config
                .get::<String>("recent_characters")
                .map_or(defaults.recent_characters, |text| text.chars().collect()),
            custom_zoom: config
                .get::<u16>("custom_zoom")
                .ok()
                .filter(|percent| preferences_dialog::CUSTOM_ZOOM.contains(&usize::from(*percent)))
                .unwrap_or(defaults.custom_zoom),
            tabs_returns: config.get("tabs_returns").unwrap_or(defaults.tabs_returns),
            column_guides: config
                .get("column_guides")
                .unwrap_or(defaults.column_guides),
            margins_in_colour: config
                .get("margins_in_colour")
                .unwrap_or(defaults.margins_in_colour),
            show_pictures: config
                .get("show_pictures")
                .unwrap_or(defaults.show_pictures),
            table_addresses: config
                .get("table_addresses")
                .unwrap_or(defaults.table_addresses),
            outline_styles: config
                .get("outline_styles")
                .unwrap_or(defaults.outline_styles),
            check_updates_on_start: config
                .get("check_updates_on_start")
                .unwrap_or(defaults.check_updates_on_start),
            last_update_check: config
                .get("last_update_check")
                .unwrap_or(defaults.last_update_check),
            keep_recovery: config
                .get("keep_recovery")
                .unwrap_or(defaults.keep_recovery),
            recovery_minutes: config
                .get("recovery_minutes")
                .unwrap_or(defaults.recovery_minutes),
        };
        let styles_visible = config.get("styles_visible").ok();
        let settings = Self {
            config,
            saved: (preferences.clone(), styles_visible.unwrap_or(true)),
        };
        (settings, preferences, styles_visible)
    }

    fn save(
        &mut self,
        preferences: &Preferences,
        styles_visible: bool,
    ) -> Result<(), cosmic::cosmic_config::Error> {
        use cosmic::cosmic_config::ConfigSet;
        if self.saved.0 == *preferences && self.saved.1 == styles_visible {
            return Ok(());
        }
        // Once per change, also when it fails, so a failure is reported once.
        self.saved = (preferences.clone(), styles_visible);
        let transaction = self.config.transaction();
        let keys: Vec<String> = preferences
            .toolbar
            .iter()
            .map(|item| chrome::toolbar_item_key(*item))
            .collect();
        transaction.set("toolbar", keys)?;
        transaction.set("toolbar_visible", preferences.toolbar_visible)?;
        transaction.set("ruler_visible", preferences.ruler_visible)?;
        transaction.set("status_bar_visible", preferences.status_bar_visible)?;
        let folders: Vec<String> = preferences
            .style_folders
            .iter()
            .map(|folder| folder.to_string_lossy().into_owned())
            .collect();
        transaction.set("style_folders", folders)?;
        if let Some(frame) = preferences.last_frame {
            let numbers: Vec<i64> = [frame.width, frame.height, frame.top, frame.left]
                .into_iter()
                .map(i64::from)
                .collect();
            transaction.set("last_frame", numbers)?;
        }
        transaction.set(
            "frame_defaults",
            preferences.frame_defaults.to_numbers().to_vec(),
        )?;
        if let Some(code) = preferences.last_bullet {
            transaction.set("last_bullet", code as u32)?;
        }
        if let Some((columns, rows)) = preferences.insert_table {
            transaction.set("insert_table", vec![columns as u32, rows as u32])?;
        }
        transaction.set("recent_colours", preferences.recent_colours.clone())?;
        transaction.set(
            "find_options",
            find_options_numbers(&preferences.find_options),
        )?;
        transaction.set("recent_searches", preferences.recent_searches.clone())?;
        transaction.set("field_shading", preferences.field_shading)?;
        transaction.set("show_notes", preferences.show_notes)?;
        transaction.set("show_bookmarks", preferences.show_bookmarks)?;
        transaction.set("note_initials", preferences.note_initials.clone())?;
        transaction.set("note_colour", preferences.note_colour)?;
        transaction.set("index_ranges", preferences.index_ranges)?;
        transaction.set("glossary_file", preferences.glossary_file.clone())?;
        transaction.set("complete_glossary", preferences.complete_glossary)?;
        transaction.set("spell_options", preferences.spell_options)?;
        transaction.set("spell_as_you_type", preferences.spell_as_you_type)?;
        transaction.set("grammar_as_you_type", preferences.grammar_as_you_type)?;
        transaction.set("grammar_options", preferences.grammar_options)?;
        transaction.set("default_language", preferences.default_language.clone())?;
        let saved: Vec<String> = preferences
            .saved_fields
            .iter()
            .map(|(name, code)| format!("{name}\u{1F}{code}"))
            .collect();
        transaction.set("saved_fields", saved)?;
        let addresses: Vec<String> = preferences
            .return_addresses
            .iter()
            .map(|(name, lines)| format!("{name}\u{1F}{lines}"))
            .collect();
        transaction.set("return_addresses", addresses)?;
        transaction.set("undo_levels", preferences.undo_levels)?;
        transaction.set("keep_backup", preferences.keep_backup)?;
        transaction.set("recent_count", preferences.recent_count)?;
        let recent: Vec<String> = preferences
            .recent_files
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        transaction.set("recent_files", recent)?;
        transaction.set("default_sheet", preferences.default_sheet.clone())?;
        transaction.set("recent_sheets", preferences.recent_sheets.clone())?;
        transaction.set("custom_zoom", preferences.custom_zoom)?;
        transaction.set("tabs_returns", preferences.tabs_returns)?;
        transaction.set("column_guides", preferences.column_guides)?;
        transaction.set("margins_in_colour", preferences.margins_in_colour)?;
        transaction.set("show_pictures", preferences.show_pictures)?;
        transaction.set("table_addresses", preferences.table_addresses)?;
        transaction.set("outline_styles", preferences.outline_styles)?;
        transaction.set("check_updates_on_start", preferences.check_updates_on_start)?;
        transaction.set("last_update_check", preferences.last_update_check)?;
        transaction.set("keep_recovery", preferences.keep_recovery)?;
        transaction.set("recovery_minutes", preferences.recovery_minutes)?;
        transaction.set(
            "recent_characters",
            preferences.recent_characters.iter().collect::<String>(),
        )?;
        #[cfg(feature = "page-surface")]
        {
            transaction.set("envelope", preferences.envelope.to_numbers())?;
            transaction.set("envelope_return", preferences.envelope.last_return.clone())?;
        }
        #[cfg(feature = "page-surface")]
        {
            transaction.set("pdf_bookmarks", preferences.pdf_additions.bookmarks)?;
            transaction.set("pdf_properties", preferences.pdf_additions.properties)?;
            transaction.set("pdf_archival", preferences.pdf_additions.archival)?;
        }
        transaction.set(
            "export_folder",
            preferences
                .export_folder
                .as_ref()
                .map_or_else(String::new, |folder| folder.display().to_string()),
        )?;
        transaction.set(
            "documents_folder",
            preferences
                .documents_folder
                .as_ref()
                .map_or_else(String::new, |folder| folder.display().to_string()),
        )?;
        transaction.set(
            "export_from",
            preferences
                .export_from
                .as_ref()
                .map_or_else(String::new, |folder| folder.display().to_string()),
        )?;
        transaction.set("export_format", preferences.export_format.clone())?;
        transaction.set("import_styles", preferences.import_styles.clone())?;
        transaction.set("import_line_ends", preferences.import_line_ends.clone())?;
        transaction.set("import_keep_names", preferences.import_keep_names)?;
        for (key, folder) in [
            ("import_from", &preferences.import_from),
            ("import_to", &preferences.import_to),
        ] {
            transaction.set(
                key,
                folder
                    .as_ref()
                    .map_or_else(String::new, |folder| folder.display().to_string()),
            )?;
        }
        transaction.set("styles_visible", styles_visible)?;
        transaction.commit()
    }
}

/// Find & Replace's options as the settings keep them (the page surface's
/// find bar; the text editor keeps them unread).
fn find_options_numbers(options: &amipro_editing::find::FindOptions) -> Vec<i64> {
    #[cfg(feature = "page-surface")]
    return find_bar::options_to_numbers(options);
    #[cfg(not(feature = "page-surface"))]
    {
        let _ = options;
        Vec::new()
    }
}

fn find_options_from(numbers: &[i64]) -> Option<amipro_editing::find::FindOptions> {
    #[cfg(feature = "page-surface")]
    return find_bar::options_from_numbers(numbers);
    #[cfg(not(feature = "page-surface"))]
    {
        let _ = numbers;
        None
    }
}

/// The last frame's width, height, top, and left as the settings keep them
/// (Ami Pro's `frmcrt` order), or `None` when they are not four lengths.
fn frame_geometry_from(numbers: &[i64]) -> Option<amipro_editing::frame_creation::FrameGeometry> {
    let [width, height, top, left] = *numbers else {
        return None;
    };
    let twips = |value: i64| {
        amipro_document::Twips::try_from(value)
            .ok()
            .filter(|value| (0..=amipro_editing::layout_setup::MAX_LENGTH).contains(value))
    };
    Some(amipro_editing::frame_creation::FrameGeometry {
        width: twips(width).filter(|width| *width > 0)?,
        height: twips(height).filter(|height| *height > 0)?,
        top: twips(top)?,
        left: twips(left)?,
    })
}

/// Windows narrower than this show the rail as an overlay, so that it never
/// squeezes the page (`UI_SPEC.md` §5).
const RAIL_OVERLAY_WIDTH: f32 = 1_200.0;
/// The context rail's width (`UI_SPEC.md` §4: 216 by default).
const RAIL_WIDTH: f32 = 216.0;

struct AmiProApp {
    core: cosmic::Core,
    /// Whether Shift is held: Shift+Enter in the find bar finds the
    /// previous match.
    #[cfg(feature = "page-surface")]
    shift: bool,
    /// Whether Ctrl is held: Ctrl+click on a TOC row goes to its heading
    /// (TX6).
    #[cfg(feature = "page-surface")]
    control: bool,
    /// The find bar last closed, in any tab: its texts open the next one,
    /// as Ami Pro's dialog keeps them for the session.
    #[cfg(feature = "page-surface")]
    last_find: Option<find_bar::FindBar>,
    /// The spelling dictionaries and the user's (`UI_SPEC.md` §35).
    #[cfg(feature = "page-surface")]
    spelling: spelling::Spelling,
    /// Power fields' Set variables: Ami Pro's macro globals, which live
    /// while it runs, across documents (VM fixture FD6).
    #[cfg(feature = "page-surface")]
    field_variables: std::collections::BTreeMap<String, amipro_editing::field_language::Value>,
    sessions: Vec<DocumentSession>,
    /// Derived native tab state; `sync_tab_model` keeps it in session order.
    tab_model: segmented_button::SingleSelectModel,
    active_session: usize,
    next_session_id: u64,
    pending_close: Option<usize>,
    /// Import Options, before a file is read (`UI_SPEC.md` §31): the
    /// document does not exist yet, as in Ami Pro.
    import_dialog: Option<import::OptionsDialog>,
    close_after_save: Option<u64>,
    styles_visible: bool,
    rail: RailState,
    workspace: WorkspaceState,
    preferences: Preferences,
    settings: Option<Settings>,
    /// Help › Check for Updates… (P6.2): where the update stands, and the
    /// network and system it works through (a fake in tests).
    #[cfg(feature = "page-surface")]
    updates: updates_panel::UpdatesState,
    #[cfg(feature = "page-surface")]
    update_backend: Arc<dyn updates::Backend>,
    /// File › Recover Documents… and the autosave behind it (P6.3).
    #[cfg(feature = "page-surface")]
    recovery: recovery_panel::RecoveryState,
    /// Help (P6.4): where the reader is, and Back and Forward.
    #[cfg(feature = "page-surface")]
    help: help_panel::HelpState,
    /// Tools › Customize Toolbar…, while it is open.
    toolbar_editor: Option<ToolbarEditor>,
    /// Tools › Preferences…, while it is open (CM6).
    preferences_dialog: Option<preferences_dialog::PreferencesState>,
    /// File › Revert to Saved… asks first.
    revert_pending: bool,
    /// The password box, when it is open.
    password_dialog: Option<password_dialog::PasswordDialog>,
    /// View › Show Clean Screen (CM9): the chrome hidden.
    clean_screen: bool,
    /// Insert › Special Character…, while it is open (CM12).
    #[cfg(feature = "page-surface")]
    special_character: bool,
    /// The status bar's Face list (CM12): the installed families and the
    /// faces documents name, kept sorted.
    #[cfg(feature = "page-surface")]
    font_families: Vec<String>,
    /// Fast Format's question, while it asks (CM4).
    #[cfg(feature = "page-surface")]
    fast_format_ask: Option<fast_format::FastFormatAsk>,
    /// File › New…, while it is open (CM1).
    new_document_dialog: Option<new_document_dialog::NewDocumentDialog>,
    about: About,
    show_about: bool,
    /// Help › Keyboard Shortcuts or Compatibility and Fidelity, beside
    /// About in the context drawer (CM12).
    help_drawer: Option<help_drawers::HelpDrawer>,
    quit_requested: bool,
    clipboard_has_text: Option<bool>,
    /// Whether the clipboard holds an image Paste takes (P7).
    #[cfg(feature = "page-surface")]
    clipboard_has_image: Option<bool>,
    /// The cells copied last (`UI_SPEC.md` §32): the text the clipboard got
    /// for them, the cells, and whether they were a whole table; a paste of
    /// that text pastes the cells.
    #[cfg(feature = "page-surface")]
    cell_clipboard: Option<(String, amipro_editing::tables::CellBlock, bool)>,
    /// The last Draw mode's look and grid, for the next drawing (Ami Pro
    /// keeps them in `AmiDraw.ini`).
    #[cfg(feature = "page-surface")]
    draw_memory: Option<draw_mode::DrawMode>,
    /// Drawing objects copied in Draw mode, for Paste when the clipboard
    /// gives nothing a drawing takes.
    #[cfg(feature = "page-surface")]
    draw_clipboard: Option<Vec<amipro_pictures::sdw::Object>>,
    /// Raised when the page should take the keyboard without a click (a
    /// new data file's first cell): the page surface takes it when this
    /// changes.
    #[cfg_attr(not(feature = "page-surface"), allow(dead_code))]
    page_keyboard: u64,
    /// Edit on the page surface rather than the text editor. True in builds
    /// with the feature; tests of the other editors leave it off.
    #[cfg(feature = "page-surface")]
    page_surface: bool,
}

impl Deref for AmiProApp {
    type Target = DocumentSession;

    fn deref(&self) -> &Self::Target {
        &self.sessions[self.active_session]
    }
}

impl DerefMut for AmiProApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sessions[self.active_session]
    }
}

#[derive(Clone, Debug)]
enum Message {
    /// Nothing to change: asks the runtime for a new frame (see
    /// [`scroll_workspace`]).
    Repaint,
    /// The Go To dialog.
    #[cfg(feature = "page-surface")]
    GoTo(GoToAction),
    /// The find bar (`UI_SPEC.md` §27).
    #[cfg(feature = "page-surface")]
    Find(find_bar::FindAction),
    /// Esc: cancels the topmost open dialog; with none, ends Show Clean
    /// Screen, then closes a focused find bar.
    Escape,
    /// The answer to a field's prompt (`UI_SPEC.md` §28).
    #[cfg(feature = "page-surface")]
    FieldPrompt(field_commands::PromptAction),
    /// The keyboard's modifiers changed.
    #[cfg(feature = "page-surface")]
    Modifiers(keyboard::Modifiers),
    /// Format › Special Effects….
    #[cfg(feature = "page-surface")]
    SpecialEffects(character_dialogs::EffectsAction),
    /// Format › Font….
    #[cfg(feature = "page-surface")]
    FontDialog(character_dialogs::FontAction),
    /// A colour button and its panel (`UI_SPEC.md` §23).
    #[cfg(feature = "page-surface")]
    Colour(colour_control::ColourAction),
    /// The Styles filter's text.
    StyleFilter(String),
    /// Format › Styles › Save as a Style Sheet….
    SaveSheet(SaveSheetAction),
    /// Format › Styles › Create Style….
    CreateStyle(style_dialogs::CreateAction),
    /// The right button pressed a Styles panel row (`Some`), or the panel's
    /// right-click menu closed (`None`).
    StyleRowContext(Option<StyleId>),
    /// An entry of the Styles panel's right-click menu, for the row the
    /// right button pressed.
    StyleRowMenu(StyleRowAction),
    RenameStyle(RenameAction),
    /// Format › Styles › Style Management….
    StyleManagement(style_dialogs::ManagementAction),
    /// Style Properties in the rail (Format › Styles › Modify Style…).
    #[cfg(feature = "page-surface")]
    StyleProperties(style_properties::StyleAction),
    /// Page Layout Properties in the rail (Format › Page Layout…).
    #[cfg(feature = "page-surface")]
    PageLayoutProperties(page_layout_properties::LayoutAction),
    #[cfg(feature = "page-surface")]
    TableProperties(table_properties::TableAction),
    /// The Fields panel and Document Properties (`UI_SPEC.md` §28).
    #[cfg(feature = "page-surface")]
    FieldsPanel(fields_panel::FieldsAction),
    /// The Notes, Bookmarks, and Footnotes panels (`UI_SPEC.md` §34).
    #[cfg(feature = "page-surface")]
    References(references_panel::ReferenceAction),
    /// The Contents & Index panel (`UI_SPEC.md` §36).
    #[cfg(feature = "page-surface")]
    Contents(contents_panel::ContentsAction),
    /// The Master Document panel (`UI_SPEC.md` §37).
    #[cfg(feature = "page-surface")]
    Master(master_panel::MasterAction),
    /// A master's file read off the interface's thread (MS3).
    #[cfg(feature = "page-surface")]
    MasterRead {
        master: u64,
        stored: String,
        result: Result<std::sync::Arc<master::Laid>, String>,
    },
    /// The Merge panel (`UI_SPEC.md` §38).
    #[cfg(feature = "page-surface")]
    Merge(merge_panel::MergeAction),
    /// The Envelope panel (MG8).
    #[cfg(feature = "page-surface")]
    Envelope(envelope_panel::EnvelopeAction),
    /// A merge data file read off the interface's thread (MG2).
    #[cfg(feature = "page-surface")]
    MergeRead {
        session_id: u64,
        result: Result<std::sync::Arc<merge::MergeData>, String>,
    },
    /// The Glossary panel (`UI_SPEC.md` §36).
    #[cfg(feature = "page-surface")]
    Glossary(glossary_panel::GlossaryAction),
    /// The glossary data file chosen (or made) in the desktop's dialog.
    #[cfg(feature = "page-surface")]
    GlossaryFile {
        path: Option<PathBuf>,
        new: bool,
    },
    #[cfg(feature = "page-surface")]
    Proofing(proofing_panel::ProofingAction),
    #[cfg(feature = "page-surface")]
    DocumentProperties(document_properties::PropertiesAction),
    /// The Print panel's actions (`UI_SPEC.md` §29).
    #[cfg(feature = "page-surface")]
    PrintPanel(print_panel::PrintAction),
    /// Insert › Floating Header/Footer….
    #[cfg(feature = "page-surface")]
    FloatingHeaderFooter(page_dialogs::FloatingAction),
    /// Insert › Page Number….
    #[cfg(feature = "page-surface")]
    PageNumber(page_dialogs::PageNumberAction),
    /// Insert › Frame… (the Create Frame dialog).
    #[cfg(feature = "page-surface")]
    CreateFrame(page_dialogs::CreateFrameAction),
    /// Insert › Table…'s dialog.
    #[cfg(feature = "page-surface")]
    InsertTable(table_commands::InsertTableAction),
    /// Insert › Bullet…'s choices and buttons (`UI_SPEC.md` §21).
    #[cfg(feature = "page-surface")]
    InsertBullet(page_dialogs::BulletAction),
    /// Outline Numbering…'s choices and buttons (`UI_SPEC.md` §22).
    #[cfg(feature = "page-surface")]
    OutlineNumbering(page_dialogs::OutlineNumberingAction),
    /// Frame Properties' Make Default for New Frames and Restore Ami Pro's
    /// Default.
    #[cfg(feature = "page-surface")]
    NewFrameDefault(NewFrameDefault),
    /// Format › Styles › Use Another Style Sheet…, and the sheet its
    /// Browse… chose.
    UseStyleSheet(style_dialogs::UseSheetAction),
    /// Insert › Image…'s file chooser answered, with its Link to file
    /// choice (P2).
    #[cfg(feature = "page-surface")]
    ImageChosen {
        session_id: u64,
        result: FileDialogResult,
        link: bool,
    },
    /// An image or copied picture files read from the clipboard for Paste
    /// (P7), and the clipboard's text, pasted when there is neither.
    #[cfg(feature = "page-surface")]
    ClipboardImage {
        session_id: u64,
        image: Option<picture_insert::ClipboardImage>,
        text: Option<String>,
    },
    /// Whether the clipboard holds an image, for Paste's state.
    #[cfg(feature = "page-surface")]
    ClipboardImageAvailability {
        has_image: bool,
        retry: bool,
    },
    /// Files dropped on the page (`text/uri-list`), or an image a browser
    /// dragged (P7).
    #[cfg(feature = "page-surface")]
    Dropped {
        session_id: u64,
        data: Vec<u8>,
        mime: String,
    },
    /// File › Export PDF… (`UI_SPEC.md` §29): the file the save dialog
    /// chose, then what the export, made off the interface's thread, wrote.
    #[cfg(feature = "page-surface")]
    ExportPdfChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    #[cfg(feature = "page-surface")]
    PdfExported {
        session_id: u64,
        result: Result<print::Exported, String>,
    },
    /// File › Export ▸ (`UI_SPEC.md` §30): the file the save dialog chose
    /// for a format, then the export made in memory off the interface's
    /// thread, then the loss dialog's answer, and the status bar's Open.
    #[cfg(feature = "page-surface")]
    ExportChosen {
        session_id: u64,
        format: amipro_exchange::Format,
        result: FileDialogResult,
    },
    #[cfg(feature = "page-surface")]
    ExportPrepared {
        session_id: u64,
        result: Result<export::Prepared, String>,
    },
    #[cfg(feature = "page-surface")]
    ExportAnyway,
    #[cfg(feature = "page-surface")]
    CancelExport,
    #[cfg(feature = "page-surface")]
    OpenExported,
    #[cfg(feature = "page-surface")]
    ExportedOpened(Result<(), String>),
    /// File › Export ▸ Folder… (`UI_SPEC.md` §30, EX6): the panel's
    /// actions, and what the export's thread reports.
    #[cfg(feature = "page-surface")]
    ExportFolder(export_folder::FolderAction),
    #[cfg(feature = "page-surface")]
    ExportFolderProgress {
        session_id: u64,
        progress: export_folder::Progress,
    },
    /// File › Import Folder… (`UI_SPEC.md` §31, IM11): the panel's
    /// actions, and what the import's thread reports.
    #[cfg(feature = "page-surface")]
    ImportFolder(import_folder::FolderAction),
    #[cfg(feature = "page-surface")]
    ImportFolderProgress {
        session_id: u64,
        progress: import_folder::Progress,
    },
    /// File › Print… (`UI_SPEC.md` §29): what the desktop's print dialog
    /// chose (`None` when cancelled), then what was sent.
    #[cfg(feature = "page-surface")]
    PrintPrepared {
        session_id: u64,
        result: Result<Option<print_portal::Prepared>, String>,
    },
    #[cfg(feature = "page-surface")]
    Printed {
        session_id: u64,
        result: Result<usize, String>,
    },
    /// Frame Properties › Picture › Save Picture As… (P7): the save dialog,
    /// then the file it chose.
    #[cfg(feature = "page-surface")]
    SavePicture,
    #[cfg(feature = "page-surface")]
    SavePictureChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    SheetChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    SheetDialogFinished {
        session_id: u64,
        description: String,
        /// With contents, as the dialog had it.
        contents: bool,
        result: FileDialogResult,
    },
    /// A change in Tools › Customize Toolbar….
    Toolbar(ToolbarEdit),
    CloseToolbarEditor,
    /// The window's new width, for the rail's overlay rule.
    WindowResized(f32),
    #[cfg_attr(
        feature = "rich-editor-prototype",
        allow(
            dead_code,
            reason = "the accepted editor is retained for non-prototype builds"
        )
    )]
    Edit(text_editor::Action),
    Command(CommandId),
    ClipboardRead {
        command: CommandId,
        session_id: u64,
        contents: Option<String>,
    },
    /// Look at the clipboard again, for Paste's state: when the window
    /// gains focus or the right button opens a menu.
    RefreshClipboard,
    /// The second look, after the first found nothing.
    RetryClipboard,
    ClipboardAvailability {
        has_text: bool,
        retry: bool,
    },
    SaveDialogFinished {
        session_id: u64,
        result: FileDialogResult,
        /// The format chosen in the dialog, when the portal reports it.
        format: Option<SaveFormat>,
    },
    ConfirmOverwrite,
    CancelOverwrite,
    /// Save As SAM after the loss check listed what Ami Pro cannot store.
    SaveSamAnyway,
    CancelSamSave,
    /// The save problem dialog: choose another name, or close it.
    SaveProblemRetry,
    SaveProblemClose,
    OpenDialogFinished(FileDialogResult),
    /// Import Options' choices (`UI_SPEC.md` §31).
    ImportOptions(import::OptionsAction),
    /// A file read and realised off the interface's thread.
    Imported(Box<Result<import::Imported, String>>),
    /// The clipboard's rich text for Paste, if it holds any (IM12).
    #[cfg(feature = "page-surface")]
    ClipboardRtf {
        session_id: u64,
        rtf: Option<import::ClipboardRtf>,
    },
    /// The Import Report's actions (IM8).
    #[cfg(feature = "page-surface")]
    ImportReport(import_report::ReportAction),
    /// Insert › File…'s file chooser.
    #[cfg(feature = "page-surface")]
    InsertFileDialogFinished {
        session_id: u64,
        result: FileDialogResult,
    },
    ApplyStyle(StyleId),
    ActivateTab(usize),
    TabActivated(segmented_button::Entity),
    TabCloseRequested(segmented_button::Entity),
    ConfirmCloseDiscard,
    SaveAndClose,
    CancelClose,
    CloseAbout,
    HelpDrawer(help_drawers::HelpAction),
    Preferences(preferences_dialog::PreferencesAction),
    #[cfg(feature = "page-surface")]
    ParagraphProperties(paragraph_properties::ParagraphAction),
    NewDocumentDialog(new_document_dialog::NewAction),
    #[cfg(feature = "page-surface")]
    FastFormat(fast_format::FastAction),
    /// File › 1 NAME…: opens a recent file.
    OpenRecent(usize),
    /// Revert to Saved's question answered: revert (true) or not.
    RevertToSaved(bool),
    /// The password box: File › Open of a protected file, File › Password….
    Password(password_dialog::PasswordAction),
    /// The Outline navigator in the rail (OB.6), and the plain arrow keys
    /// and Enter no widget took, which it acts on while it holds the keys.
    #[cfg(feature = "page-surface")]
    OutlineNavigator(outline_navigator::NavigatorAction),
    /// The Chart panel in the rail (DO.8).
    #[cfg(feature = "page-surface")]
    Chart(chart_panel::ChartAction),
    /// The Equation editor's rail panel and source card (DO.9).
    #[cfg(feature = "page-surface")]
    Equation(equation_panel::EquationAction),
    /// The clipboard's text for a LaTeX equation (`None`: it held none).
    #[cfg(feature = "page-surface")]
    EquationClipboard {
        session_id: u64,
        text: Option<String>,
        create: bool,
    },
    /// Import Equation… or Save As Equation…: the file the dialog chose.
    #[cfg(feature = "page-surface")]
    EquationFile {
        session_id: u64,
        path: Option<std::path::PathBuf>,
        save: bool,
    },
    /// The clipboard's text for a new chart (`None`: it held none).
    #[cfg(feature = "page-surface")]
    ChartClipboard {
        session_id: u64,
        text: Option<String>,
    },
    /// Save Chart as Image…: the file the dialog chose.
    #[cfg(feature = "page-surface")]
    ChartImageChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    /// The Links panel in the rail (DO.3).
    #[cfg(feature = "page-surface")]
    Links(links_panel::LinksAction),
    /// The Updates panel in the rail (P6.2).
    #[cfg(feature = "page-surface")]
    Updates(updates_panel::UpdatesAction),
    /// The Recover documents panel and the autosave (P6.3).
    #[cfg(feature = "page-surface")]
    Recovery(recovery_panel::RecoveryAction),
    /// The Help panel in the rail (P6.4).
    #[cfg(feature = "page-surface")]
    HelpPanel(help_panel::PanelAction),
    /// The Paste Special panel in the rail (DO.4).
    #[cfg(feature = "page-surface")]
    PasteSpecial(paste_special::PasteSpecialAction),
    /// One look at the clipboard for a Paste Special format: `slot` of
    /// [`paste_special::CANDIDATES`], with what was found.
    #[cfg(feature = "page-surface")]
    PasteSpecialFound {
        session_id: u64,
        slot: usize,
        data: Option<(Vec<u8>, String)>,
    },
    /// The first look found nothing: the clipboard is looked at again.
    #[cfg(feature = "page-surface")]
    PasteSpecialRetry(u64),
    /// The Sort panel in the rail (TD.8).
    #[cfg(feature = "page-surface")]
    Sort(sort_panel::SortAction),
    #[cfg(feature = "page-surface")]
    NavigatorKey(keyboard::key::Named),
    #[cfg(feature = "page-surface")]
    SpecialCharacter(special_character::CharacterAction),
    /// The status bar's Face and Point Size lists (CM12): entry chosen.
    #[cfg(feature = "page-surface")]
    StatusFace(usize),
    #[cfg(feature = "page-surface")]
    StatusSize(usize),
    /// Enter in the Styles filter: the first style listed (CK7).
    StyleFilterSubmit,
    Scrolled(cosmic::iced::widget::scrollable::Viewport),
    #[cfg(feature = "rich-editor-prototype")]
    Prototype(PrototypeAction),
    #[cfg(feature = "page-surface")]
    Page(PageAction),
    #[cfg(feature = "page-surface")]
    Frame(FrameAction),
    /// Draw mode's events from the page surface (`UI_SPEC.md` §33).
    #[cfg(feature = "page-surface")]
    Draw(draw_mode::Event),
    #[cfg(feature = "page-surface")]
    DrawTool(draw_mode::Tool),
    #[cfg(feature = "page-surface")]
    DrawStyle(amipro_editing::drawing::StyleChange),
    #[cfg(feature = "page-surface")]
    DrawProperties(drawing_properties::PropertyAction),
    /// What the clipboard held for Paste in Draw mode (`UI_SPEC.md` §33).
    #[cfg(feature = "page-surface")]
    DrawPaste(Option<draw_files::DrawingPaste>),
    /// Draw › Import Drawing…'s and Save Drawing As…'s files.
    #[cfg(feature = "page-surface")]
    DrawImportChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    #[cfg(feature = "page-surface")]
    DrawSaveChosen {
        session_id: u64,
        result: FileDialogResult,
    },
    #[cfg(feature = "page-surface")]
    ConfirmFrameDelete,
    #[cfg(feature = "page-surface")]
    CancelFrameDelete,
    #[cfg(feature = "page-surface")]
    InspectorInput(frame_inspector::Field, String),
    #[cfg(feature = "page-surface")]
    InspectorSubmit(frame_inspector::Field),
    #[cfg(feature = "page-surface")]
    InspectorChoice(frame_inspector::Choice),
    /// A slider in the Image section moved (DO.5): the picture shows it at
    /// once, and the whole drag is one undo step.
    #[cfg(feature = "page-surface")]
    ImageSlide(frame_inspector::Field, u8),
    /// The slider was let go.
    #[cfg(feature = "page-surface")]
    ImageSlideEnd,
    #[cfg(feature = "page-surface")]
    ZoomTo(u16),
    #[cfg(feature = "page-surface")]
    Ruler(ruler_view::RulerEvent),
    #[cfg(feature = "page-surface")]
    RulerMenu(ruler_view::RulerMenu),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MenuCommand {
    Command(CommandId),
    ActivateTab(usize),
    OpenRecent(usize),
    /// Applies a style: the page menu's Paragraph Style list.
    #[cfg(feature = "page-surface")]
    ApplyStyle(StyleId),
    /// The Styles panel's right-click menu, for the row pressed.
    StyleRowMenu(StyleRowAction),
    /// The ruler's right-click menus, for what the right button pressed.
    #[cfg(feature = "page-surface")]
    Ruler(ruler_view::RulerMenu),
    /// The find bar's options and recent searches.
    #[cfg(feature = "page-surface")]
    Find(find_bar::FindMenu),
    /// The right-click menu's spelling and thesaurus entries
    /// (`UI_SPEC.md` §35).
    #[cfg(feature = "page-surface")]
    Proofing(proofing_panel::ContextEntry),
}

impl MenuAction for MenuCommand {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::Command(command) => Message::Command(*command),
            Self::ActivateTab(index) => Message::ActivateTab(*index),
            Self::OpenRecent(index) => Message::OpenRecent(*index),
            #[cfg(feature = "page-surface")]
            Self::ApplyStyle(style) => Message::ApplyStyle(*style),
            Self::StyleRowMenu(action) => Message::StyleRowMenu(*action),
            #[cfg(feature = "page-surface")]
            Self::Ruler(action) => Message::RulerMenu(*action),
            #[cfg(feature = "page-surface")]
            Self::Find(action) => Message::Find(find_bar::FindAction::Menu(*action)),
            #[cfg(feature = "page-surface")]
            Self::Proofing(entry) => {
                Message::Proofing(proofing_panel::ProofingAction::Context(*entry))
            }
        }
    }
}

/// What a Styles panel row's context menu does (`UI_SPEC.md` §14).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StyleRowAction {
    Apply,
    Modify,
    CreateBasedOn,
    Rename,
    /// Style Management's Remove: a changed sheet style takes the sheet's
    /// version again (ST1).
    Remove,
}

#[derive(Clone, Debug)]
enum FileDialogResult {
    Selected(PathBuf),
    Cancelled,
    Failed(String),
}

impl cosmic::Application for AmiProApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn on_app_exit(&mut self) -> Option<Message> {
        self.sessions
            .iter()
            .any(|session| session.dirty)
            .then_some(Message::Command(CommandId::Quit))
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Message>> {
        if self.show_about {
            return Some(
                context_drawer::context_drawer(
                    widget::about::about(&self.about, |_| Message::CloseAbout),
                    Message::CloseAbout,
                )
                .title("About AmiPro-Linux"),
            );
        }
        self.help_drawer.as_ref().map(|drawer| {
            context_drawer::context_drawer(
                help_drawers::view(drawer, self.sessions[self.active_session].id),
                Message::HelpDrawer(help_drawers::HelpAction::Close),
            )
            .title(drawer.title())
        })
    }

    fn init(core: cosmic::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Message>>) {
        let document = new_document();
        let session = DocumentSession::new(
            1,
            document,
            DocumentLocation::Untitled,
            "Ready — native format; SAM/STY import is read-only".into(),
        );
        let (settings, preferences, styles_visible) =
            cosmic::cosmic_config::Config::new(APP_ID, SETTINGS_VERSION)
                .ok()
                .map_or((None, Preferences::default(), None), |config| {
                    let (settings, preferences, styles_visible) = Settings::load(config);
                    (Some(settings), preferences, styles_visible)
                });
        let mut application = Self {
            core,
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling {
                thesaurus_folders: spelling::thesaurus_folders(),
                ..spelling::Spelling::new(spelling::available(), spelling::user_dictionary_folder())
            },
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            // Open on a desktop-width window to teach the style workflow;
            // a narrow window keeps the overlay closed (`UI_SPEC.md` §5).
            // Later starts keep the user's choice (gap R5).
            styles_visible: styles_visible.unwrap_or(true),
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences,
            settings,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::RealBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default()
                .name("AmiPro-Linux")
                .version(env!("CARGO_PKG_VERSION"))
                .comments(
                    "A native COSMIC word processor with Lotus Ami Pro document compatibility",
                )
                .license("MPL-2.0"),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: true,
        };
        application.sync_tab_model();
        #[cfg(feature = "page-surface")]
        if let Some(root) = recovery::default_root() {
            application.recovery = recovery_panel::RecoveryState::with_store(recovery::Store::new(
                root,
                Arc::new(recovery::SystemProbe),
            ));
            application.recovery_at_start();
        }
        #[cfg(feature = "page-surface")]
        let startup_check = application.startup_update_check();
        #[cfg(not(feature = "page-surface"))]
        let startup_check = Task::none();
        let tasks = Task::batch([
            application.update_title(),
            Self::clipboard_availability_task(true),
            startup_check,
        ]);
        (application, tasks)
    }

    fn view(&self) -> Element<'_, Message> {
        let menu = application_menu(self);

        let style_entries: Vec<_> = self
            .document
            .styles()
            .values()
            .map(|style| (style.id, style.name.clone()))
            .collect();
        let selected_style = self.selected_style();
        let selected_style_index = selected_style
            .and_then(|style| style_entries.iter().position(|entry| entry.0 == style));
        let style_names: Vec<_> = style_entries.iter().map(|entry| entry.1.clone()).collect();
        let style_ids: Vec<_> = style_entries.iter().map(|entry| entry.0).collect();
        let action_bar_state = ActionBarState {
            enabled_commands: COMMANDS
                .iter()
                .map(|command| (command.id, self.command_enabled(command.id)))
                .collect(),
            style_names: style_names.clone(),
            style_ids: style_ids.clone(),
            selected_style_index,
            bold: self.character_state(CharacterAttribute::Bold),
            italic: self.character_state(CharacterAttribute::Italic),
            underline: self.character_state(CharacterAttribute::Underline),
            alignment: self.selected_alignment(),
            list: self.selected_list(),
            #[cfg(feature = "page-surface")]
            fast_format: self.fast_format.is_some(),
            #[cfg(not(feature = "page-surface"))]
            fast_format: false,
            tabs_returns: self.preferences.tabs_returns,
            styles_visible: self.rail_open(),
            toolbar: self.preferences.toolbar.clone(),
        };
        // `responsive` defaults to filling both axes; without a fixed height the
        // action bar would share the window's vertical space with the document.
        let toolbar: Element<'_, Message> = cosmic::iced::widget::responsive(move |available| {
            action_bar_view(action_bar_state.clone(), available.width)
        })
        .height(ACTION_BAR_HEIGHT)
        .into();

        let tabs = tab_strip_view(&self.tab_model);

        let styles = self.styles_rail(selected_style);

        let page_setup = self.document.page_setup();
        #[cfg(feature = "rich-editor-prototype")]
        let session_id = self.id;
        let document = &self.document;
        #[cfg(not(feature = "rich-editor-prototype"))]
        let editor = &self.editor;
        #[cfg(feature = "rich-editor-prototype")]
        let prototype_selection = self.prototype_selection;
        #[cfg(feature = "rich-editor-prototype")]
        let rich_layout_generation = self.rich_layout_generation;
        let workspace: Element<'_, Message> = cosmic::iced::widget::responsive(move |available| {
            let geometry = PageViewGeometry::fit_width(&page_setup, available.width);
            #[cfg(not(feature = "rich-editor-prototype"))]
            let editor_size = 12.0 * SCREEN_PIXELS_PER_POINT * geometry.display_scale;
            #[cfg(feature = "rich-editor-prototype")]
            let page_content: Element<'_, Message> = rich_editor_prototype::surface_with_commands(
                document,
                rich_editor_prototype::LayoutEpoch::new(session_id, rich_layout_generation),
                geometry.display_scale,
                prototype_selection,
                Message::Prototype,
                Message::Command,
            )
            .into();
            #[cfg(not(feature = "rich-editor-prototype"))]
            let page_content: Element<'_, Message> = standard_editor(document, editor, editor_size);
            let page = widget::container(page_content)
                .padding([
                    geometry.margin_top,
                    geometry.margin_right,
                    geometry.margin_bottom,
                    geometry.margin_left,
                ])
                .width(geometry.page_width)
                .height(geometry.page_height)
                .class(cosmic::theme::Container::Card);
            let ruler = ruler_view(geometry);
            let page_stack = widget::column::with_children(vec![ruler, page.into()])
                .spacing(8)
                .width(geometry.page_width);
            let page_space = widget::container(page_stack)
                .padding(geometry.workspace_padding())
                .center_x(Length::Fill)
                .width(geometry.workspace_width(available.width));
            widget::scrollable(page_space)
                .id(workspace_scroll_id())
                .on_scroll(Message::Scrolled)
                .direction(Direction::Both {
                    vertical: Scrollbar::default(),
                    horizontal: Scrollbar::default(),
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
        .into();
        #[cfg(feature = "page-surface")]
        let workspace = if self.page_surface {
            self.page_workspace()
        } else {
            workspace
        };

        let (page, pages) = self.page_position();
        #[cfg(feature = "page-surface")]
        let page = if self.page_surface {
            page_surface::caret_page(&self.document, self.pages.layout(), self.page_selection)
        } else {
            page
        };
        let message = self.status.as_str();
        #[cfg(feature = "page-surface")]
        let message = if self.page_surface {
            page_surface::cell_frame_note(&self.document, self.page_selection)
                .or_else(|| {
                    page_surface::hidden_text_note(
                        &self.document,
                        self.pages.layout(),
                        self.page_selection,
                    )
                })
                .unwrap_or(message)
        } else {
            message
        };
        // The field at the caret is named first (`UI_SPEC.md` §28).
        #[cfg(feature = "page-surface")]
        let named = self
            .field_at_caret()
            .map(|field| field_commands::field_name(&field))
            .map(|name| {
                if message.is_empty() {
                    name
                } else {
                    format!("{name} — {message}")
                }
            });
        #[cfg(feature = "page-surface")]
        let message = named.as_deref().unwrap_or(message);
        let status_model = status_bar_model(
            page,
            pages,
            message,
            matches!(self.location, DocumentLocation::LegacySource(_)),
        );
        let selected_words = self.selected_text().map(|text| chrome::count_words(&text));
        let page_status: Element<'_, Message> =
            widget::text::caption(status_model.page.clone()).into();
        // Ami Pro's page arrows and Page Status button (user's guide
        // chapter 1; user, 2026-10-12): previous page, the page number
        // that opens Go To, next page.
        #[cfg(feature = "page-surface")]
        let page_status: Element<'_, Message> = if self.page_surface {
            let arrow = |command: CommandId, icon: &'static str| -> Element<'_, Message> {
                widget::button::icon(widget::icon::from_name(icon))
                    .extra_small()
                    .padding(2)
                    .name(command::metadata(command).label)
                    .tooltip(command_tooltip(command))
                    .on_press_maybe(
                        self.command_enabled(command)
                            .then_some(Message::Command(command)),
                    )
                    .into()
            };
            widget::row::with_children(vec![
                arrow(CommandId::PreviousPage, "go-up-symbolic"),
                widget::button::text(status_model.page.clone())
                    .padding([0, 4])
                    .tooltip(command_tooltip(CommandId::GoToPage))
                    .on_press(Message::Command(CommandId::GoToPage))
                    .into(),
                arrow(CommandId::NextPage, "go-down-symbolic"),
            ])
            .spacing(2)
            .align_y(cosmic::iced::Alignment::Center)
            .into()
        } else {
            page_status
        };
        let mut status_children: Vec<Element<'_, Message>> = vec![
            page_status,
            widget::divider::vertical::default().height(16).into(),
            widget::text::caption(chrome::word_count_label(self.words.total(), selected_words))
                .into(),
            widget::divider::vertical::default().height(16).into(),
        ];
        // The caret cell's address while the caret is in a table (TN9).
        #[cfg(feature = "page-surface")]
        if self.page_surface
            && self.frame_selection.is_none()
            && let Some((_, row, column)) =
                table_editing::caret_cell(&self.document, self.page_selection)
        {
            status_children.push(
                widget::text::caption(format!(
                    "Cell {}",
                    amipro_document::formula::CellRef::new(column, row).name()
                ))
                .into(),
            );
            status_children.push(widget::divider::vertical::default().height(16).into());
        }
        status_children.extend::<Vec<Element<'_, Message>>>(vec![
            widget::text::caption(status_model.message.unwrap_or_default())
                .width(Length::Fill)
                .wrapping(Wrapping::None)
                .ellipsize(Ellipsize::Middle(EllipsizeHeightLimit::Lines(1)))
                .into(),
        ]);
        // The file an export wrote, to open in the desktop's program for it
        // (`UI_SPEC.md` §30, EX10).
        #[cfg(feature = "page-surface")]
        if self.exported.is_some() {
            status_children.push(
                widget::button::text("Open")
                    .padding([0, 4])
                    .tooltip("Open the exported file")
                    .on_press(Message::OpenExported)
                    .into(),
            );
        }
        // What the import changed, opening the Import Report (IM8).
        #[cfg(feature = "page-surface")]
        if let Some(report) = &self.import_report
            && self.page_surface
            && !report.findings.is_empty()
        {
            let count = report.findings.len();
            status_children.push(
                widget::button::text(if count == 1 {
                    "1 import note".to_owned()
                } else {
                    format!("{count} import notes")
                })
                .padding([0, 4])
                .tooltip("Show the Import Report")
                .on_press(Message::ImportReport(import_report::ReportAction::Show))
                .into(),
            );
        }
        if let Some(source_state) = status_model.source_state {
            status_children.push(widget::divider::vertical::default().height(16).into());
            status_children.push(widget::text::caption(source_state).into());
        }
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            status_children.push(widget::divider::vertical::default().height(16).into());
            status_children.extend(self.font_controls());
            status_children.push(widget::divider::vertical::default().height(16).into());
            status_children.extend(self.zoom_controls());
        }
        let status = widget::container(
            widget::row::with_children(status_children)
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center),
        )
        .padding([4, CHROME_HORIZONTAL_PADDING])
        .height(STATUS_BAR_HEIGHT)
        .width(Length::Fill);

        // One rail: Frame Properties while frames are selected, else Style
        // Properties while it shows (wider), else Styles.
        let rail_width = RAIL_WIDTH;
        #[cfg(feature = "page-surface")]
        let (styles, rail_width) = match self.rail_panel() {
            RailPanel::Drawing => match self.drawing_properties_view() {
                Some(panel) => (panel, drawing_properties::WIDTH),
                None => (styles, rail_width),
            },
            RailPanel::Frame => match self.frame_inspector_view() {
                Some(inspector) => (inspector, rail_width),
                None => (styles, rail_width),
            },
            RailPanel::StyleProperties => match &self.style_properties {
                Some(state) => (
                    style_properties::panel(state, self.document.styles(), self.colour_context()),
                    style_properties::WIDTH,
                ),
                None => (styles, rail_width),
            },
            RailPanel::PageLayout => match &self.page_layout_properties {
                Some(state) => (
                    page_layout_properties::panel(state, &self.document, self.colour_context()),
                    page_layout_properties::WIDTH,
                ),
                None => (styles, rail_width),
            },
            RailPanel::Paragraph => (
                paragraph_properties::panel(
                    self.paragraph_properties
                        .as_ref()
                        .expect("Paragraph Properties shows"),
                ),
                paragraph_properties::WIDTH,
            ),
            RailPanel::Table => self.table_properties_view().unwrap_or((styles, rail_width)),
            RailPanel::Fields => (self.fields_panel_view(), fields_panel::WIDTH),
            RailPanel::References => (self.references_view(), references_panel::WIDTH),
            RailPanel::Contents => (self.contents_view(), contents_panel::WIDTH),
            RailPanel::Master => (self.master_view(), master_panel::WIDTH),
            RailPanel::Envelope => (self.envelope_view(), envelope_panel::WIDTH),
            RailPanel::Merge => (self.merge_view(), merge_panel::WIDTH),
            RailPanel::Glossary => (self.glossary_view(), glossary_panel::WIDTH),
            RailPanel::Proofing => (self.proofing_view(), proofing_panel::WIDTH),
            RailPanel::DocumentProperties => {
                (self.document_properties_view(), document_properties::WIDTH)
            }
            RailPanel::ImportFolder => (
                import_folder::panel(
                    self.import_folder
                        .as_ref()
                        .expect("the Import Folder panel shows"),
                ),
                import_folder::WIDTH,
            ),
            RailPanel::ExportFolder => (
                export_folder::panel(
                    self.export_folder
                        .as_ref()
                        .expect("the Export Folder panel shows"),
                ),
                export_folder::WIDTH,
            ),
            RailPanel::ImportReport => (
                import_report::panel(
                    self.import_report
                        .as_ref()
                        .expect("the Import Report shows"),
                ),
                import_report::WIDTH,
            ),
            RailPanel::Print => (
                print_panel::panel(
                    self.print_panel.as_ref().expect("the Print panel shows"),
                    self.print_options,
                    self.preferences.pdf_additions,
                    amipro_editing::master::is_master(&self.document)
                        .then_some(!self.print_this_only),
                ),
                print_panel::WIDTH,
            ),
            RailPanel::PasteSpecial => (
                paste_special::panel(
                    self.paste_special
                        .as_ref()
                        .expect("the Paste Special panel shows"),
                ),
                paste_special::WIDTH,
            ),
            RailPanel::Equation => (
                equation_panel::panel(
                    self.equation_panel
                        .as_ref()
                        .expect("the Equation panel shows"),
                ),
                equation_panel::WIDTH,
            ),
            RailPanel::Chart => (
                chart_panel::panel(
                    self.chart_panel.as_ref().expect("the Chart panel shows"),
                    chart_panel::Context {
                        colours: self.colour_context(),
                        in_table: table_editing::caret_cell(&self.document, self.page_selection)
                            .is_some(),
                    },
                ),
                chart_panel::WIDTH,
            ),
            RailPanel::Links => {
                let rows = links_panel::rows(&self.document);
                (
                    links_panel::panel(
                        self.links_panel.as_ref().expect("the Links panel shows"),
                        &rows,
                    ),
                    links_panel::WIDTH,
                )
            }
            RailPanel::Recovery => (recovery_panel::panel(&self.recovery), recovery_panel::WIDTH),
            RailPanel::Help => (help_panel::panel(&self.help), help_panel::WIDTH),
            RailPanel::Updates => (
                updates_panel::panel(&self.updates, &updates::installed_version().to_string()),
                updates_panel::WIDTH,
            ),
            RailPanel::Outline => {
                let entries = outline_navigator::entries(&self.document, &self.outline_state);
                let focus = self.page_selection.focus;
                let current = (focus.flow == self.document.main_flow_id())
                    .then(|| outline_navigator::current(&entries, focus.paragraph))
                    .flatten();
                (
                    outline_navigator::panel(
                        self.outline_navigator
                            .as_ref()
                            .expect("the navigator shows"),
                        &entries,
                        current,
                    ),
                    outline_navigator::WIDTH,
                )
            }
            RailPanel::Sort => {
                let (_, shown) = self.sort_scope();
                (
                    sort_panel::panel(
                        self.sort_panel.as_ref().expect("the Sort panel shows"),
                        &shown,
                    ),
                    sort_panel::WIDTH,
                )
            }
            RailPanel::Styles => (styles, rail_width),
        };
        // Docked beside the page on wide windows; over the workspace on
        // narrow ones, so that the page is never squeezed (`UI_SPEC.md` §5).
        // The workspace keeps one place in the tree whether the rail is
        // closed, docked, or over it: the tree diff rebuilds a widget that
        // moves, and the pages lost their scroll each time the rail opened
        // or closed (user, 2026-10-13). An empty slot must not be 0 pixels
        // wide in its size hint: a row or stack drops such a child.
        let empty_slot = || -> Element<'_, Message> { widget::space().height(Length::Fill).into() };
        let (docked, floating) = match (self.rail_open(), self.rail_overlays()) {
            (false, _) => (empty_slot(), empty_slot()),
            (true, true) => (
                empty_slot(),
                widget::row::with_children(vec![
                    rail_surface(styles, rail_width, true),
                    widget::space::horizontal().into(),
                ])
                .into(),
            ),
            (true, false) => (rail_surface(styles, rail_width, false), empty_slot()),
        };
        let document_row = widget::row::with_children(vec![
            docked,
            cosmic::iced::widget::stack![workspace, floating]
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ]);

        // Show Clean Screen (CM9): the chrome's places stay, empty, so the
        // pages keep their place in the tree and their scroll (`CLAUDE.md`).
        if self.clean_screen {
            let empty = || -> Element<'_, Message> {
                widget::space()
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .into()
            };
            let back = widget::row::with_children(vec![
                widget::space::horizontal().into(),
                widget::button::text("Return")
                    .tooltip("Leave Clean Screen (Esc)")
                    .on_press(Message::Command(CommandId::FullScreen))
                    .into(),
            ])
            .padding([0, CHROME_HORIZONTAL_PADDING]);
            let mut rows = vec![back.into()];
            if self.preferences.toolbar_visible {
                rows.push(empty());
            }
            rows.push(empty());
            #[cfg(feature = "page-surface")]
            if self.page_surface {
                rows.push(empty());
            }
            rows.extend([document_row.height(Length::Fill).into(), empty()]);
            return widget::column::with_children(rows).into();
        }
        // No outer frame: chrome rows share one left edge and the workspace
        // fills everything between the tabs and the status bar.
        let mut rows = vec![
            widget::container(menu)
                .padding([0, CHROME_HORIZONTAL_PADDING])
                .width(Length::Fill)
                .into(),
        ];
        // In Draw mode the draw bar takes the toolbar's place, shown even
        // with the toolbar hidden (`UI_SPEC.md` §33, DW3).
        #[cfg(feature = "page-surface")]
        let toolbar = if self.drawing_open() {
            Some(self.draw_bar_view())
        } else if self.outline.is_some() {
            // Outline mode's bar takes the toolbar's place (OB.4).
            Some(self.outline_bar_view())
        } else {
            self.preferences.toolbar_visible.then_some(toolbar)
        };
        #[cfg(not(feature = "page-surface"))]
        let toolbar = self.preferences.toolbar_visible.then_some(toolbar);
        if let Some(toolbar) = toolbar {
            rows.push(toolbar);
        }
        rows.push(tabs);
        // The find bar between the tabs and the ruler (`UI_SPEC.md` §27);
        // closed, its slot shrinks to nothing but stays, so the document row
        // keeps its place in the tree and the pages their scroll
        // (`CLAUDE.md`). It comes before the pages, so its fields take the
        // keys first.
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            rows.push(self.find_bar_view().unwrap_or_else(|| {
                widget::space()
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .into()
            }));
        }
        // View › Status Bar: hidden, the slot shrinks to nothing (it stays so
        // the tree keeps its place); a failure message brings the bar back.
        let status: Element<'_, Message> = if self.status_bar_shown() {
            status.into()
        } else {
            widget::space()
                .width(Length::Fill)
                .height(Length::Shrink)
                .into()
        };
        rows.extend([document_row.height(Length::Fill).into(), status]);
        widget::column::with_children(rows).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        #[cfg(feature = "page-surface")]
        let autosave = self
            .recovery_interval()
            .map_or_else(Subscription::none, |every| {
                cosmic::iced::time::every(every)
                    .map(|_| Message::Recovery(recovery_panel::RecoveryAction::Tick))
            });
        #[cfg(not(feature = "page-surface"))]
        let autosave = Subscription::none();
        Subscription::batch([
            autosave,
            event::listen_with(|event, status, _window| {
                if matches!(event, Event::Window(cosmic::iced::window::Event::Focused)) {
                    return Some(Message::RefreshClipboard);
                }
                if let Event::Window(cosmic::iced::window::Event::Resized(size)) = event {
                    return Some(Message::WindowResized(size.width));
                }
                // The find bar's keys come whoever took them: Esc closes it from
                // its fields, and Shift+Enter finds backwards.
                #[cfg(feature = "page-surface")]
                if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = &event {
                    return Some(Message::Modifiers(*modifiers));
                }
                // Esc comes whoever took it (a dialog's text field does).
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                }) = &event
                {
                    return Some(Message::Escape);
                }
                if status == event::Status::Captured {
                    return None;
                }
                let Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    physical_key,
                    modifiers,
                    ..
                }) = event
                else {
                    return None;
                };
                let command = command::command_for_shortcut(modifiers, &key, Some(&physical_key))
                    .map(Message::Command);
                // Plain arrows and Enter no widget took: the Outline navigator's
                // while it holds the keys (OB.6).
                #[cfg(feature = "page-surface")]
                if command.is_none()
                    && modifiers.is_empty()
                    && let keyboard::Key::Named(
                        named @ (keyboard::key::Named::ArrowUp
                        | keyboard::key::Named::ArrowDown
                        | keyboard::key::Named::ArrowLeft
                        | keyboard::key::Named::ArrowRight),
                    ) = key
                {
                    return Some(Message::NavigatorKey(named));
                }
                command
            }),
        ])
    }

    fn dialog(&self) -> Option<Element<'_, Message>> {
        if let Some(dialog) = &self.import_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = self.field_prompt_dialog() {
            return Some(dialog);
        }
        if let Some(index) = self.pending_close {
            let label = self.sessions.get(index)?.tab_label();
            return Some(
                widget::dialog()
                    .title("Discard unsaved changes?")
                    .body(format!("{label} has unsaved changes."))
                    .primary_action(
                        widget::button::destructive("Discard")
                            .on_press(Message::ConfirmCloseDiscard),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel").on_press(Message::CancelClose),
                    )
                    .tertiary_action(
                        widget::button::suggested("Save").on_press(Message::SaveAndClose),
                    )
                    .into(),
            );
        }
        if let Some((style, name)) = &self.rename_style {
            let problem =
                amipro_editing::style_name_problem(self.document.styles(), name, Some(*style));
            let old = self
                .document
                .styles()
                .get(style)
                .map_or_else(String::new, |style| style.name.clone());
            let mut controls: Vec<Element<'_, Message>> = vec![
                widget::text_input("Name", name.clone())
                    .on_input(|name| Message::RenameStyle(RenameAction::Name(name)))
                    .on_submit_maybe(
                        problem
                            .is_none()
                            .then_some(|_| Message::RenameStyle(RenameAction::Rename)),
                    )
                    .width(Length::Fill)
                    .into(),
            ];
            if let Some(problem) = problem.filter(|_| *name != old) {
                controls.push(widget::text::caption(style_dialogs::sentence(problem)).into());
            }
            return Some(
                widget::dialog()
                    .title("Rename Style")
                    .body(format!(
                        "A new name for {old}. Paragraphs in the style keep it under its new name."
                    ))
                    .control(widget::column::with_children(controls).spacing(8))
                    .primary_action(
                        widget::button::suggested("Rename").on_press_maybe(
                            problem
                                .is_none()
                                .then_some(Message::RenameStyle(RenameAction::Rename)),
                        ),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .on_press(Message::RenameStyle(RenameAction::Cancel)),
                    )
                    .into(),
            );
        }
        if let Some(state) = &self.style_management {
            let sheet = self.document.style_sheet().map(|link| link.name.as_str());
            return Some(style_dialogs::style_management_dialog(
                state,
                self.document.styles(),
                sheet,
            ));
        }
        if let Some(state) = &self.use_sheet {
            let current = self.document.style_sheet().map(|link| link.name.as_str());
            return Some(style_dialogs::use_style_sheet_dialog(state, current));
        }
        if let Some(state) = &self.create_style {
            let problem =
                amipro_editing::style_name_problem(self.document.styles(), &state.name, None);
            return Some(style_dialogs::create_style_dialog(
                state,
                problem,
                cfg!(feature = "page-surface"),
            ));
        }
        if let Some(description) = &self.save_sheet {
            return Some(
                widget::dialog()
                    .title("Save as a Style Sheet")
                    .body(
                        "Writes the styles and the page layout to a new Ami Pro style sheet \
                         (.sty). With contents, the document's text, frames, and pictures \
                         go in too, and a new document made from the sheet starts with them.",
                    )
                    .control(
                        widget::column::with_children(vec![
                            widget::text::body("Description").into(),
                            widget::text_input("Description", description.clone())
                                .on_input(|text| {
                                    Message::SaveSheet(SaveSheetAction::Description(text))
                                })
                                .on_submit(|_| Message::SaveSheet(SaveSheetAction::Choose))
                                .width(Length::Fill)
                                .into(),
                            widget::checkbox(self.save_sheet_contents)
                                .label("With contents")
                                .on_toggle(|on| Message::SaveSheet(SaveSheetAction::Contents(on)))
                                .into(),
                        ])
                        .spacing(8),
                    )
                    .primary_action(
                        widget::button::suggested("Save…")
                            .on_press(Message::SaveSheet(SaveSheetAction::Choose)),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .on_press(Message::SaveSheet(SaveSheetAction::Cancel)),
                    )
                    .into(),
            );
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.floating_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.page_number_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.create_frame_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.insert_table_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.bullet_dialog {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(dialog) = &self.outline_numbering {
            return Some(dialog.view());
        }
        #[cfg(feature = "page-surface")]
        if let Some(go_to) = &self.go_to {
            let count = self.page_count();
            let mut controls: Vec<Element<'_, Message>> = vec![
                widget::text_input("Page", go_to.page.clone())
                    .on_input(|text| Message::GoTo(GoToAction::Input(text)))
                    .on_submit(|_| Message::GoTo(GoToAction::Go))
                    .width(Length::Fill)
                    .into(),
            ];
            if let Some(error) = &go_to.error {
                controls.push(widget::text::caption(error.clone()).into());
            }
            controls.push(
                widget::row::with_children(vec![
                    widget::button::standard("First page")
                        .on_press(Message::GoTo(GoToAction::First))
                        .into(),
                    widget::button::standard("Last page")
                        .on_press(Message::GoTo(GoToAction::Last))
                        .into(),
                    widget::button::standard("Next frame")
                        .on_press_maybe(
                            (!self.document.frame_stack().is_empty())
                                .then_some(Message::GoTo(GoToAction::NextFrame)),
                        )
                        .into(),
                ])
                .spacing(8)
                .into(),
            );
            controls.push(
                widget::row::with_children(vec![
                    widget::dropdown(
                        reference_commands::GO_TO_ITEMS.to_vec(),
                        Some(go_to.item),
                        |item| Message::GoTo(GoToAction::Item(item)),
                    )
                    .width(Length::Fill)
                    .into(),
                    widget::button::standard("Previous")
                        .on_press(Message::GoTo(GoToAction::Step(false)))
                        .into(),
                    widget::button::standard("Next")
                        .on_press(Message::GoTo(GoToAction::Step(true)))
                        .into(),
                ])
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
            );
            return Some(
                widget::dialog()
                    .title("Go To")
                    .body(format!("Page number, 1 to {count}"))
                    .control(widget::column::with_children(controls).spacing(8))
                    .primary_action(
                        widget::button::suggested("Go").on_press(Message::GoTo(GoToAction::Go)),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel")
                            .on_press(Message::GoTo(GoToAction::Cancel)),
                    )
                    .into(),
            );
        }
        #[cfg(feature = "page-surface")]
        if self.pending_frame_delete {
            let frames = self
                .frame_selection
                .as_ref()
                .map_or(0, |selection| selection.members(&self.document).len());
            return Some(
                widget::dialog()
                    .title(if frames > 1 {
                        "Delete the selected frames?"
                    } else {
                        "Delete the selected frame?"
                    })
                    .body("Its contents are deleted with it. Undo brings them back.")
                    .primary_action(
                        widget::button::destructive("Delete").on_press(Message::ConfirmFrameDelete),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel").on_press(Message::CancelFrameDelete),
                    )
                    .into(),
            );
        }
        #[cfg(feature = "page-surface")]
        if let Some(effects) = &self.special_effects {
            return Some(character_dialogs::special_effects_dialog(effects));
        }
        #[cfg(feature = "page-surface")]
        if let Some(font) = &self.font_dialog {
            return Some(character_dialogs::font_dialog(font, self.colour_context()));
        }
        #[cfg(feature = "page-surface")]
        if self.special_character {
            return Some(special_character::dialog(
                &self.preferences.recent_characters,
            ));
        }
        if self.revert_pending {
            return Some(revert_saved::dialog(&self.tab_label()));
        }
        if let Some(dialog) = &self.password_dialog {
            return Some(password_dialog::dialog(dialog));
        }
        #[cfg(feature = "page-surface")]
        if let Some(ask) = self.fast_format_ask {
            return Some(fast_format::ask_dialog(ask));
        }
        if let Some(state) = &self.new_document_dialog {
            return Some(new_document_dialog::dialog(state));
        }
        if let Some(state) = &self.preferences_dialog {
            let shown = preferences_dialog::Shown {
                keep_backup: self.preferences.keep_backup,
                check_updates: self.preferences.check_updates_on_start,
                keep_recovery: self.preferences.keep_recovery,
                recovery_minutes: self.preferences.recovery_minutes,
                default_sheet: &self.preferences.default_sheet,
                style_folders: &self.preferences.style_folders,
                documents_folder: self.preferences.documents_folder.as_deref(),
                environment_folders: std::env::var_os("AMIPRO_STYLES")
                    .map(|paths| std::env::split_paths(&paths).collect())
                    .unwrap_or_default(),
                view: [
                    self.preferences.toolbar_visible,
                    self.preferences.ruler_visible,
                    self.preferences.field_shading,
                    self.preferences.show_notes,
                    self.preferences.show_bookmarks,
                    self.preferences.tabs_returns,
                    self.preferences.column_guides,
                    self.preferences.margins_in_colour,
                    self.preferences.show_pictures,
                    self.preferences.table_addresses,
                ],
                #[cfg(feature = "page-surface")]
                page_surface: self.page_surface,
                #[cfg(not(feature = "page-surface"))]
                page_surface: false,
            };
            return Some(preferences_dialog::dialog(state, &shown));
        }
        if let Some(editor) = self.toolbar_editor {
            return Some(toolbar_editor_view(&self.preferences.toolbar, editor));
        }
        if let Some(problem) = &self.save_problem {
            return Some(
                widget::dialog()
                    .title(problem.title.clone())
                    .body(problem.body.clone())
                    .primary_action(
                        widget::button::suggested("Choose Another Name…")
                            .on_press(Message::SaveProblemRetry),
                    )
                    .secondary_action(
                        widget::button::standard("Cancel").on_press(Message::SaveProblemClose),
                    )
                    .into(),
            );
        }
        if let Some(pending) = &self.sam_save {
            let mut body = String::from(
                "Ami Pro 3.1 cannot store everything in this document exactly. Saving as an Ami Pro document changes:",
            );
            for loss in &pending.losses {
                body.push_str("\n• ");
                body.push_str(loss);
            }
            let name = pending
                .path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            return Some(
                widget::dialog()
                    .title(format!("Save {name} as an Ami Pro document?"))
                    .body(body)
                    .primary_action(
                        widget::button::suggested("Cancel").on_press(Message::CancelSamSave),
                    )
                    .secondary_action(
                        widget::button::destructive("Save Anyway").on_press(Message::SaveSamAnyway),
                    )
                    .into(),
            );
        }
        #[cfg(feature = "page-surface")]
        if let Some(pending) = &self.export_pending {
            let mut body = format!(
                "{} cannot hold everything in this document. Exporting leaves out or changes:",
                pending.format.name()
            );
            for loss in &pending.losses {
                body.push_str("\n• ");
                body.push_str(loss);
            }
            if !pending.replaces.is_empty() {
                body.push_str("\nThese files already exist and will be replaced: ");
                body.push_str(&pending.replaces.join(", "));
            }
            let name = pending
                .path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            return Some(
                widget::dialog()
                    .title(format!("Export {name}?"))
                    .body(body)
                    .primary_action(
                        widget::button::suggested("Cancel").on_press(Message::CancelExport),
                    )
                    .secondary_action(
                        widget::button::destructive("Export Anyway")
                            .on_press(Message::ExportAnyway),
                    )
                    .into(),
            );
        }
        let path = self.overwrite_confirmation.pending()?;
        let mut body = format!(
            "{} already exists. Replace it with this document?",
            path.display()
        );
        if is_sam_path(path) && !file_workflow::sam_backup_path(path).exists() {
            body.push_str(&format!(
                " Its original is kept as {}.",
                file_name(&file_workflow::sam_backup_path(path))
            ));
        }
        Some(
            widget::dialog()
                .title("Replace existing file?")
                .body(body)
                .primary_action(
                    widget::button::suggested("Replace").on_press(Message::ConfirmOverwrite),
                )
                .secondary_action(
                    widget::button::standard("Cancel").on_press(Message::CancelOverwrite),
                )
                .into(),
        )
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        let password_before = self
            .password_dialog
            .as_ref()
            .map(|dialog| dialog.error.clone());
        #[cfg(feature = "page-surface")]
        let caret_before = (self.active_session, self.page_selection.focus.paragraph);
        // A press on the page takes the keys from the Outline navigator.
        #[cfg(feature = "page-surface")]
        if matches!(message, Message::Page(PageAction::MoveTo { .. }))
            && let Some(navigator) = self.sessions[self.active_session]
                .outline_navigator
                .as_mut()
        {
            navigator.keys = false;
        }
        #[cfg(feature = "page-surface")]
        let numbers_before = {
            let session = &self.sessions[self.active_session];
            let flow = session.page_selection.focus.flow;
            (
                session.id,
                session.typing_run.map(|run| run.changes),
                session.history.undo_len(),
                flow,
                session
                    .document
                    .flow(flow)
                    .and_then(|content| content.paragraphs.first().map(|first| first.text.clone())),
            )
        };
        let task = self.handle_message(message);
        #[cfg(feature = "page-surface")]
        {
            let active = self.active_session;
            self.sessions[active].settle_numbers(numbers_before);
        }
        // Outline mode: the caret never stays in hidden text; it goes on the
        // way it was moving (OB.4).
        #[cfg(feature = "page-surface")]
        {
            let forward = caret_before.0 != self.active_session
                || self.page_selection.focus.paragraph >= caret_before.1;
            self.sessions[self.active_session].leave_hidden_text(forward);
        }
        // The password box takes the keyboard when it opens, and again
        // after a wrong password (Ami Pro's edit has the focus).
        let task = match &self.password_dialog {
            Some(dialog) if password_before.as_ref() != Some(&dialog.error) => Task::batch([
                task,
                widget::text_input::focus::<Message>(widget::Id::new(
                    password_dialog::PASSWORD_FIELD,
                ))
                .chain(Task::done(Message::Repaint))
                .map(cosmic::Action::from),
            ]),
            _ => task,
        };
        #[cfg(feature = "page-surface")]
        self.sync_find_bar();
        #[cfg(feature = "page-surface")]
        self.sync_style_properties();
        #[cfg(feature = "page-surface")]
        self.sync_paragraph_properties();
        #[cfg(feature = "page-surface")]
        self.sync_font_families();
        #[cfg(feature = "page-surface")]
        self.sync_page_layout_properties();
        #[cfg(feature = "page-surface")]
        self.sync_table_properties();
        #[cfg(feature = "page-surface")]
        self.sync_field_panels();
        #[cfg(feature = "page-surface")]
        self.sync_chart_panel();
        #[cfg(feature = "page-surface")]
        self.sync_equation_panel();
        #[cfg(feature = "page-surface")]
        self.sync_colour_panel();
        #[cfg(feature = "page-surface")]
        self.refresh_contents_lists();
        #[cfg(feature = "page-surface")]
        self.sync_data_forms();
        #[cfg(feature = "page-surface")]
        let task = match self.sync_master_sets() {
            Some(reads) => Task::batch([task, reads]),
            None => task,
        };
        self.sync_tab_model();
        #[cfg(feature = "page-surface")]
        self.sync_recovery();
        let session = &mut self.sessions[self.active_session];
        session.words.refresh(&session.document);
        #[cfg(feature = "page-surface")]
        let task = match self.sync_spelling() {
            Some(idle) => Task::batch([task, idle]),
            None => task,
        };
        self.apply_undo_levels();
        self.save_preferences();
        match self.follow_active_session() {
            Some(scroll) => Task::batch([task, scroll]),
            None => task,
        }
    }
}

impl AmiProApp {
    /// The message Esc sends for the topmost open dialog: what its Cancel
    /// (or, with none, its OK) button sends, never a confirmation. The order
    /// is `dialog`'s, so Esc and the view name the same dialog; `escape`
    /// asserts they agree.
    fn dialog_cancel(&self) -> Option<Message> {
        if self.import_dialog.is_some() {
            return Some(Message::ImportOptions(import::OptionsAction::Cancel));
        }
        #[cfg(feature = "page-surface")]
        if self.field_prompt.is_some() {
            return Some(Message::FieldPrompt(field_commands::PromptAction::Cancel));
        }
        if let Some(index) = self.pending_close {
            self.sessions.get(index)?;
            return Some(Message::CancelClose);
        }
        if self.rename_style.is_some() {
            return Some(Message::RenameStyle(RenameAction::Cancel));
        }
        if self.style_management.is_some() {
            return Some(Message::StyleManagement(
                style_dialogs::ManagementAction::Close,
            ));
        }
        if self.use_sheet.is_some() {
            return Some(Message::UseStyleSheet(
                style_dialogs::UseSheetAction::Cancel,
            ));
        }
        if self.create_style.is_some() {
            return Some(Message::CreateStyle(style_dialogs::CreateAction::Cancel));
        }
        if self.save_sheet.is_some() {
            return Some(Message::SaveSheet(SaveSheetAction::Cancel));
        }
        #[cfg(feature = "page-surface")]
        {
            if self.floating_dialog.is_some() {
                return Some(Message::FloatingHeaderFooter(
                    page_dialogs::FloatingAction::Cancel,
                ));
            }
            if self.page_number_dialog.is_some() {
                return Some(Message::PageNumber(page_dialogs::PageNumberAction::Cancel));
            }
            if self.create_frame_dialog.is_some() {
                return Some(Message::CreateFrame(
                    page_dialogs::CreateFrameAction::Cancel,
                ));
            }
            if self.insert_table_dialog.is_some() {
                return Some(Message::InsertTable(
                    table_commands::InsertTableAction::Cancel,
                ));
            }
            if self.bullet_dialog.is_some() {
                return Some(Message::InsertBullet(page_dialogs::BulletAction::Cancel));
            }
            if self.outline_numbering.is_some() {
                return Some(Message::OutlineNumbering(
                    page_dialogs::OutlineNumberingAction::Cancel,
                ));
            }
            if self.go_to.is_some() {
                return Some(Message::GoTo(GoToAction::Cancel));
            }
            if self.pending_frame_delete {
                return Some(Message::CancelFrameDelete);
            }
            if self.special_effects.is_some() {
                return Some(Message::SpecialEffects(
                    character_dialogs::EffectsAction::Cancel,
                ));
            }
            if self.font_dialog.is_some() {
                return Some(Message::FontDialog(character_dialogs::FontAction::Cancel));
            }
            if self.special_character {
                return Some(Message::SpecialCharacter(
                    special_character::CharacterAction::Cancel,
                ));
            }
        }
        if self.revert_pending {
            return Some(Message::RevertToSaved(false));
        }
        if self.password_dialog.is_some() {
            return Some(Message::Password(password_dialog::PasswordAction::Cancel));
        }
        #[cfg(feature = "page-surface")]
        if self.fast_format_ask.is_some() {
            return Some(Message::FastFormat(fast_format::FastAction::Cancel));
        }
        if self.new_document_dialog.is_some() {
            return Some(Message::NewDocumentDialog(
                new_document_dialog::NewAction::Cancel,
            ));
        }
        if self.preferences_dialog.is_some() {
            return Some(Message::Preferences(
                preferences_dialog::PreferencesAction::Close,
            ));
        }
        if self.toolbar_editor.is_some() {
            return Some(Message::CloseToolbarEditor);
        }
        if self.save_problem.is_some() {
            return Some(Message::SaveProblemClose);
        }
        if self.sam_save.is_some() {
            return Some(Message::CancelSamSave);
        }
        #[cfg(feature = "page-surface")]
        if self.export_pending.is_some() {
            return Some(Message::CancelExport);
        }
        self.overwrite_confirmation
            .pending()
            .map(|_| Message::CancelOverwrite)
    }

    /// Esc: cancels the topmost dialog only; with none open, ends Show Clean
    /// Screen, then closes a focused find bar.
    fn escape(&mut self) -> Task<cosmic::Action<Message>> {
        let cancel = self.dialog_cancel();
        debug_assert_eq!(
            cancel.is_some(),
            cosmic::Application::dialog(self).is_some(),
            "a dialog `dialog` shows has no Esc message in `dialog_cancel`"
        );
        if let Some(message) = cancel {
            return self.handle_message(message);
        }
        // Esc in the Formula box gives the cell's formula back (TD.7).
        #[cfg(feature = "page-surface")]
        if self
            .table_properties
            .as_ref()
            .is_some_and(|state| state.formula.is_some())
        {
            return self.handle_message(Message::TableProperties(
                table_properties::TableAction::FormulaRestore,
            ));
        }
        // Esc leaves the Equation editor (what was typed is on the frame).
        #[cfg(feature = "page-surface")]
        if self.equation_panel.is_some()
            && self.find.as_ref().is_none_or(|bar| bar.focused.is_none())
        {
            self.close_equation();
            return Task::none();
        }
        // Esc closes the Chart panel (what is typed and not applied is
        // dropped), when no field of the find bar takes it.
        #[cfg(feature = "page-surface")]
        if self.chart_panel.is_some() && self.find.as_ref().is_none_or(|bar| bar.focused.is_none())
        {
            self.close_chart();
            return Task::none();
        }
        // Esc closes the Recover documents panel and keeps the copies.
        #[cfg(feature = "page-surface")]
        if self.recovery_panel.is_some()
            && self.find.as_ref().is_none_or(|bar| bar.focused.is_none())
        {
            return self.update_recovery(recovery_panel::RecoveryAction::Close);
        }
        // Esc closes the Help panel.
        #[cfg(feature = "page-surface")]
        if self.help_panel.is_some() && self.find.as_ref().is_none_or(|bar| bar.focused.is_none()) {
            return self.update_help(help_panel::PanelAction::Close);
        }
        // Esc closes the Updates panel (a transfer in flight finishes and
        // is dropped).
        #[cfg(feature = "page-surface")]
        if self.updates_panel.is_some()
            && self.find.as_ref().is_none_or(|bar| bar.focused.is_none())
        {
            return self.update_updates(updates_panel::UpdatesAction::Close);
        }
        // Esc closes the Sort panel when no field of the find bar takes it.
        #[cfg(feature = "page-surface")]
        if self.sort_panel.is_some() && self.find.as_ref().is_none_or(|bar| bar.focused.is_none()) {
            self.close_sort();
            return Task::none();
        }
        #[cfg(feature = "page-surface")]
        return self.update_find(find_bar::FindAction::Escape);
        #[cfg(not(feature = "page-surface"))]
        Task::none()
    }

    /// Writes the toolbar and the rail's visibility to the user's
    /// configuration when they changed.
    fn save_preferences(&mut self) {
        let Some(settings) = &mut self.settings else {
            return;
        };
        if let Err(error) = settings.save(&self.preferences, self.styles_visible) {
            self.status = format!("The toolbar settings could not be saved: {error}");
        }
    }

    /// After a tab change (activated, opened, created, or closed), scrolls
    /// the shared workspace to the active tab's own offset and gives the tab
    /// the workspace's size (see [`WORKSPACE_SCROLL`]).
    fn follow_active_session(&mut self) -> Option<Task<cosmic::Action<Message>>> {
        let session = &mut self.sessions[self.active_session];
        #[cfg(feature = "page-surface")]
        if self.workspace.viewport != cosmic::iced::Size::ZERO {
            session.viewport = self.workspace.viewport;
        }
        let shown = self.workspace.shown_session.replace(session.id)?;
        (shown != session.id).then(|| {
            let offset = cosmic::iced::widget::scrollable::AbsoluteOffset {
                x: Some(session.scroll_offset.x),
                y: Some(session.scroll_offset.y),
            };
            scroll_workspace(workspace_scroll_id(), offset)
        })
    }

    /// Applies one message; `update` then reconciles derived widget models.
    fn handle_message(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::Repaint => trace_scroll(|| "repaint".to_owned()),
            #[cfg(feature = "page-surface")]
            Message::GoTo(action) => return self.update_go_to(action),
            #[cfg(feature = "page-surface")]
            Message::Find(action) => return self.update_find(action),
            Message::Escape => return self.escape(),
            #[cfg(feature = "page-surface")]
            Message::FieldPrompt(action) => self.answer_field_prompt(action),
            #[cfg(feature = "page-surface")]
            Message::Modifiers(modifiers) => {
                self.shift = modifiers.shift();
                self.control = modifiers.control();
            }
            Message::CreateStyle(action) => self.update_create_style(action),
            Message::StyleRowContext(style) => self.rail.context_style = style,
            // A chosen entry forgets the row itself: Modify Style… replaces
            // the list, and its menu, before the menu reports its close.
            Message::StyleRowMenu(action) => {
                if let Some(style) = self.rail.context_style.take() {
                    return self.style_row(style, action);
                }
            }
            Message::RenameStyle(action) => {
                let Some((style, name)) = self.rename_style.clone() else {
                    return Task::none();
                };
                match action {
                    RenameAction::Name(name) => self.rename_style = Some((style, name)),
                    RenameAction::Cancel => self.rename_style = None,
                    // A name that cannot be used waits, as the button does.
                    RenameAction::Rename
                        if amipro_editing::style_name_problem(
                            self.document.styles(),
                            &name,
                            Some(style),
                        )
                        .is_some() => {}
                    RenameAction::Rename => {
                        if self.execute_document_command(&amipro_editing::Command::RenameStyle {
                            style,
                            name: name.clone(),
                        }) {
                            self.rename_style = None;
                            self.status = format!("Renamed the style {name}");
                        }
                    }
                }
            }
            Message::StyleManagement(action) => self.update_style_management(action),
            #[cfg(feature = "page-surface")]
            Message::StyleProperties(action) => self.update_style_properties(action),
            #[cfg(feature = "page-surface")]
            Message::PageLayoutProperties(action) => self.update_page_layout_properties(action),
            #[cfg(feature = "page-surface")]
            Message::TableProperties(action) => self.update_table_properties(action),
            #[cfg(feature = "page-surface")]
            Message::FieldsPanel(action) => self.update_fields_panel(action),
            #[cfg(feature = "page-surface")]
            Message::References(action) => return self.update_references(action),
            #[cfg(feature = "page-surface")]
            Message::Contents(action) => return self.update_contents(action),
            #[cfg(feature = "page-surface")]
            Message::Master(action) => return self.update_master(action),
            #[cfg(feature = "page-surface")]
            Message::MasterRead {
                master,
                stored,
                result,
            } => self.master_member_read(master, stored, result),
            #[cfg(feature = "page-surface")]
            Message::Merge(action) => return self.update_merge(action),
            #[cfg(feature = "page-surface")]
            Message::Envelope(action) => return self.update_envelope(action),
            #[cfg(feature = "page-surface")]
            Message::MergeRead { session_id, result } => {
                if let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                {
                    let active = self.active_session;
                    self.active_session = index;
                    self.merge_read(result);
                    self.active_session = active;
                }
            }
            #[cfg(feature = "page-surface")]
            Message::Glossary(action) => return self.update_glossary(action),
            #[cfg(feature = "page-surface")]
            Message::GlossaryFile { path, new } => self.glossary_file_chosen(path, new),
            #[cfg(feature = "page-surface")]
            Message::Proofing(action) => return self.update_proofing(action),
            #[cfg(feature = "page-surface")]
            Message::DocumentProperties(action) => self.update_document_properties(action),
            #[cfg(feature = "page-surface")]
            Message::PrintPanel(action) => self.update_print_panel(action),
            #[cfg(feature = "page-surface")]
            Message::FloatingHeaderFooter(action) => self.update_floating_dialog(action),
            #[cfg(feature = "page-surface")]
            Message::PageNumber(action) => self.update_page_number_dialog(action),
            #[cfg(feature = "page-surface")]
            Message::CreateFrame(action) => self.update_create_frame_dialog(action),
            #[cfg(feature = "page-surface")]
            Message::InsertTable(action) => self.update_insert_table(action),
            #[cfg(feature = "page-surface")]
            Message::InsertBullet(action) => self.update_bullet_dialog(action),
            #[cfg(feature = "page-surface")]
            Message::OutlineNumbering(action) => self.update_outline_numbering(action),
            #[cfg(feature = "page-surface")]
            Message::NewFrameDefault(action) => self.set_new_frame_default(action),
            Message::UseStyleSheet(action) => return self.update_use_style_sheet(action),
            #[cfg(feature = "page-surface")]
            Message::ImageChosen {
                session_id,
                result,
                link,
            } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => self.insert_image_file(&path, link),
                    FileDialogResult::Cancelled => {}
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Insert Image failed: {error}");
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportPdfChosen { session_id, result } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => return self.export_pdf(path),
                    FileDialogResult::Cancelled => {}
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Export PDF failed: {error}");
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::PdfExported { session_id, result } => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    session.status = match result {
                        Ok(exported) => exported.message(),
                        Err(error) => format!("Export PDF failed: {error}"),
                    };
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportChosen {
                session_id,
                format,
                result,
            } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => return self.export(path, format),
                    FileDialogResult::Cancelled => {}
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Export failed: {error}");
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportPrepared { session_id, result } => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    match result {
                        Ok(prepared) if prepared.losses.is_empty() => {
                            session.finish_export(&prepared)
                        }
                        Ok(prepared) => {
                            session.status = "Export waits for your answer".into();
                            session.export_pending = Some(prepared);
                        }
                        Err(error) => session.status = format!("Export failed: {error}"),
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportAnyway => {
                if let Some(prepared) = self.export_pending.take() {
                    self.finish_export(&prepared);
                }
            }
            #[cfg(feature = "page-surface")]
            Message::CancelExport => {
                self.export_pending = None;
                self.status = "Export cancelled; nothing was written".into();
            }
            #[cfg(feature = "page-surface")]
            Message::OpenExported => {
                if let Some(path) = self.exported.clone() {
                    return cosmic::task::future(async move {
                        Message::ExportedOpened(export::open_file(path).await)
                    });
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ImportFolder(action) => return self.update_import_folder(action),
            #[cfg(feature = "page-surface")]
            Message::ImportFolderProgress {
                session_id,
                progress,
            } => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    session.import_folder_progress(progress);
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportFolder(action) => return self.update_export_folder(action),
            #[cfg(feature = "page-surface")]
            Message::ExportFolderProgress {
                session_id,
                progress,
            } => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    session.export_folder_progress(progress);
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ExportedOpened(result) => {
                if let Err(error) = result {
                    self.status = format!("The exported file could not be opened: {error}");
                }
            }
            #[cfg(feature = "page-surface")]
            Message::PrintPrepared { session_id, result } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    Ok(Some(prepared)) => {
                        if let Some(task) = self.print_merged(&prepared) {
                            return task;
                        }
                        if let Some(task) = self.print_envelope(&prepared) {
                            return task;
                        }
                        return self.print_prepared(prepared);
                    }
                    // Cancelled: nothing waits any more.
                    Ok(None)
                        if self
                            .merge_panel
                            .as_ref()
                            .is_some_and(|panel| panel.printing)
                            || self
                                .envelope_panel
                                .as_ref()
                                .is_some_and(|panel| panel.printing) =>
                    {
                        if let Some(panel) = self.merge_panel.as_mut() {
                            panel.printing = false;
                        }
                        if let Some(panel) = self.envelope_panel.as_mut() {
                            panel.printing = false;
                        }
                        self.viewing_job = None;
                    }
                    Ok(None) => {}
                    Err(error) => self.status = format!("Print failed: {error}"),
                }
            }
            #[cfg(feature = "page-surface")]
            Message::Printed { session_id, result } => {
                if let Some(session) = self
                    .sessions
                    .iter_mut()
                    .find(|session| session.id == session_id)
                {
                    session.status = match result {
                        Ok(pages) => print::printed_message(pages),
                        Err(error) => format!("Print failed: {error}"),
                    };
                }
            }
            #[cfg(feature = "page-surface")]
            Message::SavePicture => return self.save_picture_task(),
            #[cfg(feature = "page-surface")]
            Message::SavePictureChosen { session_id, result } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => self.save_picture_file(&path),
                    FileDialogResult::Cancelled => {}
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Save Picture As failed: {error}");
                    }
                }
            }
            Message::SheetChosen { session_id, result } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => self.use_style_sheet_file(&path),
                    FileDialogResult::Cancelled => {}
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Use Another Style Sheet failed: {error}");
                    }
                }
            }
            Message::SaveSheet(action) => {
                let Some(description) = self.save_sheet.as_mut() else {
                    return Task::none();
                };
                match action {
                    SaveSheetAction::Description(text) => *description = text,
                    SaveSheetAction::Contents(on) => self.save_sheet_contents = on,
                    SaveSheetAction::Cancel => self.save_sheet = None,
                    SaveSheetAction::Choose => {
                        let description = self.save_sheet.take().unwrap_or_default();
                        return self.style_sheet_dialog_task(description, self.save_sheet_contents);
                    }
                }
            }
            Message::SheetDialogFinished {
                session_id,
                description,
                contents,
                result,
            } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(path) => {
                        self.save_style_sheet(&path, &description, contents);
                    }
                    FileDialogResult::Cancelled => {
                        self.status = "Save as a Style Sheet cancelled".into();
                    }
                    FileDialogResult::Failed(error) => {
                        self.status = format!("Save as a Style Sheet failed: {error}");
                        self.save_problem = Some(SaveProblem {
                            title: "The file chooser could not open".to_owned(),
                            body: format!("{error}. The style sheet was not saved."),
                            retry: SaveRetry::StyleSheet {
                                description: description.clone(),
                                contents,
                            },
                        });
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::SpecialEffects(action) => self.update_special_effects(&action),
            #[cfg(feature = "page-surface")]
            Message::FontDialog(action) => self.update_font_dialog(&action),
            #[cfg(feature = "page-surface")]
            Message::Colour(action) => return self.update_colour(action),
            Message::StyleFilter(text) => self.rail.style_filter = text,
            Message::Toolbar(edit) => {
                if let Some(editor) = &mut self.toolbar_editor {
                    editor.apply(&mut self.preferences.toolbar, edit);
                }
            }
            Message::CloseToolbarEditor => self.toolbar_editor = None,
            Message::WindowResized(width) => {
                // The rail keeps its state, open or closed, when a resize
                // moves the window across the docking width (an overlay when
                // narrow, docked when wide) instead of going away (the user,
                // 2026-11-06). The first report of the width changes nothing.
                let before = self
                    .rail
                    .window_width
                    .map(|_| (self.rail_overlays(), self.rail_open()));
                self.rail.window_width = Some(width);
                if let Some((was_overlay, was_open)) = before
                    && was_overlay != self.rail_overlays()
                {
                    if self.rail_overlays() {
                        self.rail.overlay_open = was_open;
                    } else {
                        self.styles_visible = was_open;
                    }
                }
            }
            Message::Command(command) => {
                // A command ends a run of typing (SU4).
                #[cfg(feature = "page-surface")]
                if let Some(session) = self.sessions.get_mut(self.active_session) {
                    session.typing_run = None;
                }
                return self.dispatch_command(command);
            }
            Message::ClipboardRead {
                command,
                session_id,
                contents,
            } => {
                self.clipboard_has_text = Some(
                    contents
                        .as_ref()
                        .is_some_and(|contents| !contents.is_empty()),
                );
                // Paste with no text, or only a picture's address or path,
                // looks for an image or copied picture files first (P7):
                // a browser's Copy Image puts the address beside the image.
                #[cfg(feature = "page-surface")]
                if command == CommandId::Paste {
                    trace_drop(|| {
                        format!(
                            "paste: text {:?}",
                            contents
                                .as_ref()
                                .map(|text| text.chars().take(120).collect::<String>())
                        )
                    });
                }
                #[cfg(feature = "page-surface")]
                if self.id == session_id
                    && self.page_surface
                    && command == CommandId::Paste
                    && contents
                        .as_deref()
                        .is_none_or(|text| text.is_empty() || picture_insert::link_text(text))
                {
                    return cosmic::iced::clipboard::read_data::<picture_insert::ClipboardImage>()
                        .map(move |image| {
                            cosmic::Action::from(Message::ClipboardImage {
                                session_id,
                                image,
                                text: contents.clone(),
                            })
                        });
                }
                if self.id == session_id
                    && matches!(command, CommandId::Paste | CommandId::PastePlainText)
                {
                    match contents.filter(|contents| !contents.is_empty()) {
                        Some(contents) => self.insert_clipboard_text(contents),
                        None => self.status = "The clipboard holds no text to paste".into(),
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ClipboardImage {
                session_id,
                image,
                text,
            } => {
                trace_drop(|| {
                    format!(
                        "paste read {:?}, text {:?}",
                        image
                            .as_ref()
                            .map(|image| (image.mime.clone(), image.bytes.len())),
                        text.as_ref()
                            .map(|text| text.chars().take(120).collect::<String>())
                    )
                });
                let image = image.filter(picture_insert::ClipboardImage::is_picture);
                self.clipboard_has_image = Some(image.is_some());
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return Task::none();
                };
                self.active_session = index;
                if image.is_some() && self.refuse_in_formula_cell() {
                    return Task::none();
                }
                match image {
                    Some(image) => match image.files() {
                        Some(files) => self.insert_image_file(&files[0], false),
                        None => {
                            let extension = image.extension();
                            self.insert_picture_bytes(image.bytes, extension, None);
                        }
                    },
                    None => match text.filter(|text| !text.is_empty()) {
                        Some(text) => self.insert_clipboard_text(text),
                        None => self.status = "The clipboard holds nothing to paste".into(),
                    },
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ClipboardImageAvailability { has_image, retry } => {
                trace_drop(|| format!("clipboard has an image: {has_image} (first look {retry})"));
                self.clipboard_has_image = Some(has_image);
                if !has_image && retry {
                    return Self::retry_clipboard();
                }
            }
            #[cfg(feature = "page-surface")]
            Message::Dropped {
                session_id,
                data,
                mime,
            } => {
                trace_drop(|| {
                    format!(
                        "dropped {mime}, {} bytes: {:?}",
                        data.len(),
                        String::from_utf8_lossy(&data[..data.len().min(300)])
                    )
                });
                if let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                {
                    self.active_session = index;
                    // A dropped document opens (IM1).
                    if mime == "text/uri-list" {
                        let documents = picture_insert::dropped_documents(&data);
                        if !documents.is_empty() {
                            for path in documents {
                                self.open_document_path(path);
                            }
                            return Task::none();
                        }
                    }
                    match picture_insert::ClipboardImage::try_from((data, mime)) {
                        Ok(image) if image.files().is_none() => {
                            let extension = image.extension();
                            self.insert_picture_bytes(image.bytes, extension, None);
                        }
                        Ok(image) => match image.files().unwrap_or_default().first() {
                            Some(path) => self.insert_image_file(path, false),
                            None => {
                                self.status =
                                    "Drop a BMP, WMF, PNG, JPEG, GIF, TIFF, PCX, or SVG file"
                                        .into();
                            }
                        },
                        Err(()) => {
                            self.status =
                                "Drop a BMP, WMF, PNG, JPEG, GIF, TIFF, PCX, or SVG file".into();
                        }
                    }
                }
            }
            Message::RefreshClipboard => return Self::clipboard_availability_task(true),
            Message::RetryClipboard => return Self::clipboard_availability_task(false),
            Message::ClipboardAvailability { has_text, retry } => {
                #[cfg(feature = "page-surface")]
                trace_drop(|| format!("clipboard has text: {has_text} (first look {retry})"));
                self.clipboard_has_text = Some(has_text);
                // Without text, an image may still paste (P7).
                #[cfg(feature = "page-surface")]
                if !has_text && self.page_surface {
                    return cosmic::iced::clipboard::read_data::<picture_insert::ClipboardImage>()
                        .map(move |image| {
                            cosmic::Action::from(Message::ClipboardImageAvailability {
                                has_image: image.is_some_and(|image| image.is_picture()),
                                retry,
                            })
                        });
                }
                if !has_text && retry {
                    return Self::retry_clipboard();
                }
            }
            Message::Edit(action) => {
                self.apply_standard_action(action);
            }
            Message::SaveDialogFinished {
                session_id,
                result,
                format,
            } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return self.update_title();
                };
                self.active_session = index;
                match result {
                    FileDialogResult::Selected(selected) => {
                        match classify_save_selection(&selected, format, |path| path.exists()) {
                            Ok(SaveSelection::ReadySam(path)) => {
                                match self.begin_sam_save(path, false) {
                                    Some(true) => {
                                        if self.close_after_save.take() == Some(session_id) {
                                            return self.close_tab(index);
                                        }
                                    }
                                    Some(false) => {
                                        self.close_after_save = None;
                                        self.quit_requested = false;
                                    }
                                    // The loss check's dialog decides.
                                    None => {}
                                }
                            }
                            Ok(SaveSelection::Ready(path)) => {
                                if self.save_to_native(path) {
                                    if self.close_after_save.take() == Some(session_id) {
                                        return self.close_tab(index);
                                    }
                                } else {
                                    self.close_after_save = None;
                                    self.quit_requested = false;
                                }
                            }
                            Ok(SaveSelection::ConfirmOverwrite(path)) => {
                                self.overwrite_confirmation.begin(path);
                            }
                            #[cfg(feature = "page-surface")]
                            Ok(SaveSelection::ExportMarkdown(path)) => {
                                // A copy: the tab keeps its file, so nothing
                                // is saved for a close in progress.
                                self.close_after_save = None;
                                self.quit_requested = false;
                                return self.export(path, amipro_exchange::Format::Markdown);
                            }
                            #[cfg(not(feature = "page-surface"))]
                            Ok(SaveSelection::ExportMarkdown(_)) => {
                                self.close_after_save = None;
                                self.quit_requested = false;
                                self.status = "Markdown needs the page surface".into();
                            }
                            Err(error) => {
                                self.close_after_save = None;
                                self.quit_requested = false;
                                self.status = format!("Save As failed: {error}");
                                self.save_problem = Some(SaveProblem::save_as(&error));
                            }
                        }
                    }
                    FileDialogResult::Cancelled => {
                        self.close_after_save = None;
                        self.quit_requested = false;
                        self.status = "Save As cancelled".into();
                    }
                    FileDialogResult::Failed(error) => {
                        self.close_after_save = None;
                        self.quit_requested = false;
                        self.status = format!("Save As dialog failed: {error}");
                        self.save_problem = Some(SaveProblem {
                            title: "The file chooser could not open".to_owned(),
                            body: format!("{error}. The document was not saved."),
                            retry: SaveRetry::SaveAs,
                        });
                    }
                }
            }
            Message::ConfirmOverwrite => {
                if let Some(path) = self.overwrite_confirmation.confirm() {
                    let session_id = self.id;
                    if is_sam_path(&path) {
                        match self.begin_sam_save(path, true) {
                            Some(true) => {
                                if self.close_after_save.take() == Some(session_id) {
                                    return self.close_tab(self.active_session);
                                }
                            }
                            Some(false) => {
                                self.close_after_save = None;
                                self.quit_requested = false;
                            }
                            // The loss check's dialog decides.
                            None => {}
                        }
                    } else if self.save_to_native(path) {
                        if self.close_after_save.take() == Some(session_id) {
                            return self.close_tab(self.active_session);
                        }
                    } else {
                        self.close_after_save = None;
                        self.quit_requested = false;
                    }
                }
            }
            Message::CancelOverwrite => {
                self.overwrite_confirmation.cancel();
                self.close_after_save = None;
                self.quit_requested = false;
                self.status = "Save As cancelled; existing file was not changed".into();
            }
            Message::SaveSamAnyway => {
                if let Some(pending) = self.sam_save.take() {
                    let session_id = self.id;
                    if self.write_sam(&pending.path, pending.saved_at, pending.replace) {
                        if self.close_after_save.take() == Some(session_id) {
                            return self.close_tab(self.active_session);
                        }
                    } else {
                        self.close_after_save = None;
                        self.quit_requested = false;
                    }
                }
            }
            Message::SaveProblemRetry => {
                if let Some(problem) = self.save_problem.take() {
                    return match problem.retry {
                        SaveRetry::SaveAs => self.save_dialog_task(),
                        SaveRetry::StyleSheet {
                            description,
                            contents,
                        } => self.style_sheet_dialog_task(description, contents),
                    };
                }
            }
            Message::SaveProblemClose => self.save_problem = None,
            Message::CancelSamSave => {
                self.sam_save = None;
                self.close_after_save = None;
                self.quit_requested = false;
                self.status = "Save cancelled; nothing was written".into();
            }
            Message::ImportOptions(action) => return self.update_import_options(action),
            #[cfg(feature = "page-surface")]
            Message::ImportReport(action) => return self.update_import_report(action),
            #[cfg(feature = "page-surface")]
            Message::ClipboardRtf { session_id, rtf } => {
                let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                else {
                    return Task::none();
                };
                self.active_session = index;
                if rtf.is_some() && self.refuse_in_formula_cell() {
                    return Task::none();
                }
                return match rtf {
                    Some(rtf) => {
                        let purpose = self.insert_purpose(true);
                        self.start_import(
                            import::Source::Clipboard(rtf.0),
                            import::paste_options(),
                            purpose,
                        )
                    }
                    None => cosmic::iced::clipboard::read().map(move |contents| {
                        cosmic::Action::from(Message::ClipboardRead {
                            command: CommandId::Paste,
                            session_id,
                            contents,
                        })
                    }),
                };
            }
            Message::Imported(result) => self.imported(*result),
            #[cfg(feature = "page-surface")]
            Message::InsertFileDialogFinished { session_id, result } => match result {
                FileDialogResult::Selected(path) => self.ask_insert_file(session_id, path),
                FileDialogResult::Cancelled => self.status = "Insert File cancelled".into(),
                FileDialogResult::Failed(error) => {
                    self.status = format!("Insert File dialog failed: {error}");
                }
            },
            Message::OpenDialogFinished(result) => match result {
                FileDialogResult::Selected(path) => self.open_document_path(path),
                FileDialogResult::Cancelled => self.status = "Open cancelled".into(),
                FileDialogResult::Failed(error) => {
                    self.status = format!("Open dialog failed: {error}");
                }
            },
            #[cfg(feature = "page-surface")]
            Message::ApplyStyle(style) if self.page_surface => self.apply_page_style(style),
            Message::ApplyStyle(style) => {
                if let Some(paragraphs) =
                    selected_paragraph_range(self.document.paragraphs().len(), self.editor.cursor())
                {
                    self.push_undo();
                    for paragraph in paragraphs {
                        self.document.paragraph_mut(paragraph).style = style;
                    }
                    self.refresh_rich_layout();
                    self.status = format!("Applied {}", self.document.styles()[&style].name);
                    self.dirty = true;
                }
            }
            Message::ActivateTab(index) => return self.activate_tab(index),
            Message::TabActivated(entity) => {
                if let Some(index) = self.session_index_for_tab(entity) {
                    return self.activate_tab(index);
                }
            }
            Message::TabCloseRequested(entity) => {
                if let Some(index) = self.session_index_for_tab(entity) {
                    return self.request_close_tab(index);
                }
            }
            Message::ConfirmCloseDiscard => {
                if let Some(index) = self.pending_close.take() {
                    return self.close_tab(index);
                }
            }
            Message::SaveAndClose => {
                if let Some(index) = self.pending_close.take()
                    && index < self.sessions.len()
                {
                    self.active_session = index;
                    let session_id = self.id;
                    match self.location.save_request() {
                        SaveRequest::ChooseDestination => {
                            self.close_after_save = Some(session_id);
                            return self.save_dialog_task();
                        }
                        SaveRequest::SaveTo(path) => {
                            if self.save_to_native(path) {
                                return self.close_tab(index);
                            } else {
                                self.quit_requested = false;
                            }
                        }
                        SaveRequest::SaveSam(path) => match self.begin_sam_save(path, true) {
                            Some(true) => return self.close_tab(index),
                            Some(false) => self.quit_requested = false,
                            // The loss check's dialog decides; the tab
                            // closes once Save Anyway has written it.
                            None => self.close_after_save = Some(session_id),
                        },
                    }
                }
            }
            Message::CancelClose => {
                self.pending_close = None;
                self.quit_requested = false;
            }
            Message::CloseAbout => {
                self.show_about = false;
                self.set_show_context(false);
            }
            Message::Preferences(action) => return self.update_preferences(action),
            #[cfg(feature = "page-surface")]
            Message::StatusFace(index) => {
                if let Some(family) = self.font_families.get(index).cloned() {
                    let session = &mut self.sessions[self.active_session];
                    session.apply_font_part(page_editing::FontChoice {
                        family: Some(family),
                        size: None,
                        colour: None,
                    });
                }
            }
            #[cfg(feature = "page-surface")]
            Message::StatusSize(index) => {
                if let Some(size) = character_dialogs::SIZES
                    .get(index)
                    .and_then(|size| size.parse::<f32>().ok())
                {
                    let session = &mut self.sessions[self.active_session];
                    session.apply_font_part(page_editing::FontChoice {
                        family: None,
                        size: Some(size),
                        colour: None,
                    });
                }
            }
            #[cfg(feature = "page-surface")]
            Message::SpecialCharacter(action) => {
                self.special_character = false;
                if let special_character::CharacterAction::Insert(character) = action {
                    special_character::remember(&mut self.preferences.recent_characters, character);
                    return self.update_page(PageAction::Insert(character.to_string()));
                }
            }
            Message::RevertToSaved(revert) => {
                self.revert_pending = false;
                if revert {
                    self.revert_to_saved();
                }
            }
            Message::Password(action) => self.update_password(action),
            #[cfg(feature = "page-surface")]
            Message::OutlineNavigator(action) => return self.update_outline_navigator(action),
            #[cfg(feature = "page-surface")]
            Message::Chart(action) => return self.update_chart(action),
            #[cfg(feature = "page-surface")]
            Message::Equation(action) => return self.update_equation(action),
            #[cfg(feature = "page-surface")]
            Message::EquationClipboard {
                session_id,
                text,
                create,
            } => return self.equation_clipboard(session_id, text, create),
            #[cfg(feature = "page-surface")]
            Message::EquationFile {
                session_id,
                path,
                save,
            } => self.equation_file_chosen(session_id, path, save),
            #[cfg(feature = "page-surface")]
            Message::ChartClipboard { session_id, text } => {
                self.charting_clipboard(session_id, text);
            }
            #[cfg(feature = "page-surface")]
            Message::ChartImageChosen { session_id, result } => {
                if let Some(index) = self
                    .sessions
                    .iter()
                    .position(|session| session.id == session_id)
                {
                    self.active_session = index;
                    match result {
                        FileDialogResult::Selected(path) => self.save_chart_image(&path),
                        FileDialogResult::Cancelled => {}
                        FileDialogResult::Failed(error) => {
                            self.status = format!("Save Chart as Image failed: {error}");
                        }
                    }
                }
            }
            #[cfg(feature = "page-surface")]
            Message::Links(action) => return self.update_links(action),
            #[cfg(feature = "page-surface")]
            Message::Updates(action) => return self.update_updates(action),
            #[cfg(feature = "page-surface")]
            Message::Recovery(action) => return self.update_recovery(action),
            #[cfg(feature = "page-surface")]
            Message::HelpPanel(action) => return self.update_help(action),
            #[cfg(feature = "page-surface")]
            Message::PasteSpecial(action) => return self.update_paste_special(action),
            #[cfg(feature = "page-surface")]
            Message::PasteSpecialFound {
                session_id,
                slot,
                data,
            } => return self.paste_special_found(session_id, slot, data),
            #[cfg(feature = "page-surface")]
            Message::PasteSpecialRetry(session_id) => {
                return Self::look_at_clipboard(session_id);
            }
            #[cfg(feature = "page-surface")]
            Message::Sort(action) => return self.update_sort(action),
            #[cfg(feature = "page-surface")]
            Message::NavigatorKey(key) => return self.navigator_key(key),
            Message::StyleFilterSubmit => {
                #[cfg(feature = "page-surface")]
                if let Some(style) = filtered_styles(&self.document, &self.rail.style_filter)
                    .first()
                    .map(|style| style.id)
                {
                    self.apply_page_style(style);
                }
            }
            #[cfg(feature = "page-surface")]
            Message::ParagraphProperties(action) => self.update_paragraph_properties(action),
            Message::NewDocumentDialog(action) => self.update_new_document(action),
            #[cfg(feature = "page-surface")]
            Message::FastFormat(action) => self.update_fast_format(action),
            Message::OpenRecent(index) => {
                if let Some(path) = self.preferences.recent_files.get(index).cloned() {
                    if path.exists() {
                        self.open_document_path(path);
                    } else {
                        self.status = format!("{} is no longer there", path.display());
                        self.preferences.recent_files.remove(index);
                    }
                }
            }
            Message::HelpDrawer(action) => match action {
                help_drawers::HelpAction::Filter(text) => {
                    if let Some(help_drawers::HelpDrawer::Shortcuts { filter }) =
                        self.help_drawer.as_mut()
                    {
                        *filter = text;
                    }
                }
                help_drawers::HelpAction::CheckAgain => {
                    self.help_drawer = Some(self.compatibility_check());
                }
                help_drawers::HelpAction::Close => {
                    self.help_drawer = None;
                    self.set_show_context(false);
                }
            },
            Message::Scrolled(viewport) => {
                trace_scroll(|| format!("scrolled {:?}", viewport.absolute_offset()));
                self.scroll_offset = viewport.absolute_offset();
            }
            #[cfg(feature = "rich-editor-prototype")]
            Message::Prototype(action) => self.update_prototype(action),
            #[cfg(feature = "page-surface")]
            Message::Page(action) => {
                // A click in the text leaves the ruler's keyboard mode.
                if matches!(action, PageAction::MoveTo { .. }) {
                    self.ruler_cursor = None;
                }
                let follow =
                    self.control && matches!(action, PageAction::MoveTo { extend: false, .. });
                // Tab after a glossary name takes its record (TX7).
                if matches!(&action, PageAction::Insert(text) if text == "\t")
                    && self.complete_glossary_on_tab()
                {
                    return self.update_title();
                }
                // Typing while the merge preview shows a record turns it
                // off and is made in the letter, at the caret's place there
                // (the user's second use; it was refused before, MC3).
                if matches!(
                    action,
                    PageAction::Insert(_)
                        | PageAction::Backspace
                        | PageAction::Delete
                        | PageAction::ParagraphBreak
                ) && self.leave_merge_preview()
                {
                    self.status = "Preview off: the document shows its merge fields again".into();
                }
                let typed = matches!(action, PageAction::Insert(_));
                // The window's keys: typing, Backspace, and Delete may join
                // a run of typing (SU4).
                let scroll = self.update_page_from(action, true);
                // Ctrl+click on a TOC row goes to its heading (TX6).
                if follow && let Some(jump) = self.follow_set_link() {
                    return Task::batch([jump, self.update_title()]);
                }
                if follow && let Some(jump) = self.follow_toc_link() {
                    return Task::batch([jump, self.update_title()]);
                }
                if typed && let Some((_, name)) = self.glossary_completion() {
                    self.status = format!("Tab inserts the glossary record “{name}”");
                }
                return Task::batch([scroll, self.update_title()]);
            }
            #[cfg(feature = "page-surface")]
            Message::Frame(action) => {
                let scroll = self.update_frame(action);
                return Task::batch([scroll, self.update_title()]);
            }
            #[cfg(feature = "page-surface")]
            Message::Draw(event) => {
                self.update_draw(event);
                return self.update_title();
            }
            #[cfg(feature = "page-surface")]
            Message::DrawTool(tool) => self.choose_draw_tool(tool),
            #[cfg(feature = "page-surface")]
            Message::DrawPaste(data) => {
                self.paste_drawing(data);
                return self.update_title();
            }
            #[cfg(feature = "page-surface")]
            Message::DrawImportChosen { session_id, result } => {
                if session_id == self.id
                    && let FileDialogResult::Selected(path) = result
                {
                    self.import_drawing_file(&path);
                }
                return self.update_title();
            }
            #[cfg(feature = "page-surface")]
            Message::DrawSaveChosen { session_id, result } => {
                if session_id == self.id
                    && let FileDialogResult::Selected(path) = result
                {
                    self.save_drawing_file(&path);
                }
            }
            #[cfg(feature = "page-surface")]
            Message::DrawProperties(action) => {
                self.update_draw_properties(action);
                return self.update_title();
            }
            #[cfg(feature = "page-surface")]
            Message::DrawStyle(change) => {
                self.restyle_drawing(change);
                return self.update_title();
            }
            #[cfg(feature = "page-surface")]
            Message::ConfirmFrameDelete => self.delete_selected_frames(),
            #[cfg(feature = "page-surface")]
            Message::CancelFrameDelete => self.pending_frame_delete = false,
            #[cfg(feature = "page-surface")]
            Message::InspectorInput(field, text) => {
                let frame = self.inspected_frame().map(|(frame, _)| frame);
                let session = &mut self.sessions[self.active_session];
                if session.inspector_frame != frame {
                    session.inspector_drafts.clear();
                    session.inspector_frame = frame;
                }
                session.inspector_drafts.insert(field, text);
            }
            #[cfg(feature = "page-surface")]
            Message::InspectorSubmit(field) => self.submit_inspector_field(field),
            #[cfg(feature = "page-surface")]
            Message::InspectorChoice(choice) => self.choose_in_inspector(choice),
            #[cfg(feature = "page-surface")]
            Message::ImageSlide(field, value) => self.slide_image(field, value),
            #[cfg(feature = "page-surface")]
            Message::ImageSlideEnd => {
                self.sessions[self.active_session].colour_drag = None;
            }
            #[cfg(feature = "page-surface")]
            Message::ZoomTo(percent) => self.zoom = Zoom::percent(percent),
            #[cfg(feature = "page-surface")]
            Message::Ruler(event) => self.ruler_event(event),
            #[cfg(feature = "page-surface")]
            Message::RulerMenu(action) => self.ruler_menu_action(action),
        }
        self.update_title()
    }
}

#[cfg(feature = "page-surface")]
impl AmiProApp {
    /// Applies the Go To dialog's actions: a page number (refused in place
    /// when out of range), the first or last page, or the next frame.
    fn update_go_to(&mut self, action: GoToAction) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let count = session.page_count();
        let Some(state) = session.go_to.as_mut() else {
            return Task::none();
        };
        let page = match action {
            GoToAction::Input(text) => {
                state.page = text;
                state.error = None;
                return Task::none();
            }
            GoToAction::Cancel => {
                session.go_to = None;
                return Task::none();
            }
            GoToAction::NextFrame => {
                session.go_to = None;
                self.select_next_frame();
                return Task::none();
            }
            GoToAction::Item(item) => {
                state.item = item;
                return Task::none();
            }
            GoToAction::Step(forward) => {
                let item = state.item;
                session.go_to = None;
                return self.go_to_item(item, forward);
            }
            GoToAction::First => 1,
            GoToAction::Last => count,
            GoToAction::Go => match state.page.trim().parse::<u32>() {
                Ok(page) if (1..=count).contains(&page) => page,
                _ => {
                    state.error = Some(format!("Enter a page from 1 to {count}."));
                    return Task::none();
                }
            },
        };
        session.go_to = None;
        session.go_to_page(page, true)
    }

    /// Applies a page-surface action to the active session.
    fn update_page(&mut self, action: PageAction) -> Task<cosmic::Action<Message>> {
        self.update_page_from(action, false)
    }

    /// `update_page` for an action; `keyed` when the window's keys made it
    /// (typing, Backspace, Delete), which may join a run of typing (SU4).
    fn update_page_from(
        &mut self,
        action: PageAction,
        keyed: bool,
    ) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        match &action {
            PageAction::Viewport(size) => {
                session.viewport = *size;
                self.workspace.viewport = *size;
                // Draft follows the window's width.
                if session.draft.is_some() && session.draft != Some(session.draft_width()) {
                    session.draft = Some(session.draft_width());
                    session.refresh_rich_layout();
                }
                if session.outline.is_some() && session.outline != Some(session.draft_width()) {
                    session.outline = Some(session.draft_width());
                    session.refresh_rich_layout();
                }
                return Task::none();
            }
            PageAction::ContextAt { margin } => {
                session.context_in_margin = *margin;
                let thesaurus = self.proofing_context();
                // The menu opens on the release: Paste's state from the
                // clipboard as it is now.
                return Task::batch([Self::clipboard_availability_task(true), thesaurus]);
            }
            PageAction::OpenPageLayout => {
                self.open_page_layout_properties();
                return Task::none();
            }
            PageAction::EditFormula => return self.edit_formula(),
            PageAction::FrameDrawn(drawn) => {
                session.drawing_frame = false;
                match *drawn {
                    Some((page, rect)) => self.create_frame(page, rect),
                    None => session.status = "No frame drawn".into(),
                }
                return Task::none();
            }
            PageAction::CellStep { forward } => {
                let forward = *forward;
                return self.table_step(forward);
            }
            PageAction::CellMove {
                down,
                right,
                extend,
            } => {
                let (down, right, extend) = (*down, *right, *extend);
                return self.table_move(down, right, extend);
            }
            PageAction::MoveRows { down } => {
                let down = *down;
                return self.move_table_rows(down);
            }
            PageAction::DropCells { frame, to, copy } => {
                let (frame, to, copy) = (*frame, *to, *copy);
                self.drop_cells(frame, to, copy);
                return Task::none();
            }
            PageAction::ResizeTrack {
                frame,
                column,
                index,
                size,
            } => {
                let (frame, index, size) = (*frame, *index, (*size).max(1));
                let sizes = if *column {
                    amipro_editing::tables::TrackSizes {
                        width: Some(size),
                        ..Default::default()
                    }
                } else {
                    amipro_editing::tables::TrackSizes {
                        height: Some(size.max(10)),
                        ..Default::default()
                    }
                };
                let (rows, columns) = if *column {
                    (0..0, index..index + 1)
                } else {
                    (index..index + 1, 0..0)
                };
                if session.execute_page_command(&amipro_editing::Command::EditTable {
                    frame,
                    edit: amipro_editing::tables::TableEdit::SetSizes {
                        rows,
                        columns,
                        sizes,
                    },
                }) {
                    session.status = "Size set".into();
                }
                return Task::none();
            }
            PageAction::OpenNote(at) => {
                let at = *at;
                let page = reference_commands::page_showing(
                    session.pages.layout(),
                    at.flow,
                    at.paragraph,
                    at.byte,
                )
                .unwrap_or(session.page_selection.page);
                session.page_selection = PageSelection::caret(at, page);
                self.open_note_at(at);
                return Task::none();
            }
            PageAction::PointerReleased => {
                self.fast_format_selection();
                return Task::none();
            }
            PageAction::Escape if session.fast_format.is_some() => {
                session.fast_format = None;
                session.status = "Fast Format off".into();
                return Task::none();
            }
            PageAction::Escape => {
                // Frame text: Esc selects the frame (`UI_SPEC.md` §11).
                if let Some(frame) = frame_editing::page_frame_of_flow(
                    &session.document,
                    session.page_selection.flow(),
                ) {
                    session.frame_selection =
                        Some(FrameSelection::one(frame, session.page_selection.page));
                    session.status = "Frame selected".into();
                    return Task::none();
                }
                session.page_selection = page_surface::escape_target(
                    &session.document,
                    session.pages.layout(),
                    session.page_selection,
                );
                session.sync_editor_to_page();
                return session.scroll_to_caret();
            }
            _ => {}
        }
        if let Some(selection) =
            page_editing::moved_selection(&session.document, session.page_selection, &action)
        {
            session.typing_run = None;
            session.frame_selection = None;
            session.page_selection = selection;
            session.sync_editor_to_page();
            return session.scroll_to_caret();
        }
        if matches!(action, PageAction::Delete | PageAction::Backspace)
            && page_editing::selection_covers_formula_cell(
                &session.document,
                session.page_selection,
            )
        {
            // The whole cell selected: Del takes the formula out, one step.
            self.apply_formula(table_formula::FormulaChange::Remove);
            return Task::none();
        }
        if page_editing::formula_cell_refuses(&session.document, session.page_selection, &action) {
            session.status = table_formula::HOLDS_A_FORMULA.into();
            return Task::none();
        }
        let Some(command) =
            page_editing::edit_command(&session.document, session.page_selection, &action)
        else {
            return Task::none();
        };
        let typing = matches!(action, PageAction::Insert(_))
            .then(|| session.typing_format().cloned())
            .flatten();
        // Only plain text edits make a run: a whole field taken, an ended
        // list, or a picture's cell is a step of its own.
        let run = TypingKind::of(&action).filter(|_| {
            keyed
                && matches!(
                    command,
                    amipro_editing::Command::ReplaceText { .. }
                        | amipro_editing::Command::DeleteCharacter { .. }
                )
        });
        let join = run.is_some_and(|kind| session.continues_typing_run(kind));
        session.typing = None;
        let changed = session.run_page_command(&command, join);
        session.typing_run = run.filter(|_| changed).map(|kind| TypingRun {
            kind,
            at: session.page_selection.focus,
            changes: 0,
            last: std::time::Instant::now(),
        });
        // Text typed at the caret takes the formatting chosen there.
        if let Some(typing) = typing {
            let end = session.page_selection.focus;
            if end != typing.at {
                for command in typing.commands(typing.at, end) {
                    session.amend_page_command(&command);
                }
            }
        }
        if let Some(run) = session.typing_run.as_mut() {
            run.changes = session.history.changes();
        }
        session.scroll_to_caret()
    }

    /// Tab and Shift+Tab in a table (`UI_SPEC.md` §32): the next or previous
    /// cell; Tab in the last cell adds a row and goes to its first cell
    /// (TD5).
    fn table_step(&mut self, forward: bool) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some((frame, row, column)) =
            table_editing::caret_cell(&session.document, session.page_selection)
        else {
            return Task::none();
        };
        let page = session.page_selection.page;
        match table_editing::step(&session.document, frame, row, column, forward) {
            table_editing::Step::Cell(position) => {
                session.page_selection = PageSelection::caret(position, page);
            }
            table_editing::Step::AddRow => {
                let Some(edit) = table_editing::add_row(&session.document, frame) else {
                    return Task::none();
                };
                if !session
                    .execute_page_command(&amipro_editing::Command::EditTable { frame, edit })
                {
                    return Task::none();
                }
                let last = table_editing::table_of(&session.document, frame)
                    .map_or(0, |table| table.rows.len().saturating_sub(1));
                if let Some(position) = table_editing::cell_start(&session.document, frame, last, 0)
                {
                    session.page_selection = PageSelection::caret(position, page);
                }
                session.status = "Row added".into();
            }
            table_editing::Step::Stay => return Task::none(),
        }
        session.frame_selection = None;
        session.sync_editor_to_page();
        session.scroll_to_caret()
    }

    /// Ctrl+arrows in a table: a cell at a time; with Shift a cell
    /// selection from the anchor's cell.
    fn table_move(
        &mut self,
        down: isize,
        right: isize,
        extend: bool,
    ) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some((frame, row, column)) =
            table_editing::caret_cell(&session.document, session.page_selection)
        else {
            return Task::none();
        };
        let Some((row, column)) =
            table_editing::neighbour(&session.document, frame, (row, column), (down, right))
        else {
            return Task::none();
        };
        let Some(position) = table_editing::cell_start(&session.document, frame, row, column)
        else {
            return Task::none();
        };
        let page = session.page_selection.page;
        session.page_selection = table_editing::extended(
            &session.document,
            session.page_selection,
            position,
            page,
            extend,
        );
        session.sync_editor_to_page();
        session.scroll_to_caret()
    }

    /// Alt+Up and Alt+Down: the caret's rows (or the selected ones) move
    /// past the row above or below, the caret with them.
    fn move_table_rows(&mut self, down: bool) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some((frame, row, _)) =
            table_editing::caret_cell(&session.document, session.page_selection)
        else {
            return Task::none();
        };
        let Some(table) = table_editing::table_of(&session.document, frame) else {
            return Task::none();
        };
        let rows = match table_editing::cell_selection(&session.document, session.page_selection) {
            Some(cells) => cells.expanded(table).rows,
            None => {
                let range = amipro_editing::tables::expanded(
                    table,
                    &amipro_editing::tables::CellRange {
                        rows: row..row + 1,
                        columns: 0..table.columns.len(),
                    },
                );
                range.rows
            }
        };
        let count = table.rows.len();
        let to = if down {
            if rows.end >= count {
                return Task::none();
            }
            rows.end + 1
        } else {
            if rows.start == 0 {
                return Task::none();
            }
            rows.start - 1
        };
        let flow = session.page_selection.focus.flow;
        let page = session.page_selection.page;
        let command = amipro_editing::Command::EditTable {
            frame,
            edit: amipro_editing::tables::TableEdit::MoveRows {
                rows: rows.clone(),
                to,
                copy: false,
            },
        };
        if session.execute_page_command(&command) {
            // The caret stays in its cell, which moved with its row.
            session.page_selection = PageSelection::caret(
                amipro_editing::TextPosition {
                    flow,
                    paragraph: 0,
                    byte: 0,
                },
                page,
            );
            session.page_selection =
                page_editing::valid_selection(&session.document, session.page_selection);
            session.status = if down {
                "Row moved down"
            } else {
                "Row moved up"
            }
            .into();
        }
        session.sync_editor_to_page();
        session.scroll_to_caret()
    }

    /// Applies a frame action from the page surface (`UI_SPEC.md` §11).
    fn update_frame(&mut self, action: FrameAction) -> Task<cosmic::Action<Message>> {
        let mut open_drawing = None;
        let task = self.update_frame_action(action, &mut open_drawing);
        if let Some(frame) = open_drawing {
            // A chart opens its panel, or Draw mode when it was last edited
            // with Drawing; a drawing opens Draw mode.
            if self.is_chart_frame(frame) {
                self.open_chart_frame(frame);
            } else if self.is_equation_frame(frame) {
                return self.open_equation_frame(frame);
            } else {
                self.enter_draw_mode(frame);
            }
        }
        task
    }

    fn update_frame_action(
        &mut self,
        action: FrameAction,
        open_drawing: &mut Option<amipro_document::FrameId>,
    ) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        if let FrameAction::Select {
            frame,
            page,
            extend,
        } = action
        {
            let selection = match &session.frame_selection {
                Some(current) if extend && current.page == page => {
                    current.toggled(&session.document, frame)
                }
                _ => FrameSelection::one(frame, page),
            };
            session.frame_selection = Some(selection);
            session.frame_keys = None;
            session.picture_mode = None;
            session.status = "Frame selected".into();
            return Task::none();
        }
        let Some(selection) = session.frame_selection.clone() else {
            return Task::none();
        };
        let Some(page) = session.page_box(selection.page) else {
            return Task::none();
        };
        let single = selection.single(&session.document);
        match action {
            FrameAction::Select { .. } => {}
            FrameAction::Move { dx, dy, key } => {
                let commands =
                    frame_editing::move_commands(&session.document, &selection, &page, dx, dy);
                drop(session.execute_frame_commands(&commands, key));
            }
            FrameAction::Resize {
                from,
                to,
                with_picture,
            } => {
                if let Some(frame) = single {
                    let mut commands =
                        frame_editing::resize_commands(&session.document, frame, &page, from, to);
                    // Shift and a corner handle: the picture grows or shrinks
                    // with the frame and becomes Custom (P6).
                    if with_picture
                        && let Some(scaling) =
                            session.picture_scaled_with_frame(frame, &page, from, to)
                    {
                        commands
                            .push(amipro_editing::Command::SetPictureScaling { frame, scaling });
                    }
                    drop(session.execute_frame_commands(&commands, false));
                }
            }
            FrameAction::Pan { dx, dy, key } => {
                let Some(frame) = single else {
                    return Task::none();
                };
                if let Some(amipro_document::FrameContent::Picture(picture)) =
                    session.document.frame(frame).map(|frame| &frame.content)
                {
                    let pan = amipro_document::Offset {
                        x: picture.pan.x + dx,
                        y: picture.pan.y + dy,
                    };
                    let command = amipro_editing::Command::MovePictureInFrame { frame, pan };
                    drop(session.execute_frame_commands(&[command], key));
                    session.status = PICTURE_MODE_NOTE.into();
                }
            }
            FrameAction::LeavePicture => {
                session.picture_mode = None;
                session.status = "Frame selected".into();
            }
            FrameAction::ResizeBy { dx, dy } => {
                let Some((frame, frame_box)) = single.and_then(|frame| {
                    page.frames
                        .iter()
                        .find(|frame_box| frame_box.frame == frame)
                        .map(|frame_box| (frame, frame_box))
                }) else {
                    return Task::none();
                };
                let from = frame_editing::rect_of(frame_box.bounds);
                let handle = if dx != 0 {
                    frame_editing::Handle::Right
                } else {
                    frame_editing::Handle::Bottom
                };
                let to = frame_editing::resized(
                    from,
                    handle,
                    dx,
                    dy,
                    false,
                    frame_editing::minimum_size(&session.document, frame),
                );
                let commands =
                    frame_editing::resize_commands(&session.document, frame, &page, from, to);
                drop(session.execute_frame_commands(&commands, true));
            }
            FrameAction::Cycle { backwards } => {
                if let Some(next) = frame_editing::next_frame(
                    &session.document,
                    &page,
                    selection.frames.last().copied(),
                    backwards,
                ) {
                    session.frame_selection = Some(FrameSelection::one(next, selection.page));
                    session.frame_keys = None;
                }
            }
            FrameAction::EnterText { at } => {
                let members = selection.members(&session.document);
                let clicked = at.filter(|(position, _)| {
                    frame_editing::page_frame_of_flow(&session.document, position.flow)
                        .is_some_and(|frame| members.contains(&frame))
                });
                let entry = clicked.or_else(|| {
                    selection
                        .frames
                        .first()
                        .and_then(|frame| frame_editing::text_entry(&session.document, *frame))
                        .map(|position| (position, selection.page))
                });
                match entry {
                    Some((position, page)) => {
                        session.frame_selection = None;
                        session.page_selection = PageSelection::caret(position, page);
                        session.sync_editor_to_page();
                        session.status = "Editing frame text".into();
                    }
                    // A drawing opens in Draw mode (`UI_SPEC.md` §33).
                    None if single.is_some_and(|frame| {
                        matches!(
                            session.document.frame(frame).map(|frame| &frame.content),
                            Some(amipro_document::FrameContent::Picture(picture))
                                if matches!(
                                    picture.format(),
                                    amipro_document::PictureFormat::Drawing
                                        | amipro_document::PictureFormat::Chart
                                        | amipro_document::PictureFormat::Equation
                                )
                        )
                    }) =>
                    {
                        *open_drawing = single;
                    }
                    // A picture has no text: Enter or a double-click puts it
                    // in picture mode (P6); other frames without text open
                    // Frame Properties.
                    None => match single.filter(|frame| {
                        session.document.frame(*frame).is_some_and(|frame| {
                            matches!(frame.content, amipro_document::FrameContent::Picture(_))
                        })
                    }) {
                        Some(frame) => {
                            session.picture_mode = Some(frame);
                            session.frame_keys = None;
                            session.status = PICTURE_MODE_NOTE.into();
                        }
                        None => self.open_rail(),
                    },
                }
            }
            FrameAction::Delete => session.pending_frame_delete = true,
            FrameAction::Deselect => {
                session.picture_mode = None;
                session.page_selection = frame_editing::text_after_frames(
                    &session.document,
                    session.pages.layout(),
                    &selection,
                );
                session.frame_selection = None;
                session.sync_editor_to_page();
                return session.scroll_to_caret();
            }
        }
        Task::none()
    }

    /// Deletes the selected frames and groups, after the user confirmed, as
    /// one undo step.
    fn delete_selected_frames(&mut self) {
        let session = &mut self.sessions[self.active_session];
        session.pending_frame_delete = false;
        let Some(selection) = session.frame_selection.clone() else {
            return;
        };
        let mut commands = Vec::new();
        let mut groups = Vec::new();
        for frame in selection.members(&session.document) {
            if let Some(group) = session.document.frame(frame).and_then(|frame| frame.group) {
                if groups.contains(&group) {
                    continue;
                }
                groups.push(group);
            }
            commands.push(amipro_editing::Command::DeleteFrame { frame });
        }
        drop(session.execute_frame_commands(&commands, false));
        if session.frame_selection.is_none() {
            session.page_selection = frame_editing::text_after_frames(
                &session.document,
                session.pages.layout(),
                &selection,
            );
            session.sync_editor_to_page();
            session.status = "Deleted".into();
        }
    }

    /// Bring to Front, Send to Back, Group, and Ungroup on the selection, as
    /// one undo step.
    fn arrange_frames(&mut self, command: CommandId) {
        let session = &mut self.sessions[self.active_session];
        let Some(selection) = session.frame_selection.clone() else {
            return;
        };
        let document = &session.document;
        let commands: Vec<amipro_editing::Command> = match command {
            CommandId::GroupFrames => selection
                .groupable(document)
                .map(|frames| amipro_editing::Command::Group { frames })
                .into_iter()
                .collect(),
            CommandId::UngroupFrames => selection
                .group(document)
                .map(|group| amipro_editing::Command::Ungroup { group })
                .into_iter()
                .collect(),
            CommandId::BringToFront | CommandId::SendToBack => {
                let mut members = selection.members(document);
                // Bring the back-most first to keep the members' order.
                if command == CommandId::SendToBack {
                    members.reverse();
                }
                let mut groups = Vec::new();
                members
                    .into_iter()
                    .filter(|frame| match document.frame(*frame).and_then(|f| f.group) {
                        Some(group) if groups.contains(&group) => false,
                        Some(group) => {
                            groups.push(group);
                            true
                        }
                        None => true,
                    })
                    .map(|frame| {
                        if command == CommandId::BringToFront {
                            amipro_editing::Command::BringToFront { frame }
                        } else {
                            amipro_editing::Command::SendToBack { frame }
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        drop(session.execute_frame_commands(&commands, false));
    }

    /// Applies an inspector field's typed value as one command (one undo
    /// step), or shows why it is refused and keeps the old value.
    fn submit_inspector_field(&mut self, field: frame_inspector::Field) {
        let inspected = self.inspected_frame();
        let session = &mut self.sessions[self.active_session];
        let Some((frame, page)) =
            inspected.filter(|(frame, _)| session.inspector_frame == Some(*frame))
        else {
            return;
        };
        let Some(text) = session.inspector_drafts.remove(&field) else {
            return;
        };
        let result = if field.is_picture() {
            let natural = session.picture_natural(frame);
            picture_inspector::text_command(&session.document, &page, frame, natural, field, &text)
        } else {
            frame_inspector::text_command(&session.document, &page, frame, field, &text)
        }
        .and_then(|commands| session.execute_frame_commands(&commands, false));
        session.inspector_error = result.err().map(|reason| (field, reason));
    }

    #[cfg(feature = "page-surface")]
    /// A slider of the Image section (DO.5): the setting applies as the
    /// slider moves; the moves of one drag join the first one's undo step.
    fn slide_image(&mut self, field: frame_inspector::Field, value: u8) {
        let Some((frame, _)) = self.inspected_frame() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        let before = session.history.undo_len();
        let result = picture_inspector::slide_command(&session.document, frame, field, value)
            .and_then(|commands| session.execute_frame_commands(&commands, false));
        if let Err(reason) = result {
            session.status = format!("Image change refused: {reason}");
        } else if session.colour_drag.is_none() && session.history.undo_len() > before {
            session.colour_drag = Some(session.history.undo_len());
        }
    }

    /// Applies an inspector choice (a list or a check box) at once.
    fn choose_in_inspector(&mut self, choice: frame_inspector::Choice) {
        let Some((frame, page)) = self.inspected_frame() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        let result = if choice.is_picture() {
            let natural = session.picture_natural(frame);
            picture_inspector::choice_command(&session.document, &page, frame, natural, choice)
        } else {
            frame_inspector::choice_command(&session.document, &page, frame, choice)
        }
        .and_then(|commands| session.execute_frame_commands(&commands, false));
        session.inspector_error = None;
        if let Err(reason) = result {
            session.status = format!("Frame change refused: {reason}");
        }
        // A frame whose placement changed may now be drawn on another page.
        let drawn_on = |page: &amipro_layout::PageBox| {
            page.frames.iter().any(|frame_box| frame_box.frame == frame)
        };
        let still_there = session
            .frame_selection
            .as_ref()
            .and_then(|selection| session.page_box(selection.page))
            .is_some_and(|page| drawn_on(&page));
        if !still_there
            && let Some(number) = session
                .pages
                .layout()
                .pages
                .iter()
                .find(|page| drawn_on(page))
                .map(|page| page.number)
            && let Some(selection) = session.frame_selection.as_mut()
        {
            selection.page = number;
        }
    }

    /// Selects the next frame on the caret's page (or the selection's).
    fn select_next_frame(&mut self) {
        let session = &mut self.sessions[self.active_session];
        let (page_number, current) = match &session.frame_selection {
            Some(selection) => (selection.page, selection.frames.last().copied()),
            None => (
                page_surface::caret_page(
                    &session.document,
                    session.pages.layout(),
                    session.page_selection,
                ),
                None,
            ),
        };
        let next = session
            .page_box(page_number)
            .and_then(|page| frame_editing::next_frame(&session.document, &page, current, false));
        match next {
            Some(frame) => {
                session.frame_selection = Some(FrameSelection::one(frame, page_number));
                session.frame_keys = None;
                session.status = "Frame selected".into();
            }
            None => session.status = "No frames on this page".into(),
        }
    }

    /// What the ruler reports (`UI_SPEC.md` §19): hover names and
    /// positions in the status bar, the guide line while a marker moves,
    /// and each release, click, or menu item as one command and one undo
    /// step on the caret's paragraphs or box.
    #[cfg(feature = "page-surface")]
    fn ruler_event(&mut self, event: ruler_view::RulerEvent) {
        use ruler_view::RulerEvent;
        let session = &mut self.sessions[self.active_session];
        let model = ruler::Ruler::at(
            &session.document,
            session.pages.layout(),
            session.page_selection,
        );
        let describe = |marker: ruler::Marker, at: f32| {
            let columns = model.as_ref().map_or(1, |ruler| ruler.columns.len());
            format!("{} at {}", marker.name(columns), ruler_view::inches(at))
        };
        let command = match event {
            RulerEvent::Hover(Some((marker, at))) => {
                session.status = describe(marker, at);
                session.ruler_status = true;
                None
            }
            RulerEvent::Hover(None) => {
                if session.ruler_status {
                    session.status.clear();
                    session.ruler_status = false;
                }
                None
            }
            RulerEvent::Drag(marker, at) => {
                session.ruler_guide = model.as_ref().map(|ruler| (ruler.page, at));
                session.status = describe(marker, at);
                session.ruler_status = true;
                None
            }
            RulerEvent::Release(marker, at, off) => {
                session.ruler_guide = None;
                session.ruler_status = false;
                model.and_then(|ruler| ruler.release(&session.document, marker, at, off))
            }
            RulerEvent::AddTab(at) => {
                let (kind, leader) = session.ruler_tab;
                model.and_then(|ruler| ruler.add_tab(at, kind, leader))
            }
            RulerEvent::NextTabKind => {
                session.ruler_tab.0 = ruler_view::next_kind(session.ruler_tab.0);
                session.status =
                    format!("New tabs: {}", ruler_view::kind_name(session.ruler_tab.0));
                session.ruler_status = false;
                None
            }
            RulerEvent::Cancel => {
                session.ruler_guide = None;
                None
            }
            RulerEvent::Context { marker, part } => {
                session.ruler_context = Some((marker, part));
                None
            }
            RulerEvent::Cursor(at) => {
                session.ruler_cursor = Some(at);
                session.status = format!("Ruler at {}", ruler_view::inches(at));
                None
            }
            RulerEvent::Leave => {
                session.ruler_cursor = None;
                session.status = "Back to the text".into();
                None
            }
            RulerEvent::Key(key, at) => model.and_then(|ruler| {
                use ruler_view::RulerKey;
                // A key acts as a drag of its marker to the line would.
                let to = |marker: ruler::Marker| {
                    let from = ruler.position(marker);
                    ruler.release(
                        &session.document,
                        marker,
                        ruler.dragged(marker, from, at),
                        false,
                    )
                };
                match key {
                    RulerKey::AddTab => {
                        let (kind, leader) = session.ruler_tab;
                        ruler.add_tab(at, kind, leader)
                    }
                    RulerKey::RemoveTab => (0..ruler.tabs.len())
                        .find(|index| {
                            (ruler.position(ruler::Marker::Tab(*index)) - at).abs()
                                <= ruler::SNAP / 2.0
                        })
                        .and_then(|index| ruler.remove_tab(index)),
                    RulerKey::Indent(marker) => to(marker),
                    RulerKey::Edge { right } => to(ruler::Marker::Edge {
                        column: ruler.caret_column,
                        right,
                    }),
                }
            }),
        };
        if let Some(command) = command {
            self.run_ruler_command(&command);
        }
    }

    /// An item of the ruler's right-click menus, for what the right button
    /// pressed (`UI_SPEC.md` §19).
    #[cfg(feature = "page-surface")]
    fn ruler_menu_action(&mut self, action: ruler_view::RulerMenu) {
        use ruler_view::RulerMenu;
        let session = &mut self.sessions[self.active_session];
        let marker = session.ruler_context.and_then(|(marker, _)| marker);
        let Some(model) = ruler::Ruler::at(
            &session.document,
            session.pages.layout(),
            session.page_selection,
        ) else {
            return;
        };
        let pressed = match marker {
            Some(ruler::Marker::Tab(index)) => model.tabs.get(index).map(|stop| (index, *stop)),
            _ => None,
        };
        let command = match action {
            RulerMenu::TabKind(kind) => {
                pressed.and_then(|(index, stop)| model.set_tab(index, kind, stop.leader))
            }
            RulerMenu::TabLeader(leader) => {
                pressed.and_then(|(index, stop)| model.set_tab(index, stop.kind, leader))
            }
            RulerMenu::RemoveTab => pressed.and_then(|(index, _)| model.remove_tab(index)),
            RulerMenu::ClearTabs => Some(model.clear_tabs()),
            RulerMenu::RemoveRuler => Some(model.remove_ruler()),
            RulerMenu::NewTabKind(kind) => {
                session.ruler_tab.0 = kind;
                None
            }
            RulerMenu::NewTabLeader(leader) => {
                session.ruler_tab.1 = leader;
                None
            }
            RulerMenu::Columns(columns) => model.set_columns(&session.document, columns),
            RulerMenu::EvenColumns => model.even_columns(&session.document),
            RulerMenu::BoxSettings => {
                match model.target {
                    ruler::RulerBox::Table(_) => self.open_table_properties(),
                    ruler::RulerBox::Frame(frame) => {
                        let page = session.pages.layout().pages[model.page].number;
                        session.frame_selection = Some(FrameSelection::one(frame, page));
                        session.frame_keys = None;
                        self.open_rail();
                    }
                    _ => {
                        drop(self.dispatch_command(CommandId::PageLayout));
                    }
                }
                None
            }
        };
        if let Some(command) = command {
            self.run_ruler_command(&command);
        }
    }

    /// The ruler's right-click menu for what the right button last pressed
    /// on it (`UI_SPEC.md` §19): a tab's kind, leader, and Remove; the
    /// upper half's Clear Tabs and Remove Paragraph Ruler with the new
    /// tabs' kind and leader; the kind button's kinds and leaders; the
    /// lower half's Columns, Even Columns, and the box's settings.
    #[cfg(feature = "page-surface")]
    fn ruler_context_menu(&self, model: Option<&ruler::Ruler>) -> Vec<MenuEntry> {
        use ruler_view::{KINDS, LEADERS, Part, RulerMenu};
        let session = &self.sessions[self.active_session];
        let Some(model) = model else {
            return Vec::new();
        };
        let item = |label: String, action: RulerMenu, enabled: bool, checked: Option<bool>| {
            MenuEntry::Item {
                label,
                command: MenuCommand::Ruler(action),
                enabled,
                checked,
            }
        };
        let kinds = |current: amipro_document::TabKind, new: bool| -> Vec<MenuEntry> {
            KINDS
                .iter()
                .map(|kind| {
                    item(
                        format!("{} Tab", ruler_view::kind_name(*kind)),
                        if new {
                            RulerMenu::NewTabKind(*kind)
                        } else {
                            RulerMenu::TabKind(*kind)
                        },
                        true,
                        Some(*kind == current),
                    )
                })
                .collect()
        };
        let leaders = |current: amipro_document::TabLeader, new: bool| -> Vec<MenuEntry> {
            LEADERS
                .iter()
                .map(|leader| {
                    item(
                        ruler_view::leader_name(*leader).to_owned(),
                        if new {
                            RulerMenu::NewTabLeader(*leader)
                        } else {
                            RulerMenu::TabLeader(*leader)
                        },
                        true,
                        Some(*leader == current),
                    )
                })
                .collect()
        };
        let (new_kind, new_leader) = session.ruler_tab;
        let new_tabs = |entries: &mut Vec<MenuEntry>| {
            entries.push(MenuEntry::Folder {
                label: "New Tabs".into(),
                indented: true,
                entries: kinds(new_kind, true),
            });
            entries.push(MenuEntry::Folder {
                label: "New Tabs' Leader".into(),
                indented: true,
                entries: leaders(new_leader, true),
            });
        };
        let (marker, part) = session.ruler_context.unwrap_or((None, Part::Upper));
        let own_ruler = session
            .document
            .flow(model.flow)
            .and_then(|flow| flow.paragraph(model.first))
            .is_some_and(|paragraph| paragraph.format.tabs.is_some());
        let mut entries = Vec::new();
        match (marker, part) {
            (Some(ruler::Marker::Tab(index)), _) if index < model.tabs.len() => {
                let stop = model.tabs[index];
                entries.extend(kinds(stop.kind, false));
                entries.push(MenuEntry::Divider);
                entries.extend(leaders(stop.leader, false));
                entries.push(MenuEntry::Divider);
                entries.push(item("Remove Tab".into(), RulerMenu::RemoveTab, true, None));
            }
            (_, Part::Button) => {
                entries.extend(kinds(new_kind, true));
                entries.push(MenuEntry::Divider);
                entries.extend(leaders(new_leader, true));
            }
            // A table's columns: even them, or its properties (TD6).
            (_, Part::Lower) if matches!(model.target, ruler::RulerBox::Table(_)) => {
                entries.push(item(
                    "Distribute Columns Evenly".into(),
                    RulerMenu::EvenColumns,
                    model.uneven(),
                    Some(false),
                ));
                entries.push(MenuEntry::Divider);
                entries.push(item(
                    "Table Properties…".into(),
                    RulerMenu::BoxSettings,
                    true,
                    Some(false),
                ));
            }
            (_, Part::Lower) if model.edits_columns() => {
                let count = model.columns.len();
                entries.push(MenuEntry::Folder {
                    label: "Columns".into(),
                    indented: true,
                    entries: (1..=8u8)
                        .map(|columns| {
                            item(
                                columns.to_string(),
                                RulerMenu::Columns(columns),
                                true,
                                Some(usize::from(columns) == count),
                            )
                        })
                        .collect(),
                });
                let uneven = model.uneven();
                entries.push(item(
                    "Even Columns".into(),
                    RulerMenu::EvenColumns,
                    uneven,
                    Some(false),
                ));
                entries.push(MenuEntry::Divider);
                let settings = match model.target {
                    ruler::RulerBox::Frame(_) => "Frame Properties…",
                    _ => "Page Layout…",
                };
                entries.push(item(
                    settings.into(),
                    RulerMenu::BoxSettings,
                    true,
                    Some(false),
                ));
            }
            _ => {
                entries.push(item(
                    "Clear Tabs".into(),
                    RulerMenu::ClearTabs,
                    true,
                    Some(false),
                ));
                entries.push(item(
                    "Remove Paragraph Ruler".into(),
                    RulerMenu::RemoveRuler,
                    own_ruler,
                    Some(false),
                ));
                entries.push(MenuEntry::Divider);
                new_tabs(&mut entries);
            }
        }
        entries
    }

    /// Edit › Go To Ruler: the ruler takes the keyboard, its line where the
    /// caret paragraph's indent starts (user's guide, ch. 8). The ruler
    /// shows if it was hidden.
    #[cfg(feature = "page-surface")]
    fn go_to_ruler(&mut self) {
        self.preferences.ruler_visible = true;
        let session = &mut self.sessions[self.active_session];
        let Some(model) = ruler::Ruler::at(
            &session.document,
            session.pages.layout(),
            session.page_selection,
        ) else {
            return;
        };
        let at = ruler::snap(model.position(ruler::Marker::All));
        session.ruler_cursor = Some(at);
        session.status = format!(
            "Ruler at {}: arrows move, Space adds a tab, Del removes, 0–3 indents, [ ] margins, Esc returns",
            ruler_view::inches(at)
        );
    }

    /// A ruler change as one undo step, the selection kept as it was (Ami
    /// Pro's ruler leaves it).
    #[cfg(feature = "page-surface")]
    fn run_ruler_command(&mut self, command: &amipro_editing::Command) {
        let session = &mut self.sessions[self.active_session];
        let selection = session.page_selection;
        if session.execute_page_command(command) {
            session.page_selection = page_editing::valid_selection(&session.document, selection);
            session.sync_editor_to_page();
        }
    }

    /// Bold or Italic over the page selection, in any flow.
    /// Format › Align Left, Center, Align Right, and Justify (Ami Pro's
    /// Text › Alignment): the selected paragraphs, or the caret's, take
    /// `alignment` over their styles' as one undo step (gap A2).
    fn align_page_paragraphs(&mut self, alignment: Alignment) {
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        session.execute_page_command(&amipro_editing::Command::SetParagraphAlignment {
            flow: start.flow,
            first: start.paragraph,
            last: end.paragraph,
            alignment: Some(alignment),
        });
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
    }

    /// Format › Bullets and Numbering (`UI_SPEC.md` §21, B1): the selected
    /// paragraphs, or the caret's, take the document's list style of that
    /// kind (made from Ami Pro's default when it has none), or Body Text
    /// when every one already shows it; one undo step.
    #[cfg(feature = "page-surface")]
    fn toggle_page_list(&mut self, kind: amipro_editing::lists::ListKind) {
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        let command = amipro_editing::lists::toggle_list(
            &session.document,
            start.flow,
            start.paragraph,
            end.paragraph,
            kind,
        );
        if !session.execute_page_command(&command) {
            return;
        }
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
        let label = match kind {
            amipro_editing::lists::ListKind::Bullets => "Bullets",
            amipro_editing::lists::ListKind::Numbering => "Numbering",
        };
        session.status = match session
            .document
            .flow(start.flow)
            .and_then(|flow| flow.paragraphs.get(start.paragraph))
            .and_then(|paragraph| session.document.styles().get(&paragraph.style))
        {
            Some(style) if amipro_editing::lists::shown_list(style) == Some(kind) => {
                format!("{label}: {}", style.name)
            }
            _ => format!("{label} off"),
        };
    }

    /// Format › Promote and Demote (`UI_SPEC.md` §22, O1, O2): the selected
    /// paragraphs, or the caret's, take the style one outline level up or
    /// down (a nested list's next depth), made as needed; one undo step.
    #[cfg(feature = "page-surface")]
    /// View › Outline ▸ Outline Navigator (OB.6): the rail shows the
    /// headings; the other rail panels give way, as each panel's opening
    /// does.
    #[cfg(feature = "page-surface")]
    fn open_outline_navigator(&mut self) {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.outline_navigator = Some(outline_navigator::OutlineNavigator::default());
        self.open_rail();
    }

    /// Edit › Links… (DO.3): the rail lists the OLE objects and links; the
    /// other rail panels give way.
    #[cfg(feature = "page-surface")]
    fn open_links(&mut self) {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.links_panel = Some(links_panel::LinksPanel::default());
        self.open_rail();
    }

    /// Edit › Paste Special… (DO.4): the rail lists the formats the
    /// clipboard offers, found by looking for each in turn.
    #[cfg(feature = "page-surface")]
    fn open_paste_special(&mut self) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.paste_special = Some(paste_special::PasteSpecial {
            pending: paste_special::CANDIDATES.len(),
            ..paste_special::PasteSpecial::default()
        });
        session.status = "Choose a format to paste".into();
        let session_id = session.id;
        self.open_rail();
        Self::look_at_clipboard(session_id)
    }

    /// One look at the clipboard for each format Paste Special lists.
    #[cfg(feature = "page-surface")]
    fn look_at_clipboard(session_id: u64) -> Task<cosmic::Action<Message>> {
        fn look<const K: usize>(session_id: u64) -> Task<cosmic::Action<Message>> {
            cosmic::iced::clipboard::read_data::<paste_special::Offered<K>>().map(move |found| {
                cosmic::Action::from(Message::PasteSpecialFound {
                    session_id,
                    slot: K,
                    data: found.map(|found| (found.0, found.1)),
                })
            })
        }
        Task::batch([
            cosmic::iced::clipboard::read().map(move |text| {
                cosmic::Action::from(Message::PasteSpecialFound {
                    session_id,
                    slot: 0,
                    data: text
                        .filter(|text| !text.is_empty())
                        .map(|text| (text.into_bytes(), "text/plain".into())),
                })
            }),
            look::<1>(session_id),
            look::<2>(session_id),
            look::<3>(session_id),
            look::<4>(session_id),
            look::<5>(session_id),
            look::<6>(session_id),
            look::<7>(session_id),
            look::<8>(session_id),
            look::<9>(session_id),
        ])
    }

    /// An answer to one look (DO.4). When every look has answered and none
    /// found anything, the clipboard is looked at once more after a moment
    /// (the first read can run before the window has the clipboard).
    #[cfg(feature = "page-surface")]
    fn paste_special_found(
        &mut self,
        session_id: u64,
        slot: usize,
        data: Option<(Vec<u8>, String)>,
    ) -> Task<cosmic::Action<Message>> {
        let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        else {
            return Task::none();
        };
        let Some(panel) = session.paste_special.as_mut() else {
            return Task::none();
        };
        match data {
            Some((bytes, mime)) => panel.found(paste_special::Format { slot, bytes, mime }),
            None => panel.missed(),
        }
        if panel.empty() && !panel.retried {
            panel.retried = true;
            panel.pending = paste_special::CANDIDATES.len();
            return Task::future(async move {
                let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    let _ = done.send(());
                });
                let _ = wait.await;
                cosmic::Action::from(Message::PasteSpecialRetry(session_id))
            });
        }
        Task::none()
    }

    /// The Paste Special panel's actions: close it, choose a row, or paste
    /// the row as what it is (a picture or text); a format the app cannot
    /// hold shows Ami Pro's message 3092 and leaves the panel open.
    #[cfg(feature = "page-surface")]
    fn update_paste_special(
        &mut self,
        action: paste_special::PasteSpecialAction,
    ) -> Task<cosmic::Action<Message>> {
        use paste_special::{Kind, PasteSpecialAction};
        let close = |app: &mut Self| {
            app.sessions[app.active_session].paste_special = None;
            if app.rail_open() {
                app.toggle_rail();
            }
        };
        match action {
            PasteSpecialAction::Close => {
                close(self);
                Task::none()
            }
            PasteSpecialAction::Select(index) => {
                if let Some(panel) = self.sessions[self.active_session].paste_special.as_mut()
                    && index < panel.formats.len()
                {
                    panel.selected = Some(index);
                }
                Task::none()
            }
            PasteSpecialAction::Paste(index) => {
                let Some(format) = self.sessions[self.active_session]
                    .paste_special
                    .as_ref()
                    .and_then(|panel| panel.formats.get(index.or(panel.selected)?).cloned())
                else {
                    return Task::none();
                };
                let refused = |app: &mut Self| {
                    app.sessions[app.active_session].status = paste_special::CANNOT_ACCEPT.into();
                    Task::none()
                };
                match format.kind() {
                    Kind::Text | Kind::Html => {
                        let text = String::from_utf8_lossy(&format.bytes).into_owned();
                        let text = if format.kind() == Kind::Html {
                            paste_special::html_text(&text)
                        } else {
                            text
                        };
                        if text.is_empty() {
                            return refused(self);
                        }
                        close(self);
                        self.insert_clipboard_text(text);
                        Task::none()
                    }
                    Kind::RichText => {
                        if !format.bytes.starts_with(b"{\\rtf") {
                            return refused(self);
                        }
                        if self.refuse_in_formula_cell() {
                            return Task::none();
                        }
                        close(self);
                        let purpose = self.insert_purpose(true);
                        self.start_import(
                            import::Source::Clipboard(format.bytes),
                            import::paste_options(),
                            purpose,
                        )
                    }
                    Kind::Image => {
                        let extension = format.extension();
                        let readable = pictures::natural_of(
                            amipro_document::PictureFormat::from_extension(extension),
                            &format.bytes,
                        )
                        .is_some();
                        if !readable {
                            return refused(self);
                        }
                        if self.refuse_in_formula_cell() {
                            return Task::none();
                        }
                        close(self);
                        self.insert_picture_bytes(format.bytes, extension, None);
                        Task::none()
                    }
                    Kind::Files => {
                        let Some(path) = picture_insert::dropped_pictures(&format.bytes)
                            .into_iter()
                            .next()
                        else {
                            return refused(self);
                        };
                        if self.refuse_in_formula_cell() {
                            return Task::none();
                        }
                        close(self);
                        self.insert_image_file(&path, false);
                        Task::none()
                    }
                }
            }
        }
    }

    /// The Links panel's actions: close it, or select a row's frame (its
    /// Frame Properties show the Object section).
    #[cfg(feature = "page-surface")]
    fn update_links(&mut self, action: links_panel::LinksAction) -> Task<cosmic::Action<Message>> {
        match action {
            links_panel::LinksAction::Close => {
                self.sessions[self.active_session].links_panel = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
                Task::none()
            }
            links_panel::LinksAction::Select(index) => {
                let rows = links_panel::rows(&self.document);
                let Some(row) = rows.get(index) else {
                    return Task::none();
                };
                let frame = row.frame;
                let page = self
                    .pages
                    .layout()
                    .pages
                    .iter()
                    .find(|page| page.frames.iter().any(|found| found.frame == frame))
                    .map(|page| page.number);
                let Some(page) = page else {
                    self.status = "That object is not on a page".into();
                    return Task::none();
                };
                let task = self.go_to_page(page, true);
                let session = &mut self.sessions[self.active_session];
                if let Some(panel) = session.links_panel.as_mut() {
                    panel.selected = Some(index);
                }
                session.frame_selection = Some(FrameSelection::one(frame, page));
                session.status = "Frame selected".into();
                task
            }
        }
    }

    /// Puts the caret at the start of main paragraph `paragraph` and shows
    /// it (a navigator click, or its Up and Down).
    #[cfg(feature = "page-surface")]
    fn go_to_heading(&mut self, paragraph: usize) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let at = amipro_editing::TextPosition {
            flow: session.document.main_flow_id(),
            paragraph,
            byte: 0,
        };
        let page = session.pages.page_of(paragraph, 0);
        session.frame_selection = None;
        session.page_selection = PageSelection::caret(at, page);
        session.sync_editor_to_page();
        session.scroll_to_caret()
    }

    /// The Outline navigator's actions (OB.6).
    #[cfg(feature = "page-surface")]
    fn update_outline_navigator(
        &mut self,
        action: outline_navigator::NavigatorAction,
    ) -> Task<cosmic::Action<Message>> {
        use outline_navigator::{NavigatorAction as A, Target};
        let session = &mut self.sessions[self.active_session];
        let Some(navigator) = session.outline_navigator.as_mut() else {
            return Task::none();
        };
        match action {
            A::Close => {
                session.outline_navigator = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            A::Press(paragraph) => {
                navigator.pressed = Some(paragraph);
                navigator.over = Some(Target::Before(paragraph));
                navigator.keys = true;
            }
            A::Over(target) => {
                if navigator.pressed.is_some() {
                    navigator.over = Some(target);
                }
            }
            A::Release(target) => {
                navigator.over = None;
                let Some(from) = navigator.pressed.take() else {
                    return Task::none();
                };
                if target == Target::Before(from) {
                    return self.go_to_heading(from);
                }
                let main = session.document.main_flow_id();
                let count = session.document.paragraphs().len();
                let to = match target {
                    Target::Before(heading) => heading,
                    Target::End => count,
                };
                let levels = amipro_editing::sections::levels(&session.document, main);
                let own = amipro_editing::sections::section(&levels, from);
                let command = amipro_editing::Command::MoveSectionTo {
                    flow: main,
                    paragraph: from,
                    to,
                };
                if own.contains(&to) || to == own.end || !session.execute_page_command(&command) {
                    session.status = "The section is already there".into();
                    return Task::none();
                }
                let landed = if to > own.end { to - own.len() } else { to };
                session.status = "Moved the section".into();
                return self.go_to_heading(landed);
            }
            A::Toggle(paragraph) => {
                let document = session.document.clone();
                let collapsed = outline_navigator::entries(&document, &session.outline_state)
                    .iter()
                    .any(|entry| entry.paragraph == paragraph && entry.collapsed);
                let changed = if collapsed {
                    session.outline_state.expand(&document, paragraph)
                } else {
                    session.outline_state.contract(&document, paragraph)
                };
                if changed {
                    session.refresh_rich_layout();
                    session.leave_hidden_text(false);
                }
            }
        }
        Task::none()
    }

    /// Up, Down, Left, and Right while the navigator holds the keys: the
    /// heading before or after, Contract, Expand.
    #[cfg(feature = "page-surface")]
    fn navigator_key(&mut self, key: keyboard::key::Named) -> Task<cosmic::Action<Message>> {
        use keyboard::key::Named;
        let session = &self.sessions[self.active_session];
        if !session
            .outline_navigator
            .as_ref()
            .is_some_and(|navigator| navigator.keys)
            || !self.rail_open()
        {
            return Task::none();
        }
        let entries = outline_navigator::entries(&session.document, &session.outline_state);
        let focus = session.page_selection.focus;
        let current = outline_navigator::current(&entries, focus.paragraph);
        let index =
            current.and_then(|current| entries.iter().position(|entry| entry.paragraph == current));
        match key {
            Named::ArrowUp | Named::ArrowDown => {
                let next = match (index, key) {
                    (None, _) => entries.first(),
                    (Some(index), Named::ArrowUp) => entries.get(index.saturating_sub(1)),
                    (Some(index), _) => entries.get(index + 1),
                };
                match next {
                    Some(entry) => self.go_to_heading(entry.paragraph),
                    None => Task::none(),
                }
            }
            Named::ArrowLeft | Named::ArrowRight => {
                let Some(entry) = index.and_then(|index| entries.get(index)) else {
                    return Task::none();
                };
                if entry.has_children && entry.collapsed == (key == Named::ArrowRight) {
                    return self.update_outline_navigator(
                        outline_navigator::NavigatorAction::Toggle(entry.paragraph),
                    );
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// View › Outline ▸ Expand and Contract (OB.4): the caret's heading's
    /// whole section shown or hidden, in Outline mode; the caret then leaves
    /// hidden text. Body text has no section (nothing happens, as in Ami
    /// Pro).
    #[cfg(feature = "page-surface")]
    fn outline_expand(&mut self, expand: bool) {
        let session = &mut self.sessions[self.active_session];
        if session.outline.is_none() {
            session.status =
                "Expand and Contract work in Outline mode (View › Outline Mode)".into();
            return;
        }
        let caret = session.page_selection.focus;
        if caret.flow != session.document.main_flow_id() {
            return;
        }
        let document = session.document.clone();
        let changed = if expand {
            session.outline_state.expand(&document, caret.paragraph)
        } else {
            session.outline_state.contract(&document, caret.paragraph)
        };
        if changed {
            session.refresh_rich_layout();
            session.leave_hidden_text(true);
            session.status = if expand {
                "Expanded the heading's section".into()
            } else {
                "Contracted the heading's section".into()
            };
        }
    }

    /// View › Outline ▸ Show n Levels and Show All Levels (OB.4).
    #[cfg(feature = "page-surface")]
    fn outline_show_levels(&mut self, levels: Option<u8>) {
        let session = &mut self.sessions[self.active_session];
        if session.outline.is_none() {
            session.status = "Show levels works in Outline mode (View › Outline Mode)".into();
            return;
        }
        session.outline_state.show_levels(levels);
        session.refresh_rich_layout();
        session.leave_hidden_text(false);
        session.status = match levels {
            Some(1) => "Showing level 1".into(),
            Some(levels) => format!("Showing levels 1 to {levels}"),
            None => "Showing all levels".into(),
        };
    }

    /// Outline › Move Up and Move Down (OM3): the caret's heading with its
    /// section, a body paragraph alone, one undo step; the caret stays in
    /// the moved paragraph (in a table the page moves rows instead).
    #[cfg(feature = "page-surface")]
    fn move_section(&mut self, down: bool) {
        use amipro_editing::sections;
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let caret = session.page_selection.focus;
        let levels = sections::levels(&session.document, caret.flow);
        if caret.paragraph >= levels.len() {
            return;
        }
        let Some(other) = sections::neighbour(&levels, caret.paragraph, down) else {
            session.status = if down {
                "The section is already at the end"
            } else {
                "The section is already at the start"
            }
            .into();
            return;
        };
        let own = sections::section(&levels, caret.paragraph);
        let command = amipro_editing::Command::MoveSection {
            flow: caret.flow,
            paragraph: own.start,
            down,
        };
        if !session.execute_page_command(&command) {
            return;
        }
        let shift = caret.paragraph - own.start;
        let paragraph = if down {
            own.start + other.len() + shift
        } else {
            other.start + shift
        };
        let at = amipro_editing::TextPosition { paragraph, ..caret };
        let page = session.page_selection.page;
        session.page_selection =
            page_editing::valid_selection(&session.document, PageSelection::caret(at, page));
        session.sync_editor_to_page();
        session.status = if down {
            "Moved the section down".into()
        } else {
            "Moved the section up".into()
        };
    }

    fn change_page_level(&mut self, change: amipro_editing::outline::LevelChange) {
        use amipro_editing::outline::LevelChange;
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        let command = amipro_editing::Command::ChangeLevel {
            flow: start.flow,
            first: start.paragraph,
            last: end.paragraph,
            change,
        };
        if !session.execute_page_command(&command) {
            session.status = match change {
                LevelChange::Promote => "Already at the top level",
                LevelChange::Demote => "Already at the lowest level",
            }
            .into();
            return;
        }
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
        if let Some(style) = session
            .document
            .flow(start.flow)
            .and_then(|flow| flow.paragraphs.get(start.paragraph))
            .and_then(|paragraph| session.document.styles().get(&paragraph.style))
        {
            let level = style.bullets.level;
            session.status = if level > 0 {
                format!("{}, level {level}", style.name)
            } else {
                style.name.clone()
            };
        }
    }

    /// Switches `attribute` over the page selection; at a caret, for the
    /// text typed next (VM fixture C1).
    fn toggle_page_attribute(&mut self, attribute: CharacterAttribute) {
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let on = session.page_attribute_state(attribute) != ToggleState::On;
        let state = if on { "on" } else { "off" };
        let label = attribute_label(attribute);
        let selection = session.page_selection;
        if selection.is_caret() {
            session.typing_format_mut().set(attribute, on);
            session.status = format!("{label} {state} for typing");
            return;
        }
        let (start, end) = selection.ordered();
        session.execute_page_command(&amipro_editing::Command::SetCharacterAttribute {
            start,
            end,
            attribute,
            on,
        });
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
        session.status = format!("{label} {state}");
    }

    /// Ami Pro's Text › Normal: the selection's character formatting and
    /// its paragraphs' formatting back to their styles (VM fixture C1); at a
    /// caret, the paragraph's formatting and the typing choices.
    fn normal_page_text(&mut self) {
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        session.typing = None;
        session
            .execute_page_command(&amipro_editing::Command::ClearDirectFormatting { start, end });
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
        session.status = "Normal".into();
    }

    /// Applies a Special Effects dialog action.
    fn update_special_effects(&mut self, action: &character_dialogs::EffectsAction) {
        use character_dialogs::EffectsAction;
        let session = &mut self.sessions[self.active_session];
        let Some(effects) = &mut session.special_effects else {
            return;
        };
        match action {
            EffectsAction::Toggle(index, on) => {
                if let Some(chosen) = effects.chosen.get_mut(*index) {
                    *chosen = *on;
                }
            }
            EffectsAction::Overstrike(on) => effects.overstrike = *on,
            EffectsAction::OverstrikeText(text) => {
                // One character: the last typed; typing one turns it on.
                effects.overstrike_text = text.chars().last().map(String::from).unwrap_or_default();
                effects.overstrike |= !effects.overstrike_text.is_empty();
            }
            EffectsAction::Cancel => session.special_effects = None,
            EffectsAction::Apply => {
                let changes = effects.changes();
                session.special_effects = None;
                session.apply_attribute_changes(&changes);
            }
        }
    }

    /// Applies a Font dialog action.
    /// A colour button, its panel, and the picker (`UI_SPEC.md` §23):
    /// a swatch applies its colour and closes the panel; a drag in the
    /// picker applies the colour live as one undo step (C3); each colour
    /// chosen becomes Recent's newest (C4).
    fn update_colour(
        &mut self,
        action: colour_control::ColourAction,
    ) -> Task<cosmic::Action<Message>> {
        use colour_control::{ColourAction, Panel};
        use cosmic::widget::color_picker::ColorPickerUpdate;
        let session = &mut self.sessions[self.active_session];
        match action {
            ColourAction::Toggle(target, current) => {
                session.colour_drag = None;
                if session
                    .colour_panel
                    .as_ref()
                    .is_some_and(|panel| panel.target == target)
                {
                    session.colour_panel = None;
                } else {
                    let in_use = colours::document_colours(&session.document);
                    session.colour_panel = Some(Panel::new(target, current, in_use));
                }
            }
            ColourAction::Choose(colour) => {
                let Some(target) = session.colour_panel.as_ref().map(|panel| panel.target) else {
                    return Task::none();
                };
                session.colour_drag = None;
                self.apply_colour(target, colour);
                self.remember_colour(colour);
                self.sessions[self.active_session].colour_panel = None;
            }
            ColourAction::More => {
                if let Some(panel) = session.colour_panel.as_mut() {
                    panel.more = !panel.more;
                }
            }
            ColourAction::Picker(update) => {
                let Some(panel) = session.colour_panel.as_mut() else {
                    return Task::none();
                };
                let target = panel.target;
                let task = panel.picker.update::<Message>(update.clone());
                let finished = matches!(
                    update,
                    ColorPickerUpdate::ActionFinished | ColorPickerUpdate::AppliedColor
                );
                let colour = match update {
                    ColorPickerUpdate::ActiveColor(hsv) => Some(colours::from_hsv(hsv)),
                    _ if finished => panel.picker.get_applied_color().map(colours::from_color),
                    _ => None,
                };
                if let Some(colour) = colour {
                    let before = session.history.undo_len();
                    self.apply_colour(target, colour);
                    let session = &mut self.sessions[self.active_session];
                    if finished {
                        session.colour_drag = None;
                        if let Some(panel) = session.colour_panel.as_mut() {
                            // A fresh picker: its own recent row would repeat
                            // the panel's.
                            let more = panel.more;
                            *panel = Panel::new(
                                target,
                                colour,
                                colours::document_colours(&session.document),
                            );
                            panel.more = more;
                        }
                        self.remember_colour(colour);
                    } else if session.colour_drag.is_none() && session.history.undo_len() > before {
                        session.colour_drag = Some(session.history.undo_len());
                    }
                }
                return task.map(cosmic::Action::App);
            }
        }
        Task::none()
    }

    /// Applies `colour` to what `target` colours, through the panel's own
    /// action, so that its rules (and undo) are the panel's.
    fn apply_colour(&mut self, target: colour_control::Target, colour: amipro_document::Colour) {
        use colour_control::Target;
        use frame_inspector::Field;
        let field = match target {
            Target::FontText => {
                self.update_font_dialog(&character_dialogs::FontAction::Colour(colour));
                return;
            }
            Target::StyleText => {
                self.update_style_properties(style_properties::StyleAction::Colour(colour));
                return;
            }
            Target::StyleRule => {
                self.update_style_properties(style_properties::StyleAction::RuleColour(colour));
                return;
            }
            Target::PageLines => {
                self.update_page_layout_properties(
                    page_layout_properties::LayoutAction::LineColour(colour),
                );
                return;
            }
            Target::TableFill | Target::TableLines => {
                self.table_colour(target, colour);
                return;
            }
            Target::DrawLine | Target::DrawFill | Target::DrawBackground | Target::DrawText => {
                self.draw_colour(target, colour);
                return;
            }
            Target::ChartColour => {
                self.chart_colour(colour);
                return;
            }
            Target::FrameLine => Field::LineColour,
            Target::FrameShadow => Field::ShadowColour,
            Target::FrameBackground => Field::Background,
            Target::FrameColumnLine => Field::ColumnLineColour,
        };
        let Some((frame, page)) = self.inspected_frame() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        let result = frame_inspector::text_command(
            &session.document,
            &page,
            frame,
            field,
            &colours::hex(colour),
        )
        .and_then(|commands| session.execute_frame_commands(&commands, false));
        if let Err(reason) = result {
            session.status = format!("Frame change refused: {reason}");
        }
    }

    /// Closes the colour panel when what holds its button no longer shows:
    /// the Font dialog, Style Properties, Page Layout Properties, or Frame
    /// Properties on one frame.
    fn sync_colour_panel(&mut self) {
        use colour_control::Target;
        let Some(target) = self.colour_panel.as_ref().map(|panel| panel.target) else {
            return;
        };
        let shows = match target {
            Target::FontText => self.font_dialog.is_some(),
            Target::StyleText | Target::StyleRule => self.style_properties.is_some(),
            Target::PageLines => self.page_layout_properties.is_some(),
            Target::TableFill | Target::TableLines => self.table_properties.is_some(),
            Target::DrawLine | Target::DrawFill | Target::DrawBackground | Target::DrawText => {
                self.drawing_open()
            }
            Target::ChartColour => self.chart_panel.is_some(),
            Target::FrameLine
            | Target::FrameShadow
            | Target::FrameBackground
            | Target::FrameColumnLine => self.inspected_frame().is_some(),
        };
        if !shows {
            let session = &mut self.sessions[self.active_session];
            session.colour_panel = None;
            session.colour_drag = None;
        }
    }

    /// What views need for colour buttons: the open panel and Recent.
    fn colour_context(&self) -> colour_control::Context<'_> {
        colour_control::Context {
            open: self.colour_panel.as_ref(),
            recent: &self.preferences.recent_colours,
        }
    }

    /// `colour` becomes Recent's newest (C4), kept in the settings.
    fn remember_colour(&mut self, colour: amipro_document::Colour) {
        let mut recent = colours::Recent::from_numbers(&self.preferences.recent_colours);
        recent.push(colour);
        self.preferences.recent_colours = recent.to_numbers();
    }

    fn update_font_dialog(&mut self, action: &character_dialogs::FontAction) {
        use character_dialogs::FontAction;
        let session = &mut self.sessions[self.active_session];
        let Some(dialog) = &mut session.font_dialog else {
            return;
        };
        match action {
            FontAction::Cancel => session.font_dialog = None,
            FontAction::Apply => match dialog.choice() {
                Err(error) => dialog.error = Some(error),
                Ok(choice) => {
                    session.font_dialog = None;
                    session.apply_font(choice);
                }
            },
            _ => dialog.update(action),
        }
    }

    /// A paragraph style over the paragraphs of the page selection.
    fn apply_page_style(&mut self, style: StyleId) {
        let session = &mut self.sessions[self.active_session];
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        session.execute_page_command(&amipro_editing::Command::SetParagraphStyle {
            flow: start.flow,
            first: start.paragraph,
            last: end.paragraph,
            style,
        });
        session.page_selection = page_editing::valid_selection(&session.document, selection);
        session.sync_editor_to_page();
        if let Some(style) = session.document.styles().get(&style) {
            session.status = format!("Applied {}", style.name);
        }
    }

    /// The Frame Properties inspector for the selected frames, in Ami Pro's
    /// Modify Frame Layout order (`UI_SPEC.md` §11). Typed values apply on
    /// Enter or when the field loses focus; lists and check boxes at once.
    fn frame_inspector_view(&self) -> Option<Element<'_, Message>> {
        use frame_inspector::{
            Choice, Field, LINE_POSITION_LABELS, LINE_POSITIONS, PlacementKind, SHADOW_LABELS,
            ShadowSide, WRAP_LABELS, WRAPS,
        };
        const SIDES: [&str; 4] = ["Left line", "Right line", "Top line", "Bottom line"];

        let selection = self.frame_selection.as_ref()?;
        fn rail(children: Vec<Element<'_, Message>>) -> Element<'_, Message> {
            widget::column::with_children(vec![
                rail_header("Frame Properties"),
                rail_scroller(widget::column::with_children(children).spacing(8).into()),
            ])
            .spacing(8)
            .padding([8, 8, 8, 8])
            .height(Length::Fill)
            .into()
        }
        let mut children: Vec<Element<'_, Message>> = Vec::new();
        let inspected = self.inspected_frame().and_then(|(frame, page)| {
            frame_inspector::fields(&self.document, &page, frame).map(|fields| (frame, fields))
        });
        let Some((frame, shown)) = inspected else {
            children.push(
                widget::text::body(format!(
                    "{} frames selected. Select one frame to change its properties.",
                    selection.members(&self.document).len()
                ))
                .into(),
            );
            return Some(rail(children));
        };
        let draft = |field: Field| {
            (self.inspector_frame == Some(frame))
                .then(|| self.inspector_drafts.get(&field).cloned())
                .flatten()
                .unwrap_or_else(|| shown.value(field).to_owned())
        };
        let error = |field: Field| {
            self.inspector_error
                .as_ref()
                .filter(|(failed, _)| *failed == field)
                .map(|(_, reason)| reason.clone())
        };
        let input = |label: &'static str, field: Field, editable: bool| -> Element<'_, Message> {
            let mut input = widget::text_input(label, draft(field)).width(Length::Fill);
            if editable {
                input = input
                    .on_input(move |text| Message::InspectorInput(field, text))
                    .on_submit(move |_| Message::InspectorSubmit(field))
                    .on_unfocus(Message::InspectorSubmit(field));
            }
            let mut column = vec![widget::text::caption(label).into(), input.into()];
            if let Some(reason) = error(field) {
                column.push(widget::text::caption(reason).into());
            }
            widget::column::with_children(column).spacing(2).into()
        };
        let section =
            |title: &'static str| -> Element<'_, Message> { widget::text::heading(title).into() };
        let picture_input = |label: &'static str,
                             field: Field,
                             view: &picture_inspector::PictureView|
         -> Element<'_, Message> {
            let value = (self.inspector_frame == Some(frame))
                .then(|| self.inspector_drafts.get(&field).cloned())
                .flatten()
                .unwrap_or_else(|| view.value(field).to_owned());
            let input = widget::text_input(label, value)
                .width(Length::Fill)
                .on_input(move |text| Message::InspectorInput(field, text))
                .on_submit(move |_| Message::InspectorSubmit(field))
                .on_unfocus(Message::InspectorSubmit(field));
            let mut column = vec![widget::text::caption(label).into(), input.into()];
            if let Some(reason) = error(field) {
                column.push(widget::text::caption(reason).into());
            }
            widget::column::with_children(column).spacing(2).into()
        };

        // Frame Properties › Object (`UI_SPEC.md` §42.1, OD7): what the OLE
        // object is, read-only; the server is not run here.
        if let Some(object) = picture_inspector::object_view(&self.document, frame) {
            children.push(section("Object"));
            for (label, value) in &object.rows {
                children.push(
                    widget::column::with_children(vec![
                        widget::text::caption(*label).into(),
                        widget::text::body(value.clone()).into(),
                    ])
                    .into(),
                );
            }
            children.push(widget::text::caption(picture_inspector::OBJECT_CAPTION).into());
            if object.convertible {
                children.push(rail_button(
                    "Convert Object to Picture",
                    Some(Message::InspectorChoice(Choice::ObjectToPicture)),
                ));
                children.push(widget::text::caption(picture_inspector::CONVERT_NOTE).into());
            }
        }

        // Frame Properties › Picture (`UI_SPEC.md` §25, P5, P7): Ami Pro's
        // Graphics Scaling, first for a picture frame.
        let natural = self.picture_natural(frame);
        if let Some(view) = self
            .inspected_frame()
            .and_then(|(_, page)| picture_inspector::view(&self.document, &page, frame, natural))
        {
            children.push(section("Picture"));
            children.push(widget::text::caption(view.about.clone()).into());
            if let Some(path) = &view.linked {
                children.push(widget::text::caption(format!("Linked to {path}")).into());
            }
            children.push(widget::text::caption("Sizing").into());
            children.push(
                widget::dropdown(
                    &picture_inspector::SIZING_LABELS,
                    picture_inspector::SIZINGS
                        .iter()
                        .position(|sizing| *sizing == view.sizing),
                    |index| {
                        Message::InspectorChoice(Choice::PictureSizing(
                            picture_inspector::SIZINGS[index],
                        ))
                    },
                )
                .width(Length::Fill)
                .into(),
            );
            children.push(
                widget::text::caption(picture_inspector::sizing_note(
                    view.sizing,
                    view.keep_aspect,
                ))
                .into(),
            );
            children.push(picture_input(
                "Percentage of its size (%)",
                Field::PicturePercent,
                &view,
            ));
            children.push(picture_input("Width (in)", Field::PictureWidth, &view));
            children.push(picture_input("Height (in)", Field::PictureHeight, &view));
            children.push(
                widget::checkbox(view.keep_aspect)
                    .label("Keep aspect ratio")
                    .on_toggle(|keep| Message::InspectorChoice(Choice::PictureAspect(keep)))
                    .into(),
            );
            if view.rotatable {
                children.push(picture_input(
                    "Rotate (degrees)",
                    Field::PictureRotation,
                    &view,
                ));
            }
            children.push(picture_input(
                "Moved right (in)",
                Field::PictureRight,
                &view,
            ));
            children.push(picture_input("Moved down (in)", Field::PictureDown, &view));
            children.push(rail_button(
                "Reset",
                Some(Message::InspectorChoice(Choice::PictureReset)),
            ));
            children.push(rail_button(
                "Replace Picture…",
                Some(Message::Command(CommandId::Image)),
            ));
            children.push(rail_button(
                "Fit Frame to Picture",
                Some(Message::InspectorChoice(Choice::PictureFitFrame)),
            ));
            children.push(rail_button("Save Picture As…", Some(Message::SavePicture)));
            children.push(picture_input(
                "Description",
                Field::PictureDescription,
                &view,
            ));
            children.push(
                widget::text::caption("Kept in .amipro files; Ami Pro's cannot store it.").into(),
            );
            if let Some(image) = &view.image {
                children.push(section("Image"));
                for (field, label) in picture_inspector::IMAGE_SLIDERS {
                    let value = (self.inspector_frame == Some(frame))
                        .then(|| self.inspector_drafts.get(&field).cloned())
                        .flatten()
                        .unwrap_or_else(|| image.value(field).to_owned());
                    let number: u8 = image.value(field).parse().unwrap_or(0);
                    let input = widget::text_input("", value)
                        .width(Length::Fixed(64.0))
                        .on_input(move |text| Message::InspectorInput(field, text))
                        .on_submit(move |_| Message::InspectorSubmit(field))
                        .on_unfocus(Message::InspectorSubmit(field));
                    let slider = widget::slider(0..=100u8, number, move |value| {
                        Message::ImageSlide(field, value)
                    })
                    .on_release(Message::ImageSlideEnd)
                    .width(Length::Fill);
                    let mut column = vec![
                        widget::text::caption(label).into(),
                        widget::row::with_children(vec![input.into(), slider.into()])
                            .spacing(8)
                            .align_y(cosmic::iced::Alignment::Center)
                            .into(),
                    ];
                    if let Some(reason) = error(field) {
                        column.push(widget::text::caption(reason).into());
                    }
                    children.push(widget::column::with_children(column).spacing(2).into());
                }
                children.push(
                    widget::checkbox(image.adjust.invert)
                        .label("Invert")
                        .on_toggle(|on| Message::InspectorChoice(Choice::ImageInvert(on)))
                        .into(),
                );
                children.push(widget::text::caption("Halftone (a print setting)").into());
                children.push(
                    widget::dropdown(
                        &picture_inspector::HALFTONE_LABELS,
                        picture_inspector::HALFTONES
                            .iter()
                            .position(|mode| *mode == image.adjust.halftone),
                        |index| {
                            Message::InspectorChoice(Choice::ImageHalftone(
                                picture_inspector::HALFTONES[index],
                            ))
                        },
                    )
                    .width(Length::Fill)
                    .into(),
                );
                children.push(rail_button(
                    "Revert",
                    image
                        .changed
                        .then_some(Message::InspectorChoice(Choice::ImageRevert)),
                ));
                children.push(
                    widget::text::caption(if image.kept_by_ami_pro {
                        picture_inspector::IMAGE_NOTE_AMI_PRO
                    } else {
                        picture_inspector::IMAGE_NOTE_BAKED
                    })
                    .into(),
                );
            }
        }

        children.push(section("Size and position"));
        if shown.repeating {
            children
                .push(widget::text::caption("Changes apply on every page of the layout.").into());
        }
        if shown.page.is_some() {
            children.push(input("Page", Field::Page, true));
        }
        children.push(input(shown.x_label, Field::X, shown.x_editable));
        children.push(input(shown.y_label, Field::Y, shown.y_editable));
        children.push(input("Width (in)", Field::Width, true));
        children.push(input("Height (in)", Field::Height, true));

        children.push(section("Type"));
        children.push(widget::text::caption("Placement").into());
        children.push(
            widget::dropdown(
                &PlacementKind::LABELS,
                PlacementKind::ALL
                    .iter()
                    .position(|kind| *kind == shown.placement),
                |index| Message::InspectorChoice(Choice::Placement(PlacementKind::ALL[index])),
            )
            .width(Length::Fill)
            .into(),
        );
        // The wraps a placement allows: With para above frames do not wrap
        // around, and Flow with text frames always do.
        let first_wrap = WRAPS
            .iter()
            .position(|wrap| Some(wrap) == shown.wraps.first())
            .unwrap_or(0);
        let wrap_labels = &WRAP_LABELS[first_wrap..first_wrap + shown.wraps.len()];
        children.push(widget::text::caption("Text wrap").into());
        children.push(
            widget::dropdown(
                wrap_labels,
                shown.wraps.iter().position(|wrap| *wrap == shown.wrap),
                move |index| Message::InspectorChoice(Choice::Wrap(WRAPS[first_wrap + index])),
            )
            .width(Length::Fill)
            .into(),
        );
        children.push(
            widget::checkbox(shown.opaque)
                .label("Opaque")
                .on_toggle(|opaque| Message::InspectorChoice(Choice::Opaque(opaque)))
                .into(),
        );

        children.push(section("Lines and shadows"));
        for (index, label) in SIDES.into_iter().enumerate() {
            children.push(
                widget::checkbox(shown.sides[index])
                    .label(label)
                    .on_toggle(move |on| Message::InspectorChoice(Choice::Side(index, on)))
                    .into(),
            );
        }
        children.push(input("Line style (1–8)", Field::LineStyle, true));
        children.push(widget::text::caption("Line position").into());
        children.push(
            widget::dropdown(
                &LINE_POSITION_LABELS,
                LINE_POSITIONS
                    .iter()
                    .position(|position| *position == shown.line_position),
                |index| Message::InspectorChoice(Choice::LinePosition(LINE_POSITIONS[index])),
            )
            .width(Length::Fill)
            .into(),
        );
        // The four colours of Ami Pro's Modify Frame Layout, each a colour
        // button (`UI_SPEC.md` §23).
        let style = self.document.frame(frame)?.style.clone();
        let colours = self.colour_context();
        children.push(widget::text::caption("Line colour").into());
        children.push(colours.control(colour_control::Target::FrameLine, style.border.colour));
        children.push(widget::text::caption("Shadow").into());
        let shadow_index = match shown.shadow_depth {
            None => 0,
            Some(frame_inspector::NORMAL_SHADOW) => 1,
            Some(frame_inspector::DEEP_SHADOW) => 2,
            Some(_) => 3,
        };
        let current_depth = shown.shadow_depth;
        children.push(
            widget::dropdown(&SHADOW_LABELS, Some(shadow_index), move |index| {
                Message::InspectorChoice(Choice::ShadowDepth(match index {
                    0 => None,
                    1 => Some(frame_inspector::NORMAL_SHADOW),
                    2 => Some(frame_inspector::DEEP_SHADOW),
                    _ => current_depth.or(Some(frame_inspector::NORMAL_SHADOW)),
                }))
            })
            .width(Length::Fill)
            .into(),
        );
        if shown.shadow_depth.is_some() {
            if shadow_index == 3 {
                children.push(input("Shadow depth (in)", Field::ShadowDepth, true));
            }
            children.push(widget::text::caption("Shadow falls").into());
            children.push(
                widget::dropdown(
                    &ShadowSide::LABELS,
                    ShadowSide::ALL
                        .iter()
                        .position(|side| *side == shown.shadow_side),
                    |index| Message::InspectorChoice(Choice::ShadowSide(ShadowSide::ALL[index])),
                )
                .width(Length::Fill)
                .into(),
            );
            children.push(widget::text::caption("Shadow colour").into());
            children
                .push(colours.control(colour_control::Target::FrameShadow, style.shadow.colour));
        }
        children.push(widget::text::caption("Background").into());
        children.push(colours.control(colour_control::Target::FrameBackground, style.background));
        children.push(input("Rounded corners (%)", Field::Corners, true));

        children.push(section("Columns and margins"));
        children.push(input("Columns (1–8)", Field::Columns, true));
        if shown.uneven {
            children.push(even_columns_row(Message::InspectorChoice(
                Choice::EvenColumns,
            )));
        }
        // One column has no gutter, as in Page Layout Properties.
        if shown.value(Field::Columns) != "1" {
            children.push(input("Gutter (in)", Field::Gutter, true));
        }
        children.push(
            widget::checkbox(shown.balance)
                .label("Balance columns")
                .on_toggle(|on| Message::InspectorChoice(Choice::Balance(on)))
                .into(),
        );
        children.push(
            widget::checkbox(shown.line_between)
                .label("Line between columns")
                .on_toggle(|on| Message::InspectorChoice(Choice::LineBetween(on)))
                .into(),
        );
        if let Some(line) = style.interior.line_between {
            children.push(widget::text::caption("Line between columns colour").into());
            children.push(colours.control(colour_control::Target::FrameColumnLine, line.colour));
        }
        children.push(input("Top margin (in)", Field::MarginTop, true));
        children.push(input("Right margin (in)", Field::MarginRight, true));
        children.push(input("Bottom margin (in)", Field::MarginBottom, true));
        children.push(input("Left margin (in)", Field::MarginLeft, true));
        children.push(
            widget::text::caption(format!(
                "{} tab stops (editing tabs comes later)",
                shown.tabs
            ))
            .into(),
        );

        children.push(section("Name"));
        children.push(input("Frame name", Field::Name, true));
        if let Some(name) = &shown.macro_name {
            children.push(
                widget::text::caption(format!("Runs macro {name} (macros are not supported)"))
                    .into(),
            );
        }
        // Ami Pro's Make Default (`12b0:0465`; decision F2).
        children.push(section("New frames"));
        let built_in = self.preferences.frame_defaults
            == amipro_editing::frame_creation::FrameDefaults::AMI_PRO;
        children.push(
            widget::text::caption(if built_in {
                "New frames take Ami Pro's default: lines, a shadow, rounded corners, and 0.10 in \
                 margins."
            } else {
                "New frames take the default made from a frame."
            })
            .into(),
        );
        // Ami Pro's label; the heading and the caption say what the
        // buttons are for (the rail is too narrow for longer ones).
        children.push(rail_button(
            "Make Default",
            Some(Message::NewFrameDefault(NewFrameDefault::Make)),
        ));
        children.push(rail_button(
            "Restore Default",
            (!built_in).then_some(Message::NewFrameDefault(NewFrameDefault::Restore)),
        ));
        children.push(
            widget::text::caption(
                "The wrap, display, lines, colours, shadow, corners, and margins; never the \
                 placement, size, columns, or tabs.",
            )
            .into(),
        );
        Some(rail(children))
    }

    /// Make Default for New Frames and Restore Ami Pro's Default (Frame
    /// Properties; Ami Pro's Make Default, `12b0:0465`; decision F2): the
    /// default every later frame takes, kept in the settings.
    fn set_new_frame_default(&mut self, action: NewFrameDefault) {
        use amipro_editing::frame_creation::FrameDefaults;
        let defaults = match action {
            NewFrameDefault::Make => {
                let Some(defaults) = self
                    .inspected_frame()
                    .and_then(|(frame, _)| self.document.frame(frame))
                    .map(|frame| FrameDefaults::from_style(&frame.style))
                else {
                    return;
                };
                self.sessions[self.active_session].status =
                    "New frames take this frame's settings".into();
                defaults
            }
            NewFrameDefault::Restore => {
                self.sessions[self.active_session].status =
                    "New frames take Ami Pro's default".into();
                FrameDefaults::AMI_PRO
            }
        };
        self.preferences.frame_defaults = defaults;
        self.save_preferences();
    }

    /// The ruler and the scrolled pages.
    fn page_workspace(&self) -> Element<'_, Message> {
        let session = &self.sessions[self.active_session];
        // Outline mode draws its display copy (the outline styles' fonts,
        // hidden paragraphs empty): the same text at the same positions.
        let document = session
            .outline_copy
            .as_ref()
            .filter(|_| session.outline.is_some())
            .unwrap_or(&session.document);
        let layout = session.pages.layout();
        let selection = session.page_selection;
        let frames = session.frame_selection.as_ref();
        let zoom = session.zoom;
        let session_id = session.id;
        let scroll_x = session.scroll_offset.x;
        let caret_page = page_surface::caret_page(document, layout, selection);
        // The ruler at the caret; with frames selected it shows the page
        // alone.
        let ruler_model = frames
            .is_none()
            .then(|| ruler::Ruler::at(document, layout, selection))
            .flatten();
        let ruler_page = ruler_model
            .as_ref()
            .map_or(caret_page.saturating_sub(1) as usize, |ruler| ruler.page);
        // Outline mode shows no pages, so there are no margins to measure.
        let show_ruler = ruler_shown(self.preferences.ruler_visible, session);
        let ruler_height = if show_ruler { ruler_view::HEIGHT } else { 1.0 };
        let guide = session.ruler_guide;
        let drawing_frame = session.drawing_frame;
        let tab = session.ruler_tab;
        let ruler_cursor = session.ruler_cursor;
        let ruler_menu = self.ruler_context_menu(ruler_model.as_ref());
        let context = self.page_context_menu();
        let key_binds = menu_key_binds();
        let draw_session = session.draw.is_some().then_some(session);
        // The Equation editor's card sits under the frame it edits (DO.9).
        let equation_card = session.equation_panel.as_ref().and_then(|panel| {
            layout.pages.iter().enumerate().find_map(|(index, page)| {
                page.frames
                    .iter()
                    .find(|frame| frame.frame == panel.frame)
                    .map(|frame| (panel, index, frame.bounds))
            })
        });
        let facing = session.facing;
        cosmic::iced::widget::responsive(move |available| {
            // Leave room for the vertical scroll bar so that Fit width does
            // not need a horizontal one.
            let view = PageView::arranged(
                layout,
                zoom,
                cosmic::iced::Size::new(
                    (available.width - PAGE_SCROLLBAR).max(1.0),
                    (available.height - ruler_height).max(1.0),
                ),
                facing,
            );
            let ruler = ruler_view::ruler_view(
                view.clone(),
                ruler_page,
                scroll_x,
                ruler_model.clone(),
                tab,
                Message::Ruler,
            )
            .keyboard(ruler_cursor);
            let card_at =
                equation_card.map(|(panel, page, bounds)| (panel, view.rect(page, bounds)));
            let draw_view = draw_session.and_then(|session| session.draw_view(view.scale));
            let surface = page_surface::page_surface(
                document,
                layout,
                view,
                session_id,
                selection,
                Message::Page,
                Message::Command,
            )
            .keyboard(self.page_keyboard)
            // A dialog over the window takes the keys (CA.14's live check:
            // typing in the password box reached the page behind it).
            .modal(cosmic::Application::dialog(self).is_some())
            .arrows_elsewhere(
                self.rail_open() && session.outline_navigator.as_ref().is_some_and(|n| n.keys),
            )
            .outline(
                session
                    .outline
                    .is_some()
                    .then_some(session.outline_icons.as_slice()),
            )
            .draw_mode(draw_view, Message::Draw)
            .frames(frames, Message::Frame)
            .guide(guide)
            .drawing_frame(drawing_frame)
            .fast_format(session.fast_format.is_some())
            .view_marks(page_surface::ViewMarks {
                tabs_and_returns: self.preferences.tabs_returns,
                column_guides: self.preferences.column_guides,
                margins_in_colour: self.preferences.margins_in_colour,
                table_addresses: self.preferences.table_addresses,
            })
            .pictures(&session.pictures, session.location.folder())
            .pictures_shown(self.preferences.show_pictures)
            .highlights(session.find.as_ref().map(|bar| &bar.highlights))
            .underlines(
                (self.preferences.spell_as_you_type && !session.spell_marks.is_empty())
                    .then_some(&session.spell_marks),
                (self.preferences.grammar_as_you_type && !session.grammar_marks.is_empty())
                    .then_some(&session.grammar_marks),
            )
            .marks(self.preferences.show_notes, self.preferences.show_bookmarks)
            .shading(
                (self.preferences.field_shading && !session.codes_shown())
                    .then_some(&session.field_shading),
            )
            .printing(
                session
                    .print_panel
                    .as_ref()
                    .map(|_| session.print_options.mode(true)),
            )
            .picture_mode(
                session.picture_mode.is_some()
                    && frames.and_then(|selection| selection.single(document))
                        == session.picture_mode,
            );
            // A picture file dropped on the page goes in at the drop point
            // (P7): the surface sees the drag as the pointer and moves the
            // caret there on the drop; the file follows in `Dropped`.
            // Always a copy: libcosmic would ask the file manager to move
            // the file.
            // The first of these the drag offers is taken: an image a
            // browser drags comes as the image, a file as its path.
            let surface = widget::dnd_destination(surface, picture_insert::drop_types())
                .action(cosmic::iced::clipboard::dnd::DndAction::Copy)
                .preferred_action(cosmic::iced::clipboard::dnd::DndAction::Copy)
                .forward_drag_as_cursor(true)
                .on_finish(move |mime, data, _action, _x, _y| Message::Dropped {
                    session_id,
                    data,
                    mime,
                });
            let surface: Element<'_, Message> = match card_at {
                Some((panel, rect)) => widget::popover(surface)
                    .popup(equation_panel::card(panel, rect.width.max(320.0)))
                    .position(widget::popover::Position::Point(cosmic::iced::Point::new(
                        rect.x,
                        rect.y + rect.height + 6.0,
                    )))
                    .into(),
                None => surface.into(),
            };
            let pages = widget::scrollable(surface)
                .id(workspace_scroll_id())
                .on_scroll(Message::Scrolled)
                .direction(Direction::Both {
                    vertical: Scrollbar::default(),
                    horizontal: Scrollbar::default(),
                })
                .width(Length::Fill)
                .height(Length::Fill);
            // The right-click menu (`UI_SPEC.md` §16) opens on the release;
            // the press reaches the surface first and selects what is under
            // the pointer, so the menu is built for it. It wraps the
            // scrollable, not the pages: inside, it would get the pointer
            // in scrolled coordinates, and libcosmic's in-window menu then
            // neither lays out at the pointer nor takes clicks once the
            // pages are scrolled (user, 2026-10-13).
            let pages = widget::context_menu(pages, Some(menu_trees(context.clone(), &key_binds)));
            // The pages lie on a recessed canvas, darker than the chrome in
            // both themes (`UI_SPEC.md` §4; gap W1).
            let canvas = widget::container(pages)
                .class(cosmic::theme::Container::custom(|theme| {
                    cosmic::iced::widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(workspace_canvas(theme))),
                        ..Default::default()
                    }
                }))
                .width(Length::Fill)
                .height(Length::Fill);
            // Hidden, the ruler leaves a pixel-high slot: the column keeps
            // its children, so the pages keep their scroll (a zero-size
            // child is dropped, `CLAUDE.md`).
            let slot: Element<'_, Message> = if show_ruler {
                // Its right-click menus wrap it, outside any scrollable.
                widget::context_menu(ruler, Some(menu_trees(ruler_menu.clone(), &key_binds))).into()
            } else {
                widget::space().height(Length::Fixed(1.0)).into()
            };
            widget::column::with_children(vec![slot, canvas.into()]).into()
        })
        .into()
    }

    /// The page's right-click menu (`UI_SPEC.md` §16): the frame menu while
    /// frames are selected, else the text menu, naming the caret
    /// paragraph's style, with the styles to apply.
    fn page_context_menu(&self) -> Vec<MenuEntry> {
        if self.drawing_open() {
            return menu_entries(self, command::DRAW_CONTEXT_MENU);
        }
        if self.frame_selection.is_some() {
            return menu_entries(self, command::FRAME_CONTEXT_MENU);
        }
        // In a margin Ami Pro's right button opens Modify Page Layout
        // (chapter 8), so Page Layout… comes first there.
        if self.context_in_margin {
            let mut entries = menu_entries(self, command::MARGIN_CONTEXT_MENU);
            entries.extend(self.text_context_menu());
            return entries;
        }
        self.text_context_menu()
    }

    /// The right-click menu on text: Modify Style naming the caret
    /// paragraph's style, the clipboard and character commands, and the
    /// styles to apply.
    fn text_context_menu(&self) -> Vec<MenuEntry> {
        let mut entries = menu_entries(self, command::TEXT_CONTEXT_MENU);
        // On an underlined word, its suggestions first (`UI_SPEC.md` §35).
        let mut proofing = self.proofing_context_entries();
        let proofing_len = proofing.len();
        // On an underlined word with nothing selected, only the spelling
        // entries (user, 2026-11-03): libcosmic opens a menu taller than the
        // room below the pointer upwards, and in a window not maximised its
        // top, the suggestions, was above the window.
        let (start, end) = self.page_selection.ordered();
        if start == end
            && self.table_at().is_none()
            && self.context_word.as_ref().is_some_and(|word| word.flagged)
        {
            while matches!(proofing.last(), Some(MenuEntry::Divider)) {
                proofing.pop();
            }
            return proofing;
        }
        let styles = self.document.styles();
        let current = self.caret_paragraph().map(|paragraph| paragraph.style);
        if let Some(style) = current.and_then(|style| styles.get(&style))
            && let Some(MenuEntry::Item { label, .. }) = entries.first_mut()
        {
            *label = format!("Modify Style “{}”…", style.name);
        }
        // On a field, its commands after Modify Style (`UI_SPEC.md` §28).
        if self.field_at_caret().is_some() {
            let field = menu_entries(self, command::FIELD_CONTEXT_MENU);
            let at = 2.min(entries.len());
            entries.splice(at..at, field);
        }
        // In a table, its row, column, and cell commands first (`UI_SPEC.md`
        // §32).
        if self.table_at().is_some() {
            let table = menu_entries(self, command::TABLE_CONTEXT_MENU);
            entries.splice(0..0, table);
        }
        if proofing_len > 0 {
            entries.splice(0..0, proofing);
        }
        entries.push(MenuEntry::Divider);
        entries.push(MenuEntry::Folder {
            label: "Paragraph Style".into(),
            indented: false,
            entries: style_dialogs::listed_styles(styles)
                .into_iter()
                .map(|(style, label)| MenuEntry::Item {
                    label,
                    command: MenuCommand::ApplyStyle(style),
                    enabled: true,
                    checked: Some(current == Some(style)),
                })
                .collect(),
        });
        entries
    }

    /// Zoom out, a slider, the percentage, and zoom in (`UI_SPEC.md`,
    /// "Status bar"), all on the page transform.
    /// The status bar's Face and Point Size buttons (Ami Pro's; CM12): the
    /// selection's or the caret's font; a choice applies to the selection,
    /// or to the text typed next at a caret, as one undo step.
    fn font_controls(&self) -> Vec<Element<'_, Message>> {
        let session = &self.sessions[self.active_session];
        let (family, size, _) = session.selection_font();
        let face = self
            .font_families
            .iter()
            .position(|known| known.eq_ignore_ascii_case(&family));
        let size = character_dialogs::SIZES
            .iter()
            .position(|known| known.parse::<f32>().ok() == Some(size));
        vec![
            widget::dropdown(self.font_families.as_slice(), face, Message::StatusFace)
                .width(150)
                .into(),
            widget::dropdown(
                character_dialogs::SIZES.as_slice(),
                size,
                Message::StatusSize,
            )
            .width(64)
            .into(),
        ]
    }

    /// The Face list holds the caret's family, installed or not.
    fn sync_font_families(&mut self) {
        if !self.page_surface {
            return;
        }
        if self.font_families.is_empty() {
            self.font_families = page_layout::installed_families();
        }
        let (family, _, _) = self.sessions[self.active_session].selection_font();
        if !self
            .font_families
            .iter()
            .any(|known| known.eq_ignore_ascii_case(&family))
        {
            let at = self
                .font_families
                .partition_point(|known| known.to_lowercase() < family.to_lowercase());
            self.font_families.insert(at, family);
        }
    }

    fn zoom_controls(&self) -> Vec<Element<'_, Message>> {
        let percent = self.sessions[self.active_session].page_view().percent();
        let button = |command: CommandId, icon: &'static str| -> Element<'_, Message> {
            // Small enough for the 28-pixel status bar.
            widget::button::icon(widget::icon::from_name(icon))
                .extra_small()
                .padding(2)
                .name(command::metadata(command).label)
                .tooltip(command_tooltip(command))
                .on_press(Message::Command(command))
                .into()
        };
        vec![
            button(CommandId::ZoomOut, "zoom-out-symbolic"),
            widget::slider(25..=400, percent, Message::ZoomTo)
                .width(120)
                .into(),
            widget::text::caption(format!("{percent}%")).into(),
            button(CommandId::ZoomIn, "zoom-in-symbolic"),
        ]
    }
}

/// The find bar (`UI_SPEC.md` §27; decisions Find F1–F4).
#[cfg(feature = "page-surface")]
impl AmiProApp {
    /// Edit › Find… and Replace…: opens the bar at the caret, a short
    /// selection filling Find, and gives a field the keyboard with its text
    /// selected.
    fn open_find(&mut self, replacing: bool) -> Task<cosmic::Action<Message>> {
        let recent = self.preferences.recent_searches.first().cloned();
        let last = self.last_find.clone();
        let session = &mut self.sessions[self.active_session];
        let previous = session.find.take().or(last);
        let mut bar = find_bar::FindBar::open(
            &session.document,
            session.page_selection,
            replacing,
            previous.as_ref(),
        );
        if bar.find.is_empty()
            && let Some(recent) = recent
        {
            bar.find = recent;
        }
        let field = if replacing && !bar.find.is_empty() {
            find_bar::Field::Replace
        } else {
            find_bar::Field::Find
        };
        session.find = Some(bar);
        self.sync_find_bar();
        self.focus_bar_field(field, true)
    }

    /// Closes the bar, keeping its texts for the next (the last match stays
    /// selected).
    fn close_find(&mut self) {
        if let Some(bar) = self.sessions[self.active_session].find.take() {
            self.last_find = Some(bar);
        }
    }

    /// Gives a field of the bar the keyboard, and records it there: a field
    /// focused by a task hears nothing of it.
    fn focus_bar_field(
        &mut self,
        field: find_bar::Field,
        select: bool,
    ) -> Task<cosmic::Action<Message>> {
        if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
            bar.focused = Some(field);
            bar.last_field = field;
        }
        focus_find_field(field, select)
    }

    /// Finds the matches again when the text, the options, or the document
    /// changed; a current match no longer selected stops being current.
    fn sync_find_bar(&mut self) {
        let options = self.preferences.find_options.clone();
        let session = &mut self.sessions[self.active_session];
        let selection = session.page_selection;
        let Some(bar) = session.find.as_mut() else {
            return;
        };
        bar.refresh(&session.document, session.rich_layout_generation, &options);
        // The current match is the one selected, however it was.
        bar.current = bar.selected_index(selection);
        self.count_set_matches();
    }

    /// Find Next (`forward`) or Previous: selects the match and scrolls it
    /// into view on its page; without the bar, opens it.
    fn find_step(&mut self, forward: bool) -> Task<cosmic::Action<Message>> {
        if self.find.is_none() {
            return self.open_find(false);
        }
        self.sync_find_bar();
        let options = self.preferences.find_options.clone();
        let session = &mut self.sessions[self.active_session];
        let Some(bar) = session.find.as_ref() else {
            return Task::none();
        };
        if bar.find.is_empty() {
            return self.focus_bar_field(find_bar::Field::Find, true);
        }
        let text = bar.find.clone();
        // In All Files of the Master (MS11): past this file's matches the
        // search goes on into the next file.
        let all_files = bar.all_files && !bar.files.is_empty();
        let index = match (all_files, forward) {
            (true, true) => bar.next_in_file(session.page_selection, &options),
            (true, false) => bar.previous_in_file(session.page_selection),
            (false, true) => bar.next(session.page_selection, &options),
            (false, false) => bar.previous(session.page_selection),
        };
        find_bar::remember(&mut self.preferences.recent_searches, &text);
        match index {
            Some(index) => self.select_match(index),
            None if all_files => self.find_in_next_file(forward),
            None => {
                self.status = format!("“{text}” is not found");
                Task::none()
            }
        }
    }

    /// Selects match `index` on the page that shows it.
    fn select_match(&mut self, index: usize) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some(bar) = session.find.as_mut() else {
            return Task::none();
        };
        let Some(found) = bar.matches.get(index).cloned() else {
            return Task::none();
        };
        bar.current = Some(index);
        let count = bar.matches.len();
        let in_set = bar.all_files && !bar.files.is_empty();
        let page =
            find_bar::match_page(session.pages.layout(), &found, session.page_selection.page);
        session.frame_selection = None;
        session.page_selection = PageSelection {
            anchor: found.start,
            focus: found.end,
            page,
        };
        session.sync_editor_to_page();
        session.status = if in_set {
            let name = session
                .file_path()
                .and_then(std::path::Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("{} of {count} in {name}", index + 1)
        } else {
            format!("{} of {count}", index + 1)
        };
        session.scroll_to_caret()
    }

    /// Replace: the current match, if it is selected, as one undo step,
    /// then the next match.
    fn find_replace_one(&mut self) -> Task<cosmic::Action<Message>> {
        self.sync_find_bar();
        let options = self.preferences.find_options.clone();
        let session = &mut self.sessions[self.active_session];
        let Some(bar) = session.find.as_ref() else {
            return Task::none();
        };
        if let Some(found) = bar.current_selected(session.page_selection).cloned() {
            let commands = amipro_editing::find::replace_commands(
                &session.document,
                &found,
                &bar.replace,
                &options,
            );
            if commands.is_empty() {
                session.status = format!("There is no style “{}”", bar.replace.trim());
                return Task::none();
            }
            session.execute_page_command(&amipro_editing::Command::Several(commands));
            self.sync_find_bar();
        }
        self.find_step(true)
    }

    /// Replace All: every match in the streams searched as one undo step,
    /// the count in the status bar (F4).
    fn find_replace_all(&mut self) {
        self.sync_find_bar();
        if self
            .find
            .as_ref()
            .is_some_and(|bar| bar.all_files && !bar.files.is_empty())
        {
            self.find_replace_all_files();
            return;
        }
        let options = self.preferences.find_options.clone();
        let session = &mut self.sessions[self.active_session];
        let Some(bar) = session.find.as_ref() else {
            return;
        };
        let (text, count) = (bar.find.clone(), bar.matches.len());
        if count == 0 {
            session.status = format!("“{text}” is not found");
            return;
        }
        match amipro_editing::find::replace_all(
            &session.document,
            &bar.matches,
            &bar.replace,
            &options,
        ) {
            Some(command) => {
                if session.execute_page_command(&command) {
                    session.status = if count == 1 {
                        "Replaced 1 match".into()
                    } else {
                        format!("Replaced {count} matches")
                    };
                }
            }
            None => session.status = format!("There is no style “{}”", bar.replace.trim()),
        }
        // The selection it was limited to is gone with what it held.
        if let Some(bar) = session.find.as_mut() {
            bar.within = None;
            bar.current = None;
        }
        find_bar::remember(&mut self.preferences.recent_searches, &text);
        self.sync_find_bar();
    }

    fn update_find(&mut self, action: find_bar::FindAction) -> Task<cosmic::Action<Message>> {
        use find_bar::{FindAction, FindMenu, Toggle};
        // Esc ends Show Clean Screen first (CM9).
        if self.clean_screen && matches!(action, FindAction::Escape) {
            return self.set_clean_screen(false);
        }
        if self.find.is_none() {
            return Task::none();
        }
        match action {
            FindAction::Text(field, text) => {
                let session = &mut self.sessions[self.active_session];
                if let Some(bar) = session.find.as_mut() {
                    *bar.text_mut(field) = text;
                    if field == find_bar::Field::Find {
                        bar.current = None;
                    }
                }
                self.sync_find_bar();
            }
            FindAction::Submit(find_bar::Field::Find) => return self.find_step(!self.shift),
            FindAction::Submit(find_bar::Field::Replace) => return self.find_replace_one(),
            FindAction::Next => return self.find_step(true),
            FindAction::Previous => return self.find_step(false),
            FindAction::ReplaceOne => return self.find_replace_one(),
            FindAction::ReplaceAll => self.find_replace_all(),
            FindAction::Focus(field, focused) => {
                if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
                    if focused {
                        bar.focused = Some(field);
                        bar.last_field = field;
                    } else if bar.focused == Some(field) {
                        bar.focused = None;
                    }
                }
            }
            FindAction::ShowReplace => {
                let bar = self.sessions[self.active_session].find.as_mut();
                if let Some(bar) = bar {
                    bar.replacing = !bar.replacing;
                    if bar.replacing {
                        return self.focus_bar_field(find_bar::Field::Replace, true);
                    }
                }
            }
            FindAction::Close => self.close_find(),
            FindAction::Escape => {
                if self.find.as_ref().is_some_and(|bar| bar.focused.is_some()) {
                    self.close_find();
                }
            }
            FindAction::Style(field, index) => {
                let names = find_style_names(&self.document);
                if let Some(name) = names.get(index).cloned()
                    && let Some(bar) = self.sessions[self.active_session].find.as_mut()
                {
                    *bar.text_mut(field) = name;
                    bar.current = None;
                }
                self.sync_find_bar();
            }
            FindAction::Menu(menu) => match menu {
                FindMenu::Toggle(Toggle::AllFiles) => {
                    let available = self.find_set_master().is_some();
                    if let Some(bar) = self.sessions[self.active_session].find.as_mut() {
                        bar.all_files = !bar.all_files && available;
                        bar.files.clear();
                        bar.files_key = None;
                        bar.current = None;
                    }
                    self.sync_find_bar();
                }
                FindMenu::File(index) => return self.find_go_to_file(index),
                FindMenu::Toggle(Toggle::InSelection) => {
                    let session = &mut self.sessions[self.active_session];
                    let selection = session.page_selection;
                    if let Some(bar) = session.find.as_mut() {
                        bar.within = match bar.within {
                            Some(_) => None,
                            None if !selection.is_caret() => Some(selection.ordered()),
                            None => None,
                        };
                        bar.current = None;
                    }
                    self.sync_find_bar();
                }
                FindMenu::Toggle(toggle) => {
                    find_bar::toggle(&mut self.preferences.find_options, toggle);
                    self.sync_find_bar();
                }
                FindMenu::FindAttribute(choice) => {
                    let options = &mut self.preferences.find_options;
                    options.find_attributes = choice.choose(options.find_attributes);
                    self.sync_find_bar();
                }
                FindMenu::ReplaceAttribute(choice) => {
                    let options = &mut self.preferences.find_options;
                    options.replace_attributes = choice.choose(options.replace_attributes);
                }
                FindMenu::InsertTab | FindMenu::InsertReturn => {
                    let mark = if menu == FindMenu::InsertTab {
                        amipro_editing::find::TAB_MARK
                    } else {
                        amipro_editing::find::RETURN_MARK
                    };
                    let session = &mut self.sessions[self.active_session];
                    if let Some(bar) = session.find.as_mut() {
                        let field = bar.last_field;
                        bar.text_mut(field).push(mark);
                        if field == find_bar::Field::Find {
                            bar.current = None;
                        }
                        self.sync_find_bar();
                        return self.focus_bar_field(field, false);
                    }
                }
                FindMenu::Recent(index) => {
                    let text = self.preferences.recent_searches.get(index).cloned();
                    let session = &mut self.sessions[self.active_session];
                    if let (Some(text), Some(bar)) = (text, session.find.as_mut()) {
                        bar.find = text;
                        bar.current = None;
                        self.sync_find_bar();
                        return self.focus_bar_field(find_bar::Field::Find, false);
                    }
                }
            },
        }
        Task::none()
    }

    /// The bar, a chrome row like the toolbar's between the tabs and the
    /// ruler: Find with Recent ▾, ▲ ▼ and the count, Options ▾ and ×; under
    /// it Replace with Replace and Replace All. Both rows share one grid, so
    /// the fields start and end together (user, 2026-10-20).
    fn find_bar_view(&self) -> Option<Element<'_, Message>> {
        use find_bar::{Field, FindAction};
        // The grid: the Replace row's toggle column, the labels, and the
        // controls after each field, as wide in both rows.
        const LABEL: f32 = 64.0;
        const COUNT: f32 = 88.0;
        const RECENT: f32 = 84.0;
        const OPTIONS: f32 = 92.0;
        const ICON: f32 = 32.0;
        const TRAILING: f32 = RECENT
            + 2.0 * ICON
            + COUNT
            + OPTIONS
            + ICON
            + 5.0 * chrome::ICON_GAP
            + 2.0 * (chrome::GROUP_GAP * 2.0 + chrome::DIVIDER);
        let bar = self.find.as_ref()?;
        let options = &self.preferences.find_options;
        let send = Message::Find;
        let names = options.styles.then(|| find_style_names(&self.document));
        let field = |which: Field, text: &str| -> Element<'_, Message> {
            if let Some(names) = &names {
                let selected = names
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(text.trim()));
                return widget::dropdown(names.clone(), selected, move |index| {
                    Message::Find(FindAction::Style(which, index))
                })
                .width(Length::Fill)
                .into();
            }
            let id = match which {
                Field::Find => find_bar::FIND_FIELD,
                Field::Replace => find_bar::REPLACE_FIELD,
            };
            widget::text_input(
                if which == Field::Find {
                    "Text to find"
                } else {
                    "Replace with"
                },
                text.to_owned(),
            )
            .id(widget::Id::new(id))
            .on_input(move |text| Message::Find(FindAction::Text(which, text)))
            .on_submit(move |_| Message::Find(FindAction::Submit(which)))
            .on_focus(Message::Find(FindAction::Focus(which, true)))
            .on_unfocus(Message::Find(FindAction::Focus(which, false)))
            .width(Length::Fill)
            .into()
        };
        // Icon buttons as the toolbar's: 32 pixels square.
        let icon_button = |icon: &'static str,
                           label: &'static str,
                           action: FindAction,
                           enabled: bool|
         -> Element<'_, Message> {
            widget::button::icon(widget::icon::from_name(icon))
                .name(label)
                .tooltip(label)
                .width(ICON)
                .height(ICON)
                .on_press_maybe(enabled.then(|| send(action)))
                .into()
        };
        let label = |text: &'static str| -> Element<'_, Message> {
            widget::text::body(text).width(Length::Fixed(LABEL)).into()
        };
        let divider = || -> Element<'_, Message> {
            widget::divider::vertical::default()
                .height(24)
                .width(chrome::DIVIDER)
                .into()
        };
        let has_matches =
            !bar.matches.is_empty() || bar.set_total().is_some_and(|(total, _)| total > 0);
        let key_binds = HashMap::new();
        let recent: Vec<MenuEntry> = if self.preferences.recent_searches.is_empty() {
            vec![MenuEntry::Item {
                label: "No recent searches".into(),
                command: MenuCommand::Find(find_bar::FindMenu::Recent(usize::MAX)),
                enabled: false,
                checked: None,
            }]
        } else {
            self.preferences
                .recent_searches
                .iter()
                .enumerate()
                .map(|(index, text)| MenuEntry::Item {
                    label: text.clone(),
                    command: MenuCommand::Find(find_bar::FindMenu::Recent(index)),
                    enabled: true,
                    checked: None,
                })
                .collect()
        };
        // Menus as the toolbar's More ▾: a one-root menu bar of a fixed
        // width.
        let one_menu = |root: &'static str,
                        entries: Vec<MenuEntry>,
                        width: f32,
                        items: u16|
         -> Element<'_, Message> {
            let tree = menu::Tree::with_children(
                RcElementWrapper::new(Element::from(menu::root(root))),
                menu_trees(entries, &key_binds),
            );
            widget::container(
                menu::bar(vec![tree])
                    .item_height(ItemHeight::Dynamic(36))
                    .item_width(ItemWidth::Uniform(items)),
            )
            .width(Length::Fixed(width))
            .into()
        };
        let toggle = icon_button(
            if bar.replacing {
                "pan-down-symbolic"
            } else {
                "pan-end-symbolic"
            },
            if bar.replacing {
                "Hide Replace"
            } else {
                "Show Replace (Ctrl+H)"
            },
            FindAction::ShowReplace,
            true,
        );
        let mut rows = vec![find_bar_row(
            toggle,
            label("Find"),
            field(Field::Find, &bar.find),
            TRAILING,
            vec![
                one_menu("Recent ▾", recent, RECENT, 280),
                divider(),
                find_bar_group(vec![
                    icon_button(
                        "go-up-symbolic",
                        "Find Previous (Shift+F3)",
                        FindAction::Previous,
                        has_matches,
                    ),
                    icon_button(
                        "go-down-symbolic",
                        "Find Next (F3)",
                        FindAction::Next,
                        has_matches,
                    ),
                    widget::text::body(bar.count_label())
                        .width(Length::Fixed(COUNT))
                        .align_x(cosmic::iced::alignment::Horizontal::Center)
                        .into(),
                ]),
                divider(),
                find_bar_group(vec![
                    one_menu("Options ▾", self.find_options_menu(bar), OPTIONS, 300),
                    icon_button(
                        "window-close-symbolic",
                        "Close (Esc)",
                        FindAction::Close,
                        true,
                    ),
                ]),
            ],
        )];
        if bar.replacing {
            rows.push(find_bar_row(
                widget::space::horizontal()
                    .width(Length::Fixed(ICON))
                    .into(),
                label("Replace"),
                field(Field::Replace, &bar.replace),
                TRAILING,
                vec![find_bar_group(vec![
                    widget::button::standard("Replace")
                        .on_press_maybe(has_matches.then(|| send(FindAction::ReplaceOne)))
                        .into(),
                    widget::button::standard("Replace All")
                        .on_press_maybe(has_matches.then(|| send(FindAction::ReplaceAll)))
                        .into(),
                ])],
            ));
        }
        Some(widget::column::with_children(rows).into())
    }

    /// Options ▾: Ami Pro's options with check marks, the attributes, and
    /// the codes for a tab and a return.
    fn find_options_menu(&self, bar: &find_bar::FindBar) -> Vec<MenuEntry> {
        use find_bar::{ATTRIBUTE_CHOICES, FindMenu, Toggle};
        let options = &self.preferences.find_options;
        let item =
            |label: &str, menu: FindMenu, enabled: bool, checked: Option<bool>| MenuEntry::Item {
                label: label.into(),
                command: MenuCommand::Find(menu),
                enabled,
                checked,
            };
        let switch = |label: &str, toggle: Toggle, enabled: bool| {
            item(
                label,
                FindMenu::Toggle(toggle),
                enabled,
                Some(find_bar::toggled(options, bar, toggle)),
            )
        };
        let text = !options.styles;
        let attributes = |replacing: bool| {
            ATTRIBUTE_CHOICES
                .iter()
                .map(|choice| {
                    let (menu, current) = if replacing {
                        (
                            FindMenu::ReplaceAttribute(*choice),
                            options.replace_attributes,
                        )
                    } else {
                        (FindMenu::FindAttribute(*choice), options.find_attributes)
                    };
                    item(
                        choice.label(replacing),
                        menu,
                        text,
                        Some(choice.checked(current)),
                    )
                })
                .collect()
        };
        let in_main = bar.origin == self.document.main_flow_id();
        vec![
            switch("Whole Word Only", Toggle::WholeWord, text),
            switch("Match Case", Toggle::MatchCase, text),
            switch("Keep the Found Text’s Capitals", Toggle::KeepCapitals, text),
            MenuEntry::Divider,
            switch("From the Beginning", Toggle::FromBeginning, in_main),
            switch(
                "Include Headers, Footers, and Frames",
                Toggle::OtherStreams,
                true,
            ),
            switch(
                "In the Selection",
                Toggle::InSelection,
                bar.within.is_some() || !self.page_selection.is_caret(),
            ),
            switch(
                "In All Files of the Master",
                Toggle::AllFiles,
                bar.all_files || self.find_set_master().is_some(),
            ),
            MenuEntry::Folder {
                label: "Files of the Master".into(),
                indented: true,
                entries: if bar.files.is_empty() {
                    vec![item(
                        "In All Files of the Master is off",
                        FindMenu::File(usize::MAX),
                        false,
                        None,
                    )]
                } else {
                    bar.files
                        .iter()
                        .enumerate()
                        .map(|(index, file)| {
                            let label = match file.count {
                                1 => format!("{}: 1 match", file.name),
                                count => format!("{}: {count} matches", file.name),
                            };
                            item(&label, FindMenu::File(index), file.count > 0, None)
                        })
                        .collect()
                },
            },
            MenuEntry::Divider,
            switch("Paragraph Styles", Toggle::Styles, true),
            MenuEntry::Folder {
                label: "Find Attributes".into(),
                indented: true,
                entries: attributes(false),
            },
            MenuEntry::Folder {
                label: "Replace Attributes".into(),
                indented: true,
                entries: attributes(true),
            },
            MenuEntry::Divider,
            item("Insert Tab (»)", FindMenu::InsertTab, text, Some(false)),
            item(
                "Insert Return (¶)",
                FindMenu::InsertReturn,
                text,
                Some(false),
            ),
        ]
    }
}

/// One of the find bar's rows, laid out as the toolbar is: what leads it,
/// the label, the field, and its controls in a column of `trailing` pixels,
/// as wide in both rows so that the fields line up.
#[cfg(feature = "page-surface")]
fn find_bar_row<'a>(
    leading: Element<'a, Message>,
    label: Element<'a, Message>,
    field: Element<'a, Message>,
    trailing: f32,
    controls: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    widget::container(
        widget::row::with_children(vec![
            leading,
            label,
            field,
            widget::container(
                widget::row::with_children(controls)
                    .spacing(chrome::GROUP_GAP)
                    .align_y(cosmic::iced::Alignment::Center),
            )
            .width(Length::Fixed(trailing))
            .into(),
        ])
        .spacing(chrome::ICON_GAP)
        .align_y(cosmic::iced::Alignment::Center),
    )
    .padding([4.0, chrome::BAR_PADDING])
    .height(ACTION_BAR_HEIGHT)
    .width(Length::Fill)
    .into()
}

/// A group of the find bar's controls, spaced as the toolbar's icons.
#[cfg(feature = "page-surface")]
fn find_bar_group(items: Vec<Element<'_, Message>>) -> Element<'_, Message> {
    widget::row::with_children(items)
        .spacing(chrome::ICON_GAP)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// The paragraph styles a Paragraph Styles search lists, by name.
#[cfg(feature = "page-surface")]
fn find_style_names(document: &Document) -> Vec<String> {
    let mut names: Vec<String> = document
        .styles()
        .values()
        .map(|style| style.name.clone())
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// Gives a find bar field the keyboard, its text selected (`select`) or the
/// cursor at its end, then repaints (`CLAUDE.md`: an operation alone does
/// not redraw).
#[cfg(feature = "page-surface")]
fn focus_find_field(field: find_bar::Field, select: bool) -> Task<cosmic::Action<Message>> {
    let id = widget::Id::new(match field {
        find_bar::Field::Find => find_bar::FIND_FIELD,
        find_bar::Field::Replace => find_bar::REPLACE_FIELD,
    });
    let place = if select {
        widget::text_input::select_all::<Message>(id.clone())
    } else {
        widget::text_input::move_cursor_to_end::<Message>(id.clone())
    };
    widget::text_input::focus::<Message>(id)
        .chain(place)
        .chain(Task::done(Message::Repaint))
        .map(cosmic::Action::from)
}

/// The recessed canvas around the pages: the window background darkened,
/// so it keeps the theme's tint (`UI_SPEC.md` §4, "Alignment rules").
#[cfg(feature = "page-surface")]
fn workspace_canvas(theme: &cosmic::Theme) -> cosmic::iced::Color {
    let cosmic = theme.cosmic();
    let base: cosmic::iced::Color = cosmic.bg_color().into();
    let keep = if cosmic.is_dark { 0.6 } else { 0.8 };
    cosmic::iced::Color::from_rgb(base.r * keep, base.g * keep, base.b * keep)
}

#[cfg(feature = "page-surface")]
#[cfg(feature = "page-surface")]
const PAGE_SCROLLBAR: f32 = 16.0;

/// The context rail's surface: the theme's primary container, `width` wide
/// ([`RAIL_WIDTH`], or wider for Style Properties), with an edge on the
/// workspace side; over the workspace it also casts a shadow (`UI_SPEC.md`
/// §5; gaps R1, R6).
/// Where a page break switches to from the next page.
#[cfg(feature = "page-surface")]
#[derive(Clone, Copy, Debug)]
enum BreakSwitch {
    /// Nowhere: the same layout goes on.
    None,
    /// An existing layout (Ami Pro's Revert, for its own layout).
    To(amipro_document::LayoutId),
    /// A new copy of this layout (Insert Page Layout).
    CopyOf(amipro_document::LayoutId),
}

fn rail_surface(content: Element<'_, Message>, width: f32, overlay: bool) -> Element<'_, Message> {
    widget::container(content)
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            let primary = cosmic.primary_container_color();
            cosmic::iced::widget::container::Style {
                icon_color: Some(cosmic.on_bg_color().into()),
                text_color: Some(cosmic.on_bg_color().into()),
                background: Some(cosmic::iced::Background::Color(primary.into())),
                border: cosmic::iced::Border {
                    color: cosmic.bg_divider().into(),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                shadow: if overlay {
                    cosmic::iced::Shadow {
                        color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.25),
                        offset: cosmic::iced::Vector::new(2.0, 0.0),
                        blur_radius: 8.0,
                    }
                } else {
                    cosmic::iced::Shadow::default()
                },
                snap: false,
            }
        }))
        .width(width)
        .height(Length::Fill)
        .into()
}

/// The styles whose names contain `filter`, ignoring case, in style order.
fn filtered_styles<'a>(document: &'a Document, filter: &str) -> Vec<&'a ParagraphStyle> {
    let needle = filter.trim().to_lowercase();
    document
        .styles()
        .values()
        .filter(|style| needle.is_empty() || style.name.to_lowercase().contains(&needle))
        .collect()
}

/// "Unequal columns" and an Even Columns button, in Page Layout Properties
/// and Frame Properties while a box keeps uneven columns (`UI_SPEC.md`
/// §19).
#[cfg(feature = "page-surface")]
pub(crate) fn even_columns_row(message: Message) -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::text::caption("Unequal columns").into(),
        widget::space::horizontal().into(),
        widget::button::standard("Even Columns")
            .on_press(message)
            .into(),
    ])
    .spacing(8)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

/// A full-width button in the context rail whose label wraps onto another
/// line rather than being cut off: the rail is 216 px and the desktop's
/// interface font may be wider than the one the tests measure (the user saw
/// "Restore Ami Pro's Default" cut off, 2026-10-17); libcosmic's standard
/// button keeps one line at a fixed height.
#[cfg(feature = "page-surface")]
fn rail_button(label: &'static str, on_press: Option<Message>) -> Element<'static, Message> {
    widget::button::custom(
        widget::text::body(label)
            .wrapping(Wrapping::WordOrGlyph)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .width(Length::Fill),
    )
    .padding([6, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Button::Standard)
    .on_press_maybe(on_press)
    .into()
}

/// A rail panel's title with the button that closes the rail.
#[cfg(feature = "page-surface")]
fn rail_header(title: &'static str) -> Element<'static, Message> {
    widget::row::with_children(vec![
        widget::text::heading(title).into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name(format!("Close {title}"))
            .tooltip(command_tooltip(CommandId::StylesPanel))
            .on_press(Message::Command(CommandId::StylesPanel))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

/// A rail panel's scrolled body, its scroll bar beside the content rather
/// than over it (gap R7).
/// The desktop's folder dialog for Export Folder's From or To.
#[cfg(feature = "page-surface")]
fn choose_folder(
    title: &'static str,
    start: Option<PathBuf>,
    to: bool,
) -> Task<cosmic::Action<Message>> {
    chosen_folder_in(title, start, move |folder| {
        Message::ExportFolder(export_folder::FolderAction::Chosen { to, folder })
    })
}

/// The Styles panel's filter, which Select a Style (Ctrl+Y) focuses.
const STYLE_FILTER: &str = "styles-filter";

/// The desktop's folder chooser; `chosen` makes its answer a message.
pub(crate) fn chosen_folder(
    title: &'static str,
    chosen: impl FnOnce(Option<PathBuf>) -> Message + Send + 'static,
) -> Task<cosmic::Action<Message>> {
    chosen_folder_in(title, None, chosen)
}

/// [`chosen_folder`] starting in `start` (DF1).
pub(crate) fn chosen_folder_in(
    title: &'static str,
    start: Option<PathBuf>,
    chosen: impl FnOnce(Option<PathBuf>) -> Message + Send + 'static,
) -> Task<cosmic::Action<Message>> {
    cosmic::task::future(async move {
        let folder = file_dialogs::open_dialog()
            .title(title.to_owned())
            .directory(start)
            .open_folder()
            .await
            .ok()
            .and_then(|response| response.url().to_file_path().ok());
        chosen(folder)
    })
}

fn rail_scroller(content: Element<'_, Message>) -> Element<'_, Message> {
    widget::scrollable(content)
        .direction(Direction::Vertical(Scrollbar::default().spacing(4)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Whether the ruler shows: the preference, except in Outline mode, which
/// lays the text out without pages (RB2) and so has no margins to measure.
#[cfg(feature = "page-surface")]
fn ruler_shown(preference: bool, session: &DocumentSession) -> bool {
    preference && session.outline.is_none()
}

/// The shortcuts menus show beside their commands.
fn menu_key_binds() -> HashMap<KeyBind, MenuCommand> {
    COMMANDS
        .iter()
        .filter_map(|command| {
            command
                .shortcut
                .map(|shortcut| (shortcut.key_bind(), MenuCommand::Command(command.id)))
        })
        .collect()
}

fn application_menu(app: &AmiProApp) -> Element<'static, Message> {
    let key_binds = menu_key_binds();

    let roots = MENU_LAYOUTS
        .iter()
        .map(|(root, layout)| {
            menu::Tree::with_children(
                RcElementWrapper::new(Element::from(menu::root(root.label()))),
                menu_trees(menu_entries(app, layout), &key_binds),
            )
        })
        .collect();

    menu::bar(roots)
        .item_height(ItemHeight::Dynamic(36))
        .item_width(ItemWidth::Uniform(320))
        .spacing(2.0)
        .into()
}

/// A menu's rows. libcosmic draws its own submenu rows without the check
/// column, so a submenu's row is built here with the column's space when
/// its menu has one, keeping every label in line.
fn menu_trees(
    entries: Vec<MenuEntry>,
    key_binds: &HashMap<KeyBind, MenuCommand>,
) -> Vec<menu::Tree<Message>> {
    let mut trees = Vec::new();
    let mut items = Vec::new();
    for entry in entries {
        match entry {
            MenuEntry::Divider => items.push(menu::Item::Divider),
            MenuEntry::Item {
                label,
                command,
                enabled,
                checked,
            } => {
                let mut entry = menu::Entry::new(label, command).enabled(enabled);
                if let Some(checked) = checked {
                    entry = entry.checked(checked);
                }
                items.push(menu::Item::Entry(entry));
            }
            MenuEntry::Folder {
                label,
                indented,
                entries,
            } => {
                trees.extend(menu::items(key_binds, std::mem::take(&mut items)));
                let mut row: Vec<Element<'static, Message>> = Vec::new();
                if indented {
                    row.push(widget::space::horizontal().width(16).into());
                    row.push(
                        widget::space::horizontal()
                            .width(cosmic::theme::spacing().space_xxs)
                            .into(),
                    );
                }
                row.push(widget::text(label).into());
                row.push(widget::space::horizontal().into());
                row.push(
                    widget::icon::from_name("pan-end-symbolic")
                        .size(16)
                        .icon()
                        .into(),
                );
                trees.push(menu::Tree::with_children(
                    RcElementWrapper::new(Element::from(
                        menu::menu_button(row).class(cosmic::theme::Button::MenuFolder),
                    )),
                    menu_trees(entries, key_binds),
                ));
            }
        }
    }
    trees.extend(menu::items(key_binds, items));
    trees
}

/// The check mark a command's menu entry shows, for commands with an on or
/// off state: the panels, and (user, 2026-10-12: “if we are using check
/// marks [for the Styles panel] … we should use it on the other items”)
/// Bold, Italic, Underline, and the selected paragraphs' alignment, as Ami
/// Pro's Text menu checks them.
fn menu_check(app: &AmiProApp, command: CommandId) -> Option<bool> {
    let alignment = |alignment| Some(app.selected_alignment() == Some(alignment));
    let caps = |caps| Some(app.character_state(CharacterAttribute::Caps(caps)) == ToggleState::On);
    #[cfg(feature = "page-surface")]
    if let Some(checked) = app.draw_check(command) {
        return Some(checked);
    }
    match command {
        CommandId::StylesPanel => Some(app.rail_open()),
        CommandId::ShowToolbar => Some(app.preferences.toolbar_visible),
        CommandId::Ruler => Some(app.preferences.ruler_visible),
        CommandId::StatusBar => Some(app.preferences.status_bar_visible),
        CommandId::Bold => Some(app.character_state(CharacterAttribute::Bold) == ToggleState::On),
        CommandId::Italic => {
            Some(app.character_state(CharacterAttribute::Italic) == ToggleState::On)
        }
        CommandId::Underline => {
            Some(app.character_state(CharacterAttribute::Underline) == ToggleState::On)
        }
        CommandId::WordUnderline => {
            Some(app.character_state(CharacterAttribute::WordUnderline) == ToggleState::On)
        }
        #[cfg(feature = "page-surface")]
        CommandId::ProtectedText => {
            Some(app.character_state(CharacterAttribute::Protected) == ToggleState::On)
        }
        #[cfg(feature = "page-surface")]
        CommandId::NoHyphenation => {
            Some(app.character_state(CharacterAttribute::NoHyphenation) == ToggleState::On)
        }
        CommandId::UpperCase => caps(Caps::Upper),
        CommandId::LowerCase => caps(Caps::Lower),
        CommandId::InitialCaps => caps(Caps::Initial),
        CommandId::SmallCaps => caps(Caps::Small),
        CommandId::AlignLeft => alignment(Alignment::Left),
        CommandId::Center => alignment(Alignment::Centre),
        CommandId::AlignRight => alignment(Alignment::Right),
        CommandId::Justify => alignment(Alignment::Justify),
        CommandId::Bullets => {
            Some(app.selected_list() == Some(amipro_editing::lists::ListKind::Bullets))
        }
        CommandId::Numbering => {
            Some(app.selected_list() == Some(amipro_editing::lists::ListKind::Numbering))
        }
        CommandId::PageBreak => app.row_break_check(),
        #[cfg(feature = "page-surface")]
        CommandId::ShowPowerFields => Some(app.codes_shown()),
        #[cfg(feature = "page-surface")]
        CommandId::FieldShading => Some(app.preferences.field_shading),
        #[cfg(feature = "page-surface")]
        CommandId::FastFormat => Some(app.fast_format.is_some()),
        #[cfg(feature = "page-surface")]
        CommandId::FullPage => Some(app.zoom == Zoom::FullPage),
        #[cfg(feature = "page-surface")]
        CommandId::ActualSize => Some(app.zoom == Zoom::Percent(100)),
        #[cfg(feature = "page-surface")]
        CommandId::Enlarged => Some(app.zoom == Zoom::Percent(200)),
        #[cfg(feature = "page-surface")]
        CommandId::CustomZoom => Some(app.zoom == Zoom::percent(app.preferences.custom_zoom)),
        #[cfg(feature = "page-surface")]
        CommandId::FitPageWidth => Some(app.zoom == Zoom::FitWidth),
        #[cfg(feature = "page-surface")]
        CommandId::TwoPages => Some(app.facing),
        #[cfg(feature = "page-surface")]
        CommandId::DraftMode => Some(app.draft.is_some()),
        #[cfg(feature = "page-surface")]
        CommandId::OutlineMode => Some(app.outline.is_some()),
        #[cfg(feature = "page-surface")]
        CommandId::ShowAllLevels => {
            Some(app.outline.is_some() && app.outline_state.levels.is_none())
        }
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel1 => Some(app.outline.is_some() && app.outline_state.levels == Some(1)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel2 => Some(app.outline.is_some() && app.outline_state.levels == Some(2)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel3 => Some(app.outline.is_some() && app.outline_state.levels == Some(3)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel4 => Some(app.outline.is_some() && app.outline_state.levels == Some(4)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel5 => Some(app.outline.is_some() && app.outline_state.levels == Some(5)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel6 => Some(app.outline.is_some() && app.outline_state.levels == Some(6)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel7 => Some(app.outline.is_some() && app.outline_state.levels == Some(7)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel8 => Some(app.outline.is_some() && app.outline_state.levels == Some(8)),
        #[cfg(feature = "page-surface")]
        CommandId::ShowLevel9 => Some(app.outline.is_some() && app.outline_state.levels == Some(9)),

        #[cfg(feature = "page-surface")]
        CommandId::ShowTabsReturns => Some(app.preferences.tabs_returns),
        #[cfg(feature = "page-surface")]
        CommandId::ColumnGuides => Some(app.preferences.column_guides),
        #[cfg(feature = "page-surface")]
        CommandId::MarginsInColour => Some(app.preferences.margins_in_colour),
        #[cfg(feature = "page-surface")]
        CommandId::ShowPictures => Some(app.preferences.show_pictures),
        #[cfg(feature = "page-surface")]
        CommandId::TableAddresses => Some(app.preferences.table_addresses),
        #[cfg(feature = "page-surface")]
        CommandId::UseOutlineStyles => Some(app.preferences.outline_styles),
        #[cfg(feature = "page-surface")]
        CommandId::OutlineNavigator => Some(app.outline_navigator.is_some() && app.rail_open()),
        CommandId::FullScreen => Some(app.clean_screen),
        #[cfg(feature = "page-surface")]
        CommandId::ContinuousPages => Some(!app.facing),
        #[cfg(feature = "page-surface")]
        CommandId::ShowNotes => Some(app.preferences.show_notes),
        #[cfg(feature = "page-surface")]
        CommandId::SpellAsYouType => Some(app.preferences.spell_as_you_type),
        #[cfg(feature = "page-surface")]
        CommandId::ShowBookmarks => Some(app.preferences.show_bookmarks),
        #[cfg(feature = "page-surface")]
        CommandId::TableHeadings
        | CommandId::ProtectCells
        | CommandId::LeaderNone
        | CommandId::LeaderDots
        | CommandId::LeaderDashes
        | CommandId::LeaderLine => app.table_menu_check(command),
        #[cfg(feature = "page-surface")]
        CommandId::LockField => Some(matches!(
            app.field_at_caret(),
            Some(amipro_editing::fields::FieldAt::Power { span, .. }) if span.mark.locked()
        )),
        _ => None,
    }
}

/// One menu entry before it becomes a libcosmic item.
#[derive(Clone, Debug, PartialEq)]
enum MenuEntry {
    Item {
        label: String,
        command: MenuCommand,
        enabled: bool,
        /// `Some` draws the check column, ticked when true.
        checked: Option<bool>,
    },
    /// A submenu; `indented` gives its label the check column's space.
    Folder {
        label: String,
        indented: bool,
        entries: Vec<MenuEntry>,
    },
    Divider,
}

/// A menu's entries. A menu holding any checkable entry gives every entry
/// the check column, so that all labels line up (user, 2026-10-12: View ›
/// Styles Panel looked indented, the only entry with the column).
fn menu_entries(app: &AmiProApp, layout: &[MenuSlot]) -> Vec<MenuEntry> {
    let mut entries = Vec::new();
    for slot in layout {
        match slot {
            MenuSlot::Command(command) => {
                let metadata = command::metadata(*command);
                #[cfg(feature = "page-surface")]
                let label = app.table_menu_label(*command).unwrap_or(metadata.label);
                #[cfg(not(feature = "page-surface"))]
                let label = metadata.label;
                entries.push(MenuEntry::Item {
                    label: label.to_owned(),
                    command: MenuCommand::Command(*command),
                    enabled: app.command_enabled(*command),
                    checked: menu_check(app, *command),
                });
            }
            MenuSlot::Folder(label, children) => entries.push(MenuEntry::Folder {
                label: (*label).to_owned(),
                indented: false,
                entries: menu_entries(app, children),
            }),
            MenuSlot::Divider => entries.push(MenuEntry::Divider),
            MenuSlot::Recent => {
                let shown = app.preferences.recent_count;
                let recent = &app.preferences.recent_files;
                entries.extend(recent.iter().take(shown).enumerate().map(|(index, path)| {
                    MenuEntry::Item {
                        label: preferences_dialog::recent_label(index, path),
                        command: MenuCommand::OpenRecent(index),
                        enabled: true,
                        checked: None,
                    }
                }));
                if shown > 0 && !recent.is_empty() {
                    entries.push(MenuEntry::Divider);
                }
            }
            MenuSlot::Documents => {
                entries.extend(app.sessions.iter().enumerate().map(|(index, session)| {
                    MenuEntry::Item {
                        label: session.tab_label(),
                        command: MenuCommand::ActivateTab(index),
                        enabled: true,
                        checked: Some(index == app.active_session),
                    }
                }));
            }
        }
    }
    let aligned = entries.iter().any(|entry| {
        matches!(
            entry,
            MenuEntry::Item {
                checked: Some(_),
                ..
            }
        )
    });
    if aligned {
        for entry in &mut entries {
            match entry {
                MenuEntry::Item { checked, .. } => {
                    checked.get_or_insert(false);
                }
                MenuEntry::Folder { indented, .. } => *indented = true,
                MenuEntry::Divider => {}
            }
        }
    }
    entries
}

#[derive(Clone)]
struct ActionBarState {
    enabled_commands: HashMap<CommandId, bool>,
    style_names: Vec<String>,
    style_ids: Vec<StyleId>,
    selected_style_index: Option<usize>,
    bold: ToggleState,
    italic: ToggleState,
    underline: ToggleState,
    /// The selected paragraphs' alignment when they share one.
    alignment: Option<Alignment>,
    /// The list the selected paragraphs show when they share one.
    list: Option<amipro_editing::lists::ListKind>,
    /// Format › Fast Format is on: its toolbar toggle shows checked.
    fast_format: bool,
    /// View › Show ▸ Tabs & Returns is on (its toolbar toggle).
    tabs_returns: bool,
    styles_visible: bool,
    toolbar: Vec<ToolbarItem>,
}

impl ActionBarState {
    fn enabled(&self, command: CommandId) -> bool {
        self.enabled_commands
            .get(&command)
            .copied()
            .unwrap_or(false)
    }

    fn alignment_state(&self, alignment: Alignment) -> ToggleState {
        toggle(self.alignment == Some(alignment))
    }

    fn toggle_state(&self, command: CommandId) -> ToggleState {
        match command {
            CommandId::Bold => self.bold,
            CommandId::Italic => self.italic,
            CommandId::Underline => self.underline,
            CommandId::AlignLeft => self.alignment_state(Alignment::Left),
            CommandId::Center => self.alignment_state(Alignment::Centre),
            CommandId::AlignRight => self.alignment_state(Alignment::Right),
            CommandId::Justify => self.alignment_state(Alignment::Justify),
            CommandId::Bullets => {
                toggle(self.list == Some(amipro_editing::lists::ListKind::Bullets))
            }
            CommandId::Numbering => {
                toggle(self.list == Some(amipro_editing::lists::ListKind::Numbering))
            }
            CommandId::FastFormat => toggle(self.fast_format),
            CommandId::ShowTabsReturns => toggle(self.tabs_returns),
            _ => ToggleState::Off,
        }
    }
}

fn toggle(on: bool) -> ToggleState {
    if on {
        ToggleState::On
    } else {
        ToggleState::Off
    }
}

fn action_bar_view(state: ActionBarState, width: f32) -> Element<'static, Message> {
    let layout = action_bar_layout(&state.toolbar, width);
    let mut children = Vec::new();

    for group in &layout.visible_groups {
        if !children.is_empty() {
            children.push(
                widget::divider::vertical::default()
                    .height(24)
                    .width(chrome::DIVIDER)
                    .into(),
            );
        }
        let items: Vec<Element<'static, Message>> = group
            .iter()
            .filter_map(|item| match item {
                ToolbarItem::StyleField => {
                    let style_ids = state.style_ids.clone();
                    Some(
                        widget::dropdown(
                            state.style_names.clone(),
                            state.selected_style_index,
                            move |index| Message::ApplyStyle(style_ids[index]),
                        )
                        .placeholder("Mixed")
                        .width(layout.style_width)
                        .into(),
                    )
                }
                ToolbarItem::Command(command) => Some(action_icon_button(
                    *command,
                    state.enabled(*command),
                    state.toggle_state(*command),
                )),
                ToolbarItem::Separator => None,
            })
            .collect();
        children.push(
            widget::row::with_children(items)
                .spacing(chrome::ICON_GAP)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
        );
    }

    if !layout.overflow_commands.is_empty() {
        children.push(
            widget::divider::vertical::default()
                .height(24)
                .width(chrome::DIVIDER)
                .into(),
        );
        children.push(
            widget::container(action_more_menu(&state, &layout.overflow_commands))
                .width(chrome::MORE_WIDTH)
                .into(),
        );
    }

    widget::container(
        widget::row::with_children(children)
            .spacing(chrome::GROUP_GAP)
            .align_y(cosmic::iced::Alignment::Center),
    )
    .padding([4.0, chrome::BAR_PADDING])
    .height(ACTION_BAR_HEIGHT)
    .width(Length::Fill)
    .into()
}

/// Tools › Customize Toolbar… (Ami Pro's SmartIcons dialog for one
/// toolbar, `UI_SPEC.md` §12): what can be added beside the toolbar's
/// items, with Add, Remove, Move Up and Down, Add Separator, and Reset to
/// Standard. Each change shows in the bar at once and is saved.
fn toolbar_editor_view(
    toolbar: &[ToolbarItem],
    editor: ToolbarEditor,
) -> Element<'static, Message> {
    const LIST_HEIGHT: f32 = 300.0;
    let row = |item: ToolbarItem, selected: bool, message: Option<Message>| {
        let icon: Element<'static, Message> = match item {
            ToolbarItem::Command(command) => widget::icon::icon(command_icon(
                action_icon_name(command).unwrap_or("image-missing"),
            ))
            .size(16)
            .into(),
            ToolbarItem::StyleField => widget::text::body("¶").width(16).into(),
            ToolbarItem::Separator => widget::text::body("|").width(16).into(),
        };
        let label = match item {
            ToolbarItem::Command(command) => command_tooltip(command),
            _ => chrome::toolbar_item_label(item).to_owned(),
        };
        widget::button::custom(
            widget::row::with_children(vec![
                icon,
                widget::text::body(label)
                    .wrapping(Wrapping::None)
                    .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                    .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center),
        )
        .class(cosmic::theme::Button::ListItem([6.0; 4]))
        .selected(selected)
        .padding([4, 8])
        .width(Length::Fill)
        .on_press_maybe(message)
        .into()
    };
    let list = |title: &'static str, rows: Vec<Element<'static, Message>>| {
        widget::column::with_children(vec![
            widget::text::heading(title).into(),
            widget::container(
                widget::scrollable(widget::column::with_children(rows).spacing(2))
                    .height(LIST_HEIGHT),
            )
            .class(cosmic::theme::Container::Card)
            .padding(4)
            .into(),
        ])
        .spacing(6)
        .width(Length::Fill)
        .into()
    };
    // What is already on the bar is not offered again.
    let choices: Vec<Element<'static, Message>> = chrome::toolbar_choices()
        .into_iter()
        .filter(|choice| ToolbarEditor::can_add(toolbar, *choice))
        .map(|choice| {
            row(
                choice,
                editor.choice == Some(choice),
                Some(Message::Toolbar(ToolbarEdit::SelectChoice(choice))),
            )
        })
        .collect();
    let items: Vec<Element<'static, Message>> = toolbar
        .iter()
        .enumerate()
        .map(|(index, item)| {
            row(
                *item,
                editor.item == Some(index),
                Some(Message::Toolbar(ToolbarEdit::SelectItem(index))),
            )
        })
        .collect();
    let selected = editor.item.filter(|index| *index < toolbar.len());
    let can_add = editor
        .choice
        .is_some_and(|choice| ToolbarEditor::can_add(toolbar, choice));
    // One width for the column (a standard text button shrinks to its label).
    let button =
        |label: &'static str, edit: ToolbarEdit, enabled: bool| -> Element<'static, Message> {
            widget::button::custom(
                widget::text::body(label)
                    .width(Length::Fill)
                    .align_x(cosmic::iced::Alignment::Center),
            )
            .class(cosmic::theme::Button::Standard)
            .padding([6, 12])
            .width(Length::Fill)
            .on_press_maybe(enabled.then_some(Message::Toolbar(edit)))
            .into()
        };
    let buttons = widget::column::with_children(vec![
        widget::space::vertical().height(28).into(),
        button("Add", ToolbarEdit::Add, can_add),
        button("Remove", ToolbarEdit::Remove, selected.is_some()),
        button(
            "Move Up",
            ToolbarEdit::MoveUp,
            selected.is_some_and(|index| index > 0),
        ),
        button(
            "Move Down",
            ToolbarEdit::MoveDown,
            selected.is_some_and(|index| index + 1 < toolbar.len()),
        ),
        button("Add Separator", ToolbarEdit::AddSeparator, true),
    ])
    .spacing(8)
    .width(140);
    widget::dialog()
        .title("Customize Toolbar")
        .body("Changes show in the toolbar at once and are kept for your next session.")
        .control(
            widget::row::with_children(vec![
                list("Available", choices),
                buttons.into(),
                list("Toolbar", items),
            ])
            .spacing(12),
        )
        .width(720)
        .primary_action(widget::button::suggested("Done").on_press(Message::CloseToolbarEditor))
        .tertiary_action(
            widget::button::standard("Reset to Standard").on_press_maybe(
                (toolbar != chrome::standard_toolbar())
                    .then_some(Message::Toolbar(ToolbarEdit::Reset)),
            ),
        )
        .into()
}

/// An icon button's class: the theme's, with a selected toggle (Bold on,
/// the caret paragraph's alignment) drawn on the theme's selected
/// background and in its accent colour. The pinned libcosmic's icon style
/// keeps an enabled button's colours whether it is selected or not, so a
/// toggle's state was not visible (`UI_SPEC.md` §5: a selected state for
/// toggles, never colour alone).
fn toggle_button_class(selected: bool) -> cosmic::theme::Button {
    use cosmic::widget::button::{Catalog, Style};
    if !selected {
        return cosmic::theme::Button::Icon;
    }
    fn tint(mut style: Style, theme: &cosmic::Theme) -> Style {
        let cosmic = theme.cosmic();
        style.background = Some(cosmic::iced::Background::Color(
            cosmic.icon_button.selected_state_color().into(),
        ));
        style.icon_color = Some(cosmic.accent_text_color().into());
        style
    }
    cosmic::theme::Button::Custom {
        active: Box::new(|focused, theme| {
            tint(
                theme.active(focused, true, &cosmic::theme::Button::Icon),
                theme,
            )
        }),
        disabled: Box::new(|theme| theme.disabled(&cosmic::theme::Button::Icon)),
        hovered: Box::new(|focused, theme| {
            tint(
                theme.hovered(focused, true, &cosmic::theme::Button::Icon),
                theme,
            )
        }),
        pressed: Box::new(|focused, theme| {
            tint(
                theme.pressed(focused, true, &cosmic::theme::Button::Icon),
                theme,
            )
        }),
    }
}

/// Native document tabs followed by New Document.
///
/// The tab bar is bounded so that, when tabs overflow into its scroll arrows,
/// the New Document button after the last tab remains reachable.
fn tab_strip_view(model: &segmented_button::SingleSelectModel) -> Element<'_, Message> {
    const NEW_TAB_BUTTON: f32 = 32.0;
    const GAP: f32 = 4.0;
    cosmic::iced::widget::responsive(move |available| {
        let tab_width =
            (available.width - NEW_TAB_BUTTON - GAP - 2.0 * f32::from(CHROME_HORIZONTAL_PADDING))
                .max(0.0);
        let tabs = widget::tab_bar::horizontal(model)
            .minimum_button_width(96)
            .maximum_button_width(240)
            .on_activate(Message::TabActivated)
            .on_close(Message::TabCloseRequested)
            .width(Length::Shrink);
        let new_document = widget::button::icon(widget::icon::from_name("list-add-symbolic"))
            .name(command::metadata(CommandId::NewDocument).label)
            .tooltip(command_tooltip(CommandId::NewDocument))
            .width(NEW_TAB_BUTTON)
            .height(NEW_TAB_BUTTON)
            .on_press(Message::Command(CommandId::NewDocument));
        widget::row::with_children(vec![
            widget::container(tabs).max_width(tab_width).into(),
            new_document.into(),
        ])
        .spacing(GAP)
        .padding([0, CHROME_HORIZONTAL_PADDING])
        .align_y(cosmic::iced::Alignment::Center)
        .into()
    })
    .height(TAB_STRIP_HEIGHT)
    .into()
}

fn tab_entity(
    model: &segmented_button::SingleSelectModel,
    session_id: u64,
) -> Option<segmented_button::Entity> {
    model
        .iter()
        .find(|entity| model.data::<TabSession>(*entity) == Some(&TabSession(session_id)))
}

fn action_icon_button(
    command: CommandId,
    enabled: bool,
    toggle_state: ToggleState,
) -> Element<'static, Message> {
    let metadata = command::metadata(command);
    let icon_name = action_icon_name(command).expect("visible icon command must have an icon");
    let mut tooltip = command_tooltip(command);
    let description = match toggle_state {
        ToggleState::Mixed => {
            tooltip.push_str(" — mixed selection");
            "Mixed selection"
        }
        ToggleState::On => "Selected",
        ToggleState::Off => "Not selected",
        ToggleState::Unavailable => "Unavailable",
    };
    let selected = toggle_state == ToggleState::On;
    widget::button::icon(command_icon(icon_name))
        .name(metadata.label)
        .description(description)
        .tooltip(tooltip)
        .selected(selected)
        .class(toggle_button_class(selected))
        .width(32)
        .height(32)
        .on_press_maybe(enabled.then_some(Message::Command(command)))
        .into()
}

/// A command's icon: the COSMIC theme's by name, or one of the app's own
/// (`chrome::app_icon_svg`), recoloured as the theme's symbolic icons are.
fn command_icon(name: &'static str) -> widget::icon::Handle {
    match chrome::app_icon_svg(name) {
        Some(svg) => widget::icon::from_svg_bytes(svg).symbolic(true),
        None => widget::icon::from_name(name).into(),
    }
}

fn action_more_menu(state: &ActionBarState, commands: &[CommandId]) -> Element<'static, Message> {
    let key_binds: HashMap<KeyBind, MenuCommand> = commands
        .iter()
        .filter_map(|command| {
            command::metadata(*command)
                .shortcut
                .map(|shortcut| (shortcut.key_bind(), MenuCommand::Command(*command)))
        })
        .collect();
    let items = commands
        .iter()
        .map(|command| {
            let metadata = command::metadata(*command);
            let mut entry =
                menu::Entry::new(metadata.label.to_owned(), MenuCommand::Command(*command))
                    .enabled(state.enabled(*command));
            if matches!(command, CommandId::Bold | CommandId::Italic) {
                entry = entry.checked(state.toggle_state(*command) == ToggleState::On);
            } else if *command == CommandId::StylesPanel {
                entry = entry.checked(state.styles_visible);
            }
            menu::Item::Entry(entry)
        })
        .collect();
    let root = menu::Tree::with_children(
        RcElementWrapper::new(Element::from(menu::root("More ▾"))),
        menu::items(&key_binds, items),
    );
    menu::bar(vec![root])
        .item_height(ItemHeight::Dynamic(36))
        .item_width(ItemWidth::Uniform(260))
        .into()
}

impl AmiProApp {
    /// The Styles panel: a header with the close button, a live filter, and
    /// one row per style, the caret paragraph's style selected and none for
    /// a mixed selection (`UI_SPEC.md` §5 "Styles panel"; gaps R2–R4).
    fn styles_rail(&self, selected: Option<StyleId>) -> Element<'_, Message> {
        let filter = widget::search_input("Filter styles", self.rail.style_filter.clone())
            .id(widget::Id::new(STYLE_FILTER))
            .on_input(Message::StyleFilter)
            .on_submit(|_| Message::StyleFilterSubmit)
            .on_clear(Message::StyleFilter(String::new()))
            .width(Length::Fill);
        let modify = self.command_enabled(CommandId::ModifyCurrentStyle);
        let error =
            cosmic::iced::Color::from(cosmic::theme::active().cosmic().destructive_text_color());
        let rows: Vec<Element<'_, Message>> =
            filtered_styles(&self.document, &self.rail.style_filter)
                .into_iter()
                .map(|style| {
                    // Ami Pro marks a style stored in the document with a
                    // bullet and shows a name defined nowhere in red.
                    let missing = style.origin == amipro_document::StyleOrigin::Missing;
                    let mut name = widget::text::body(style_dialogs::listed_name(style))
                        .wrapping(Wrapping::None)
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
                    if missing {
                        name = name.class(cosmic::theme::Text::Color(error));
                    }
                    let row = widget::button::custom(
                        widget::row::with_children(vec![
                            widget::text::body("¶").into(),
                            name.into(),
                        ])
                        .spacing(8)
                        .align_y(cosmic::iced::Alignment::Center),
                    )
                    .class(cosmic::theme::Button::ListItem([6.0; 4]))
                    .selected(selected == Some(style.id))
                    .padding([6, 8])
                    .width(Length::Fill)
                    .on_press(Message::ApplyStyle(style.id));
                    let row: Element<'_, Message> = if missing {
                        widget::tooltip(
                            row,
                            widget::text::body("Defined nowhere: formatted as Body Text"),
                            widget::tooltip::Position::Bottom,
                        )
                        .into()
                    } else {
                        row.into()
                    };
                    widget::mouse_area(row)
                        .on_right_press(Message::StyleRowContext(Some(style.id)))
                        .into()
                })
                .collect();
        let list: Element<'_, Message> = if rows.is_empty() {
            widget::text::caption("No styles match").into()
        } else {
            // One right-click menu around the list, for the row the right
            // button pressed: a menu inside the scrolled list gets the
            // pointer in scrolled coordinates and takes no clicks once the
            // list is scrolled (user, 2026-10-13, on the page's menu).
            // Its entries are always there, so that a press and release
            // that arrive together still open it; they act on the row
            // when chosen.
            let target = self
                .rail
                .context_style
                .and_then(|style| self.document.styles().get(&style));
            let entry = |label: &str, action, enabled: bool| {
                menu::Item::Entry(
                    menu::Entry::new(label.to_owned(), MenuCommand::StyleRowMenu(action))
                        .enabled(target.is_some() && enabled),
                )
            };
            let missing =
                target.is_some_and(|style| style.origin == amipro_document::StyleOrigin::Missing);
            let removable = target.is_some_and(|style| {
                style.origin == amipro_document::StyleOrigin::Document
                    && !style.name.eq_ignore_ascii_case("Body Text")
            });
            let items = vec![
                entry("Apply", StyleRowAction::Apply, true),
                entry("Modify Style…", StyleRowAction::Modify, modify && !missing),
                entry(
                    "Create Style Based on This…",
                    StyleRowAction::CreateBasedOn,
                    !missing,
                ),
                menu::Item::Divider,
                entry("Rename…", StyleRowAction::Rename, true),
                entry("Remove", StyleRowAction::Remove, removable),
            ];
            widget::context_menu(
                rail_scroller(widget::column::with_children(rows).spacing(2).into()),
                Some(menu::items(&HashMap::new(), items)),
            )
            .on_close(Message::StyleRowContext(None))
            .into()
        };
        let header = widget::row::with_children(vec![
            widget::text::heading("Styles").into(),
            widget::space::horizontal().into(),
            widget::button::icon(widget::icon::from_name("list-add-symbolic"))
                .extra_small()
                .name("Create Style")
                .tooltip(command_tooltip(CommandId::CreateStyle))
                .on_press_maybe(
                    self.command_enabled(CommandId::CreateStyle)
                        .then_some(Message::Command(CommandId::CreateStyle)),
                )
                .into(),
            widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                .extra_small()
                .name("Close Styles")
                .tooltip(command_tooltip(CommandId::StylesPanel))
                .on_press(Message::Command(CommandId::StylesPanel))
                .into(),
        ])
        .align_y(cosmic::iced::Alignment::Center);
        widget::column::with_children(vec![header.into(), filter.into(), list])
            .spacing(8)
            .padding([8, 8, 8, 8])
            .height(Length::Fill)
            .into()
    }

    /// Whether `attribute` is on over the selection: the page
    /// selection in its own flow (main, frame, header, or cell text) on the
    /// page surface, else the text editor's.
    fn character_state(&self, attribute: CharacterAttribute) -> ToggleState {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            return self.sessions[self.active_session].page_attribute_state(attribute);
        }
        selected_character_toggle_state(&self.document, self.editor.cursor(), attribute)
    }

    /// The selected paragraphs' alignment on the page surface when they
    /// share one, for the Paragraph group's selected state; none in the
    /// default build or while frames are selected.
    fn selected_alignment(&self) -> Option<Alignment> {
        #[cfg(feature = "page-surface")]
        if self.page_surface && self.frame_selection.is_none() {
            let (start, end) = self.page_selection.ordered();
            let flow = self.document.flow(start.flow)?;
            let mut shared = None;
            for paragraph in flow.paragraphs.get(start.paragraph..=end.paragraph)? {
                let style = self.document.styles().get(&paragraph.style)?;
                let alignment = paragraph.format.alignment.unwrap_or(style.alignment);
                if shared.is_some_and(|shared| shared != alignment) {
                    return None;
                }
                shared = Some(alignment);
            }
            return shared;
        }
        None
    }

    /// The list every selected paragraph (or the caret's) shows through its
    /// style, if they share one (`UI_SPEC.md` §21).
    /// Insert › Page Break's check mark with the caret in a table cell:
    /// whether the caret's row starts a page with a break (§26, TP1).
    fn row_break_check(&self) -> Option<bool> {
        #[cfg(feature = "page-surface")]
        if self.page_surface && self.frame_selection.is_none() {
            let flow = self.page_selection.focus.flow;
            if flow != self.document.main_flow_id()
                && page_editing::table_row_of(&self.document, flow).is_some()
            {
                return Some(page_editing::row_has_break(&self.document, flow));
            }
        }
        None
    }

    fn selected_list(&self) -> Option<amipro_editing::lists::ListKind> {
        #[cfg(feature = "page-surface")]
        if self.page_surface && self.frame_selection.is_none() {
            let (start, end) = self.page_selection.ordered();
            let mut shared = None;
            for index in start.paragraph..=end.paragraph {
                let list =
                    amipro_editing::lists::paragraph_list(&self.document, start.flow, index)?;
                if shared.is_some_and(|shared| shared != list) {
                    return None;
                }
                shared = Some(list);
            }
            return shared;
        }
        None
    }

    /// Whether the window is too narrow to dock the rail beside the page.
    fn rail_overlays(&self) -> bool {
        self.rail
            .window_width
            .is_some_and(|width| width < RAIL_OVERLAY_WIDTH)
    }

    /// Whether the rail shows, docked or as an overlay.
    fn rail_open(&self) -> bool {
        if self.rail_overlays() {
            self.rail.overlay_open
        } else {
            self.styles_visible
        }
    }

    fn open_rail(&mut self) {
        if self.rail_overlays() {
            self.rail.overlay_open = true;
        } else {
            self.styles_visible = true;
        }
    }

    fn toggle_rail(&mut self) {
        if self.rail_overlays() {
            self.rail.overlay_open = !self.rail.overlay_open;
        } else {
            self.styles_visible = !self.styles_visible;
        }
    }

    /// Whether the status bar shows: View › Status Bar, and always while the
    /// message reports a failure, so an error is never concealed.
    fn status_bar_shown(&self) -> bool {
        let message = self.status.as_str();
        self.preferences.status_bar_visible
            || message.contains("failed")
            || message.contains("refused")
    }

    fn command_enabled(&self, command: CommandId) -> bool {
        let metadata = command::metadata(command);
        if metadata.implementation != Implementation::Implemented {
            return false;
        }
        // Delete also deletes selected frames (after confirmation), and the
        // picture in the caret's cell (§24).
        #[cfg(feature = "page-surface")]
        if self.drawing_open() {
            match command {
                CommandId::Delete | CommandId::SelectAll | CommandId::Paste => return true,
                CommandId::Copy | CommandId::Cut => {
                    return self
                        .draw
                        .as_ref()
                        .is_some_and(|mode| !mode.selection.is_empty());
                }
                _ => {}
            }
        }
        #[cfg(feature = "page-surface")]
        if command == CommandId::Delete
            && self.page_surface
            && (self.frame_selection.is_some() || self.caret_cell_frame().is_some())
        {
            return true;
        }
        match metadata.enabled {
            EnableRule::Always => true,
            EnableRule::HasDocument => !self.sessions.is_empty(),
            EnableRule::Paste => {
                #[cfg(feature = "page-surface")]
                let image = self.clipboard_has_image == Some(true);
                #[cfg(not(feature = "page-surface"))]
                let image = false;
                !self.sessions.is_empty() && (self.clipboard_has_text.unwrap_or(true) || image)
            }
            EnableRule::Save => self.dirty || !matches!(self.location, DocumentLocation::Native(_)),
            EnableRule::RevertToSaved => self.dirty && self.saved_file().is_some(),
            EnableRule::Undo => self.history.can_undo(),
            EnableRule::Redo => self.history.can_redo(),
            EnableRule::Selection => self.has_selection(),
            EnableRule::TextFormat => {
                #[cfg(feature = "page-surface")]
                if self.page_surface {
                    return self.frame_selection.is_none();
                }
                self.has_selection()
            }
            EnableRule::HasText => !self.editor.is_empty(),
            EnableRule::MultipleTabs => self.sessions.len() > 1,
            #[cfg(feature = "page-surface")]
            EnableRule::FrameSelected => self.page_surface && self.frame_selection.is_some(),
            #[cfg(feature = "page-surface")]
            EnableRule::PictureSelected => self.inspected_frame().is_some_and(|(frame, _)| {
                self.document.frame(frame).is_some_and(|frame| {
                    matches!(frame.content, amipro_document::FrameContent::Picture(_))
                })
            }),
            #[cfg(feature = "page-surface")]
            EnableRule::Charting | EnableRule::Equations => {
                self.page_surface && !self.drawing_open()
            }
            #[cfg(feature = "page-surface")]
            EnableRule::BitmapSelected => self.inspected_frame().is_some_and(|(frame, _)| {
                self.picture_natural(frame).is_some()
                    && self.document.frame(frame).is_some_and(|frame| {
                        matches!(&frame.content, amipro_document::FrameContent::Picture(picture)
                            if picture_inspector::is_bitmap(picture.format()))
                    })
            }),
            #[cfg(feature = "page-surface")]
            EnableRule::PictureTarget => {
                self.page_surface
                    && picture_insert::target(
                        &self.document,
                        self.frame_selection.as_ref(),
                        self.page_selection.focus,
                    )
                    .is_ok()
            }
            #[cfg(feature = "page-surface")]
            EnableRule::FramesGroupable => self
                .frame_selection
                .as_ref()
                .is_some_and(|selection| selection.groupable(&self.document).is_some()),
            #[cfg(feature = "page-surface")]
            EnableRule::FrameGroup => self
                .frame_selection
                .as_ref()
                .is_some_and(|selection| selection.group(&self.document).is_some()),
            #[cfg(feature = "page-surface")]
            EnableRule::OnField => self.field_at_caret().is_some(),
            #[cfg(feature = "page-surface")]
            EnableRule::InTable => self.table_command_enabled(command),
            #[cfg(feature = "page-surface")]
            EnableRule::InDrawing => self.draw_command_enabled(command),
            #[cfg(feature = "page-surface")]
            EnableRule::DrawingTarget => self.drawing_target_enabled(),
            #[cfg(feature = "page-surface")]
            EnableRule::TableTarget | EnableRule::TextToTable => {
                self.table_rule_enabled(metadata.enabled)
            }
            #[cfg(feature = "page-surface")]
            EnableRule::PreviousPage => self.page_surface && self.caret_page_number() > 1,
            #[cfg(feature = "page-surface")]
            EnableRule::NextPage => {
                self.page_surface && self.caret_page_number() < self.page_count()
            }
            #[cfg(not(feature = "page-surface"))]
            EnableRule::FrameSelected
            | EnableRule::FramesGroupable
            | EnableRule::FrameGroup
            | EnableRule::PreviousPage
            | EnableRule::NextPage
            | EnableRule::PictureTarget
            | EnableRule::PictureSelected
            | EnableRule::BitmapSelected
            | EnableRule::Charting
            | EnableRule::Equations
            | EnableRule::OnField
            | EnableRule::InTable
            | EnableRule::TableTarget
            | EnableRule::TextToTable
            | EnableRule::InDrawing
            | EnableRule::DrawingTarget => false,
            EnableRule::Future => false,
        }
    }

    /// The picture in the cell the caret is in, if any (§24).
    #[cfg(feature = "page-surface")]
    fn caret_cell_frame(&self) -> Option<amipro_document::FrameId> {
        page_editing::cell_frame(&self.document, self.page_selection.focus.flow)
    }

    fn has_selection(&self) -> bool {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            return self.frame_selection.is_none() && !self.page_selection.is_caret();
        }
        #[cfg(feature = "rich-editor-prototype")]
        {
            !self.prototype_selection.is_caret()
        }
        #[cfg(not(feature = "rich-editor-prototype"))]
        {
            let cursor = self.editor.cursor();
            cursor
                .selection
                .is_some_and(|anchor| anchor != cursor.position)
        }
    }

    fn dispatch_command(&mut self, command: CommandId) -> Task<cosmic::Action<Message>> {
        // Paste always reads the clipboard: its state comes from an
        // earlier look, which may have missed what is there now, and the
        // shortcut then did nothing (user, 2026-10-18).
        let paste = matches!(command, CommandId::Paste | CommandId::PastePlainText)
            && !self.sessions.is_empty();
        if !paste && !self.command_enabled(command) {
            return Task::none();
        }

        #[cfg(feature = "page-surface")]
        if self.draw_edit_command(command) {
            return Task::none();
        }
        match command {
            CommandId::NewDocument => {
                let mut document = new_document();
                document.title = next_untitled_title(&self.sessions);
                // The default language for new documents (Ami Pro's
                // `dictlang`; SP6).
                if !self.preferences.default_language.is_empty() {
                    document.set_language(amipro_document::Language::from_locale(
                        &self.preferences.default_language,
                    ));
                }
                let session = DocumentSession::new(
                    self.next_session_id,
                    document,
                    DocumentLocation::Untitled,
                    "New document".into(),
                );
                self.next_session_id += 1;
                self.sessions.push(session);
                self.active_session = self.sessions.len() - 1;
            }
            CommandId::Open => return self.open_dialog_task(),
            #[cfg(feature = "page-surface")]
            CommandId::InsertFile => return self.insert_file_task(),
            CommandId::CloseDocument => return self.request_close_tab(self.active_session),
            CommandId::Save => match self.location.save_request() {
                SaveRequest::ChooseDestination => return self.save_dialog_task(),
                SaveRequest::SaveTo(path) => {
                    self.save_to_native(path);
                }
                SaveRequest::SaveSam(path) => {
                    self.begin_sam_save(path, true);
                }
            },
            CommandId::SaveAs => return self.save_dialog_task(),
            CommandId::Quit => return self.begin_quit(),
            CommandId::Undo => self.restore_undo(),
            CommandId::Redo => self.restore_redo(),
            #[cfg(feature = "page-surface")]
            CommandId::Copy | CommandId::Cut if self.drawing_open() => {
                return self.copy_drawing(command == CommandId::Cut);
            }
            #[cfg(feature = "page-surface")]
            CommandId::Paste if self.drawing_open() => {
                return cosmic::iced::clipboard::read_data::<draw_files::DrawingPaste>()
                    .map(|data| cosmic::Action::from(Message::DrawPaste(data)));
            }
            #[cfg(feature = "page-surface")]
            CommandId::Copy | CommandId::Cut if self.copy_cells().is_some() => {
                let text = self.copy_cells().unwrap_or_default();
                self.clipboard_has_text = Some(true);
                if command == CommandId::Cut {
                    self.delete_selected_text();
                }
                return cosmic::iced::clipboard::write(text);
            }
            CommandId::Copy => {
                if let Some(selected) = self.selected_text() {
                    self.clipboard_has_text = Some(true);
                    return cosmic::iced::clipboard::write(selected);
                }
            }
            CommandId::Cut => {
                #[cfg(feature = "page-surface")]
                if self.refuse_in_formula_cell() {
                    return Task::none();
                }
                if let Some(selected) = self.selected_text() {
                    self.clipboard_has_text = Some(true);
                    self.delete_selected_text();
                    return cosmic::iced::clipboard::write(selected);
                }
            }
            // Rich text first (IM12): LibreOffice, Word, and WordPad offer
            // RTF beside the text.
            #[cfg(feature = "page-surface")]
            CommandId::Paste if self.page_surface => {
                let session_id = self.id;
                return cosmic::iced::clipboard::read_data::<import::ClipboardRtf>().map(
                    move |rtf| cosmic::Action::from(Message::ClipboardRtf { session_id, rtf }),
                );
            }
            CommandId::Paste | CommandId::PastePlainText => {
                let session_id = self.id;
                return cosmic::iced::clipboard::read().map(move |contents| {
                    cosmic::Action::from(Message::ClipboardRead {
                        command,
                        session_id,
                        contents,
                    })
                });
            }
            CommandId::Delete => self.delete_selected_text(),
            CommandId::SelectAll => self.select_all_text(),
            CommandId::StylesPanel => self.toggle_rail(),
            CommandId::ShowToolbar => {
                self.preferences.toolbar_visible = !self.preferences.toolbar_visible;
            }
            #[cfg(feature = "page-surface")]
            CommandId::Ruler => {
                self.preferences.ruler_visible = !self.preferences.ruler_visible;
            }
            #[cfg(feature = "page-surface")]
            CommandId::StatusBar => {
                self.preferences.status_bar_visible = !self.preferences.status_bar_visible;
            }
            #[cfg(feature = "page-surface")]
            CommandId::SpecialCharacter => self.special_character = true,
            CommandId::RevertToSaved => {
                if self.saved_file().is_some() {
                    self.revert_pending = true;
                }
            }
            CommandId::Password => {
                self.password_dialog = Some(password_dialog::PasswordDialog::set(
                    self.password.is_some(),
                ));
            }
            #[cfg(feature = "page-surface")]
            CommandId::ProtectedText => self.toggle_page_attribute(CharacterAttribute::Protected),
            #[cfg(feature = "page-surface")]
            CommandId::NoHyphenation => {
                self.toggle_page_attribute(CharacterAttribute::NoHyphenation);
            }
            CommandId::NewFromStyleSheet => {
                let mut state = new_document_dialog::NewDocumentDialog::new(
                    &self.style_folders(),
                    &self.preferences.recent_sheets,
                    &self.preferences.default_sheet,
                );
                Self::preview_sheet(&mut state);
                self.new_document_dialog = Some(state);
            }
            CommandId::Preferences => {
                self.preferences_dialog = Some(preferences_dialog::PreferencesState {
                    page: preferences_dialog::Page::General,
                    undo_levels: self.preferences.undo_levels.to_string(),
                    recent_files: self.preferences.recent_count.to_string(),
                    custom_zoom: self.preferences.custom_zoom.to_string(),
                });
            }
            CommandId::CustomizeToolbar => {
                // The bar shows while it is edited: changes show there.
                self.preferences.toolbar_visible = true;
                self.toolbar_editor = Some(ToolbarEditor::default());
            }
            CommandId::Styles => {
                // Select a Style (Ctrl+Y, CK7): the Styles panel, its filter
                // focused; Enter applies the first style listed. Another
                // panel gives way to the list.
                #[cfg(feature = "page-surface")]
                self.close_rail_panels();
                self.open_rail();
                return widget::text_input::focus::<Message>(widget::Id::new(STYLE_FILTER))
                    .chain(Task::done(Message::Repaint))
                    .map(cosmic::Action::from);
            }
            CommandId::CreateStyle => {
                let current = self.caret_paragraph().map(|paragraph| paragraph.style);
                self.create_style = Some(style_dialogs::CreateStyle::new(
                    self.document.styles(),
                    current,
                ));
            }
            #[cfg(feature = "page-surface")]
            CommandId::ModifyCurrentStyle => {
                self.open_style_properties(None);
            }
            #[cfg(feature = "page-surface")]
            CommandId::BulletNumberStyle => {
                self.open_style_properties(None);
                if let Some(state) = self.style_properties.as_mut() {
                    state.open =
                        std::collections::BTreeSet::from([style_properties::Section::Bullets]);
                }
            }
            #[cfg(feature = "page-surface")]
            CommandId::PageLayout | CommandId::PageSetup => self.open_page_layout_properties(),
            #[cfg(feature = "page-surface")]
            CommandId::FastFormat => self.toggle_fast_format(),
            #[cfg(feature = "page-surface")]
            CommandId::InsertRuler => self.paragraph_ruler(true),
            #[cfg(feature = "page-surface")]
            CommandId::RemoveRuler => self.paragraph_ruler(false),
            #[cfg(feature = "page-surface")]
            CommandId::InsertPageLayout => self.insert_page_layout_command(),
            #[cfg(feature = "page-surface")]
            CommandId::RemovePageLayout => self.remove_page_layout_command(),
            #[cfg(feature = "page-surface")]
            CommandId::RevertPageLayout => self.revert_page_layout_command(),
            #[cfg(feature = "page-surface")]
            CommandId::LineNumbering => {
                self.open_page_layout_properties();
                if let Some(panel) = self.page_layout_properties.as_mut() {
                    panel
                        .open
                        .insert(page_layout_properties::Section::LineNumbering);
                }
            }
            #[cfg(feature = "page-surface")]
            CommandId::ParagraphProperties => {
                self.open_paragraph_properties(paragraph_properties::Section::Indention);
            }
            #[cfg(feature = "page-surface")]
            CommandId::IncreaseIndent | CommandId::DecreaseIndent => {
                self.step_indent(command == CommandId::IncreaseIndent);
            }
            #[cfg(feature = "page-surface")]
            CommandId::UpdateFields => self.update_fields_command(),
            #[cfg(feature = "page-surface")]
            CommandId::UpdateAllFields => self.run_field_update(
                amipro_editing::fields::UpdateScope::All,
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
            ),
            #[cfg(feature = "page-surface")]
            CommandId::NextField => self.go_to_field(true),
            #[cfg(feature = "page-surface")]
            CommandId::PreviousField => self.go_to_field(false),
            #[cfg(feature = "page-surface")]
            CommandId::ShowPowerFields => self.toggle_show_codes(),
            #[cfg(feature = "page-surface")]
            CommandId::FieldShading => {
                self.preferences.field_shading = !self.preferences.field_shading;
                self.save_preferences();
            }
            #[cfg(feature = "page-surface")]
            CommandId::ShowNotes => {
                self.preferences.show_notes = !self.preferences.show_notes;
                self.save_preferences();
            }
            #[cfg(feature = "page-surface")]
            CommandId::ShowBookmarks => {
                self.preferences.show_bookmarks = !self.preferences.show_bookmarks;
                self.save_preferences();
            }
            #[cfg(feature = "page-surface")]
            CommandId::InsertNote => self.insert_note(),
            #[cfg(feature = "page-surface")]
            CommandId::Footnote => self.insert_footnote(),
            #[cfg(feature = "page-surface")]
            CommandId::CrossReference => self.cross_reference(),
            #[cfg(feature = "page-surface")]
            CommandId::Spelling => return self.open_proofing(proofing_panel::Page::Spelling, true),
            #[cfg(feature = "page-surface")]
            CommandId::Thesaurus => {
                return self.open_proofing(proofing_panel::Page::Thesaurus, true);
            }
            #[cfg(feature = "page-surface")]
            CommandId::GrammarCheck => {
                return self.open_proofing(proofing_panel::Page::Grammar, true);
            }
            #[cfg(feature = "page-surface")]
            CommandId::ReadabilityStatistics => {
                let task = self.open_proofing(proofing_panel::Page::Grammar, false);
                self.show_statistics();
                return task;
            }
            #[cfg(feature = "page-surface")]
            CommandId::Sort => self.open_sort(),
            #[cfg(feature = "page-surface")]
            CommandId::Links => self.open_links(),
            #[cfg(feature = "page-surface")]
            CommandId::CheckForUpdates => return self.open_updates(),
            #[cfg(feature = "page-surface")]
            CommandId::RecoverDocuments => self.open_recovery(),
            #[cfg(feature = "page-surface")]
            CommandId::Help => return self.open_help_for_context(),
            #[cfg(feature = "page-surface")]
            CommandId::HelpContents => return self.open_help(help::Page::Contents, false),
            #[cfg(feature = "page-surface")]
            CommandId::HelpSearch => return self.open_help(help::Page::Search, true),
            #[cfg(feature = "page-surface")]
            CommandId::Charting => return self.open_charting(),
            #[cfg(feature = "page-surface")]
            CommandId::Equations => return self.open_equations(),
            #[cfg(feature = "page-surface")]
            CommandId::PasteLatexEquation => return self.paste_latex_equation(),
            #[cfg(feature = "page-surface")]
            CommandId::ChartFromTable => return self.chart_from_table(),
            #[cfg(feature = "page-surface")]
            CommandId::PasteSpecial => return self.open_paste_special(),
            #[cfg(feature = "page-surface")]
            CommandId::Language => {
                let task = self.open_proofing(proofing_panel::Page::Spelling, false);
                if let Some(panel) = self.proofing.as_mut() {
                    panel.show_options = true;
                }
                return task;
            }
            #[cfg(feature = "page-surface")]
            CommandId::SpellAsYouType => {
                self.preferences.spell_as_you_type = !self.preferences.spell_as_you_type;
                self.save_preferences();
                return self.check_as_you_type();
            }
            #[cfg(feature = "page-surface")]
            CommandId::Notes => self.open_reference_panel(references_panel::Kind::Notes),
            #[cfg(feature = "page-surface")]
            CommandId::Bookmarks => self.open_reference_panel(references_panel::Kind::Bookmarks),
            #[cfg(feature = "page-surface")]
            CommandId::FootnoteOptions => {
                self.open_reference_panel(references_panel::Kind::Footnotes)
            }
            #[cfg(feature = "page-surface")]
            CommandId::ContentsIndex => self.open_contents_panel(contents_panel::Page::Contents),
            #[cfg(feature = "page-surface")]
            CommandId::MasterDocument => self.open_master_panel(),
            #[cfg(feature = "page-surface")]
            CommandId::Merge | CommandId::MergeField => return self.open_merge_panel(),
            #[cfg(feature = "page-surface")]
            CommandId::PrintEnvelope => return self.open_envelope_panel(),
            #[cfg(feature = "page-surface")]
            CommandId::MarkIndexEntry | CommandId::MarkTocEntry => {
                self.open_contents_panel(contents_panel::Page::Entries)
            }
            #[cfg(feature = "page-surface")]
            CommandId::MarkGlossary => self.open_glossary_panel(),
            #[cfg(feature = "page-surface")]
            CommandId::InsertGlossary => self.insert_glossary_record(),
            #[cfg(feature = "page-surface")]
            CommandId::LockField => self.toggle_field_lock(),
            #[cfg(feature = "page-surface")]
            CommandId::Field => self.open_fields_panel(fields_panel::Page::Power),
            #[cfg(feature = "page-surface")]
            CommandId::DateTime => self.open_fields_panel(fields_panel::Page::DateTime),
            #[cfg(feature = "page-surface")]
            CommandId::EditField => self.open_fields_panel(fields_panel::Page::ThisField),
            #[cfg(feature = "page-surface")]
            CommandId::DocumentProperties => {
                self.open_document_properties(document_properties::Section::Description)
            }
            #[cfg(feature = "page-surface")]
            CommandId::ExportPdf => return self.export_pdf_task(),
            #[cfg(feature = "page-surface")]
            CommandId::ExportDocx => return self.export_task(amipro_exchange::Format::Docx),
            #[cfg(feature = "page-surface")]
            CommandId::ExportOdt => return self.export_task(amipro_exchange::Format::Odt),
            #[cfg(feature = "page-surface")]
            CommandId::ExportRtf => return self.export_task(amipro_exchange::Format::Rtf),
            #[cfg(feature = "page-surface")]
            CommandId::ExportText => return self.export_task(amipro_exchange::Format::Text),
            #[cfg(feature = "page-surface")]
            CommandId::ExportMarkdown => {
                return self.export_task(amipro_exchange::Format::Markdown);
            }
            #[cfg(feature = "page-surface")]
            CommandId::ExportFolder => self.open_export_folder(),
            #[cfg(feature = "page-surface")]
            CommandId::ImportFolder => self.open_import_folder(),
            #[cfg(feature = "page-surface")]
            CommandId::Print => return self.print_task(false),
            #[cfg(feature = "page-surface")]
            CommandId::PrintCurrentPage => return self.print_task(true),
            #[cfg(feature = "page-surface")]
            CommandId::PrintOptions => self.open_print_panel(),
            #[cfg(feature = "page-surface")]
            CommandId::WordCount => {
                self.open_document_properties(document_properties::Section::Statistics)
            }
            CommandId::StyleManagement => {
                let sheet = self.document_style_sheet();
                self.style_management = Some(style_dialogs::StyleManagement {
                    selected: None,
                    sheet,
                });
            }
            CommandId::UseAnotherStyleSheet => {
                let folders = self.style_sheet_folders();
                self.use_sheet = Some(style_dialogs::UseStyleSheet::new(folders));
            }
            CommandId::SaveAsStyleSheet => {
                let description = amipro_formats::document_description(&self.document);
                self.save_sheet = Some(description);
                self.save_sheet_contents = true;
            }
            CommandId::Bold => self.toggle_selected_character_trait(CharacterAttribute::Bold),
            CommandId::Italic => self.toggle_selected_character_trait(CharacterAttribute::Italic),
            CommandId::Underline => {
                self.toggle_selected_character_trait(CharacterAttribute::Underline);
            }
            #[cfg(feature = "page-surface")]
            CommandId::AlignLeft => self.align_page_paragraphs(Alignment::Left),
            #[cfg(feature = "page-surface")]
            CommandId::Center => self.align_page_paragraphs(Alignment::Centre),
            #[cfg(feature = "page-surface")]
            CommandId::AlignRight => self.align_page_paragraphs(Alignment::Right),
            #[cfg(feature = "page-surface")]
            CommandId::Justify => self.align_page_paragraphs(Alignment::Justify),
            #[cfg(feature = "page-surface")]
            CommandId::EditFormula => return self.edit_formula(),
            #[cfg(feature = "page-surface")]
            CommandId::QuickAddRow
            | CommandId::QuickAddColumn
            | CommandId::RecalculateTable
            | CommandId::Table
            | CommandId::InsertRowAbove
            | CommandId::InsertRowBelow
            | CommandId::InsertColumnLeft
            | CommandId::InsertColumnRight
            | CommandId::DeleteRow
            | CommandId::DeleteColumn
            | CommandId::DeleteTable
            | CommandId::SelectColumn
            | CommandId::SelectRow
            | CommandId::SelectTable
            | CommandId::ConnectCells
            | CommandId::TableHeadings
            | CommandId::LeaderNone
            | CommandId::LeaderDots
            | CommandId::LeaderDashes
            | CommandId::LeaderLine
            | CommandId::ProtectCells
            | CommandId::DistributeRows
            | CommandId::DistributeColumns
            | CommandId::SortAscending
            | CommandId::SortDescending
            | CommandId::TableStyleGrid
            | CommandId::TableStyleOutline
            | CommandId::TableStyleHeaderRow
            | CommandId::TableStyleBandedRows
            | CommandId::TableStylePlain
            | CommandId::TextToTable
            | CommandId::TableToText
            | CommandId::TableProperties => self.run_table_command(command),
            #[cfg(feature = "page-surface")]
            CommandId::Drawing => self.insert_drawing(),
            #[cfg(feature = "page-surface")]
            CommandId::DrawProperties
            | CommandId::DrawExtract
            | CommandId::DrawApply
            | CommandId::DrawRotateLeft
            | CommandId::DrawRotateRight
            | CommandId::DrawRotate
            | CommandId::DrawFlipHorizontal
            | CommandId::DrawFlipVertical
            | CommandId::DrawGroup
            | CommandId::DrawUngroup
            | CommandId::DrawFront
            | CommandId::DrawBack
            | CommandId::DrawForward
            | CommandId::DrawBackward
            | CommandId::DrawAlignLeft
            | CommandId::DrawAlignCentre
            | CommandId::DrawAlignRight
            | CommandId::DrawAlignTop
            | CommandId::DrawAlignMiddle
            | CommandId::DrawAlignBottom
            | CommandId::DrawDistributeAcross
            | CommandId::DrawDistributeDown
            | CommandId::DrawDuplicate
            | CommandId::DrawSnap
            | CommandId::DrawGrid
            | CommandId::DrawSelectAll
            | CommandId::ImportDrawing
            | CommandId::SaveDrawingAs
            | CommandId::DrawDone => return self.run_draw_command(command),
            #[cfg(feature = "page-surface")]
            CommandId::Bullets => self.toggle_page_list(amipro_editing::lists::ListKind::Bullets),
            #[cfg(feature = "page-surface")]
            CommandId::Numbering => {
                self.toggle_page_list(amipro_editing::lists::ListKind::Numbering);
            }
            #[cfg(feature = "page-surface")]
            CommandId::Promote => {
                self.change_page_level(amipro_editing::outline::LevelChange::Promote);
            }
            #[cfg(feature = "page-surface")]
            CommandId::Demote => {
                self.change_page_level(amipro_editing::outline::LevelChange::Demote);
            }
            #[cfg(feature = "page-surface")]
            CommandId::MoveUp | CommandId::MoveDown => {
                self.move_section(command == CommandId::MoveDown);
            }
            CommandId::NextDocument => return self.cycle_tab(1),
            CommandId::PreviousDocument => return self.cycle_tab(-1),
            CommandId::About => {
                self.show_about = true;
                self.help_drawer = None;
                self.set_show_context(true);
            }
            CommandId::KeyboardShortcuts => {
                self.show_about = false;
                self.help_drawer = Some(help_drawers::HelpDrawer::Shortcuts {
                    filter: String::new(),
                });
                self.set_show_context(true);
            }
            CommandId::Compatibility => {
                self.show_about = false;
                self.help_drawer = Some(self.compatibility_check());
                self.set_show_context(true);
            }
            #[cfg(feature = "page-surface")]
            CommandId::BringToFront
            | CommandId::SendToBack
            | CommandId::GroupFrames
            | CommandId::UngroupFrames => self.arrange_frames(command),
            #[cfg(feature = "page-surface")]
            CommandId::NextFrame => self.select_next_frame(),
            #[cfg(feature = "page-surface")]
            CommandId::WordUnderline => {
                self.toggle_page_attribute(CharacterAttribute::WordUnderline);
            }
            #[cfg(feature = "page-surface")]
            CommandId::UpperCase => {
                self.toggle_page_attribute(CharacterAttribute::Caps(Caps::Upper))
            }
            #[cfg(feature = "page-surface")]
            CommandId::LowerCase => {
                self.toggle_page_attribute(CharacterAttribute::Caps(Caps::Lower))
            }
            #[cfg(feature = "page-surface")]
            CommandId::InitialCaps => {
                self.toggle_page_attribute(CharacterAttribute::Caps(Caps::Initial));
            }
            #[cfg(feature = "page-surface")]
            CommandId::SmallCaps => {
                self.toggle_page_attribute(CharacterAttribute::Caps(Caps::Small))
            }
            #[cfg(feature = "page-surface")]
            CommandId::SpecialEffects => {
                let session = &mut self.sessions[self.active_session];
                let states = character_dialogs::EFFECTS.map(|(attribute, _)| {
                    session.page_attribute_state(attribute) == ToggleState::On
                });
                let overstrike = session.selection_overstrike();
                session.special_effects =
                    Some(character_dialogs::SpecialEffects::new(states, overstrike));
            }
            #[cfg(feature = "page-surface")]
            CommandId::Font => {
                let session = &mut self.sessions[self.active_session];
                let (family, size, colour) = session.selection_font();
                let mut families = page_layout::installed_families();
                families.extend(
                    session
                        .document
                        .styles()
                        .values()
                        .map(|style| style.font_family.clone()),
                );
                session.font_dialog = Some(character_dialogs::FontDialog::new(
                    families, &family, size, colour,
                ));
            }
            #[cfg(feature = "page-surface")]
            CommandId::ClearDirectFormatting => self.normal_page_text(),
            #[cfg(feature = "page-surface")]
            CommandId::Find => return self.open_find(false),
            #[cfg(feature = "page-surface")]
            CommandId::Replace => return self.open_find(true),
            #[cfg(feature = "page-surface")]
            CommandId::FindNext => return self.find_step(true),
            #[cfg(feature = "page-surface")]
            CommandId::FindPrevious => return self.find_step(false),
            #[cfg(feature = "page-surface")]
            CommandId::GoToPage => {
                let session = &mut self.sessions[self.active_session];
                session.go_to = Some(GoToState {
                    page: session.caret_page_number().to_string(),
                    error: None,
                    item: 0,
                });
            }
            #[cfg(feature = "page-surface")]
            CommandId::HeaderFooter => {
                let at = self.page_selection.ordered().0;
                if at.flow == self.document.main_flow_id() && self.frame_selection.is_none() {
                    self.floating_dialog =
                        Some(page_dialogs::FloatingDialog::new(&self.document, at));
                } else {
                    self.status = "Floating headers and footers go in the main text".into();
                }
            }
            #[cfg(feature = "page-surface")]
            CommandId::OutlineNumbering => {
                self.outline_numbering =
                    Some(page_dialogs::OutlineNumberingDialog::new(&self.document));
            }
            #[cfg(feature = "page-surface")]
            CommandId::InsertBullet => {
                if self.frame_selection.is_none() {
                    let last = self.preferences.last_bullet;
                    self.bullet_dialog = Some(page_dialogs::BulletDialog::new(last));
                }
            }
            #[cfg(feature = "page-surface")]
            CommandId::PageNumber => {
                if self.frame_selection.is_none() {
                    let at = self.page_selection.ordered().0;
                    let page = self.caret_page_number();
                    self.page_number_dialog = Some(page_dialogs::PageNumberDialog::new(
                        &self.document,
                        at,
                        page,
                    ));
                }
            }
            #[cfg(feature = "page-surface")]
            CommandId::Frame => {
                let page = self.page_in_view();
                let paper = self.paper_of(page);
                let remembered = self.preferences.last_frame;
                let session = &mut self.sessions[self.active_session];
                session.drawing_frame = false;
                session.create_frame_dialog = Some(page_dialogs::CreateFrameDialog::new(
                    page, paper, remembered,
                ));
            }
            #[cfg(feature = "page-surface")]
            CommandId::DrawFrame => self.start_drawing_frame(),
            #[cfg(feature = "page-surface")]
            CommandId::Image => return self.choose_image_task(),
            #[cfg(feature = "page-surface")]
            CommandId::PageBreak => {
                self.insert_page_break(BreakSwitch::None);
            }
            #[cfg(feature = "page-surface")]
            CommandId::ColumnBreak => self.insert_column_break(),
            #[cfg(feature = "page-surface")]
            CommandId::GoToHeader => {
                self.go_to_band(amipro_editing::layout_setup::Band::Header);
            }
            #[cfg(feature = "page-surface")]
            CommandId::GoToFooter => {
                self.go_to_band(amipro_editing::layout_setup::Band::Footer);
            }
            #[cfg(feature = "page-surface")]
            CommandId::GoToRuler => self.go_to_ruler(),
            #[cfg(feature = "page-surface")]
            CommandId::PreviousPage => {
                let session = &mut self.sessions[self.active_session];
                let page = session.caret_page_number().saturating_sub(1).max(1);
                return session.go_to_page(page, false);
            }
            #[cfg(feature = "page-surface")]
            CommandId::NextPage => {
                let session = &mut self.sessions[self.active_session];
                let page = session.caret_page_number() + 1;
                return session.go_to_page(page, true);
            }
            #[cfg(feature = "page-surface")]
            CommandId::FrameProperties | CommandId::GraphicsScaling => self.open_rail(),
            // Tools › Image Processing…: the Image section of the selected
            // bitmap's Frame Properties (`UI_SPEC.md` §42.3).
            #[cfg(feature = "page-surface")]
            CommandId::ImageProcessing => {
                self.open_rail();
                self.sessions[self.active_session].status =
                    "Image Processing is in Frame Properties, under Image".into();
            }
            #[cfg(feature = "page-surface")]
            CommandId::ZoomIn
            | CommandId::ZoomOut
            | CommandId::ActualSize
            | CommandId::Enlarged
            | CommandId::CustomZoom
            | CommandId::FitPageWidth => {
                let percent = self.sessions[self.active_session].page_view().percent();
                let custom = self.preferences.custom_zoom;
                self.zoom_before_full_page = None;
                self.zoom = match command {
                    CommandId::ZoomIn => Zoom::zoom_in(percent),
                    CommandId::ZoomOut => Zoom::zoom_out(percent),
                    CommandId::ActualSize => Zoom::Percent(100),
                    CommandId::Enlarged => Zoom::Percent(200),
                    CommandId::CustomZoom => Zoom::percent(custom),
                    _ => Zoom::FitWidth,
                };
            }
            // Ami Pro's Full Page goes back to the view before it.
            #[cfg(feature = "page-surface")]
            CommandId::FullPage => match self.zoom_before_full_page.take() {
                Some(before) if self.zoom == Zoom::FullPage => self.zoom = before,
                _ => {
                    self.zoom_before_full_page = Some(self.zoom);
                    self.zoom = Zoom::FullPage;
                }
            },
            #[cfg(feature = "page-surface")]
            CommandId::TwoPages => self.facing = !self.facing,
            #[cfg(feature = "page-surface")]
            CommandId::ShowTabsReturns => {
                self.preferences.tabs_returns = !self.preferences.tabs_returns;
            }
            #[cfg(feature = "page-surface")]
            CommandId::ColumnGuides => {
                self.preferences.column_guides = !self.preferences.column_guides;
            }
            #[cfg(feature = "page-surface")]
            CommandId::MarginsInColour => {
                self.preferences.margins_in_colour = !self.preferences.margins_in_colour;
            }
            #[cfg(feature = "page-surface")]
            CommandId::ShowPictures => {
                self.preferences.show_pictures = !self.preferences.show_pictures;
            }
            #[cfg(feature = "page-surface")]
            CommandId::TableAddresses => {
                self.preferences.table_addresses = !self.preferences.table_addresses;
            }
            #[cfg(feature = "page-surface")]
            CommandId::OutlineNavigator => {
                if self.outline_navigator.is_some() && self.rail_open() {
                    self.sessions[self.active_session].outline_navigator = None;
                    self.toggle_rail();
                } else {
                    self.open_outline_navigator();
                }
            }
            // Every open document at once, as Ami Pro applies its INI bit.
            #[cfg(feature = "page-surface")]
            CommandId::UseOutlineStyles => {
                self.preferences.outline_styles = !self.preferences.outline_styles;
                let on = self.preferences.outline_styles;
                for session in &mut self.sessions {
                    session.outline_styles = on;
                    if session.outline.is_some() {
                        session.refresh_rich_layout();
                    }
                }
                self.status = if on {
                    "Outline mode shows each level in its outline style".into()
                } else {
                    "Outline mode shows the paragraphs' own styles".into()
                };
            }
            #[cfg(feature = "page-surface")]
            CommandId::DraftMode => {
                let session = &mut self.sessions[self.active_session];
                session.draft = match session.draft {
                    Some(_) => None,
                    None => Some(session.draft_width()),
                };
                session.outline = None;
                session.refresh_rich_layout();
                session.status = if session.draft.is_some() {
                    "Draft mode: the text at the window's width".into()
                } else {
                    "Layout mode".into()
                };
            }
            #[cfg(feature = "page-surface")]
            CommandId::OutlineExpand | CommandId::OutlineContract => {
                self.outline_expand(command == CommandId::OutlineExpand);
            }
            #[cfg(feature = "page-surface")]
            CommandId::ShowAllLevels
            | CommandId::ShowLevel1
            | CommandId::ShowLevel2
            | CommandId::ShowLevel3
            | CommandId::ShowLevel4
            | CommandId::ShowLevel5
            | CommandId::ShowLevel6
            | CommandId::ShowLevel7
            | CommandId::ShowLevel8
            | CommandId::ShowLevel9 => {
                let levels = match command {
                    CommandId::ShowLevel1 => Some(1),
                    CommandId::ShowLevel2 => Some(2),
                    CommandId::ShowLevel3 => Some(3),
                    CommandId::ShowLevel4 => Some(4),
                    CommandId::ShowLevel5 => Some(5),
                    CommandId::ShowLevel6 => Some(6),
                    CommandId::ShowLevel7 => Some(7),
                    CommandId::ShowLevel8 => Some(8),
                    CommandId::ShowLevel9 => Some(9),
                    _ => None,
                };
                self.outline_show_levels(levels);
            }
            #[cfg(feature = "page-surface")]
            CommandId::OutlineMode => {
                let outline_styles = self.preferences.outline_styles;
                let session = &mut self.sessions[self.active_session];
                session.outline_styles = outline_styles;
                session.outline = match session.outline {
                    Some(_) => None,
                    None => Some(session.draft_width()),
                };
                session.draft = None;
                session.refresh_rich_layout();
                session.status = if session.outline.is_some() {
                    "Outline mode: the text by outline level".into()
                } else {
                    "Layout mode".into()
                };
            }
            CommandId::FullScreen => return self.set_clean_screen(!self.clean_screen),
            #[cfg(feature = "page-surface")]
            CommandId::ContinuousPages => self.facing = false,
            // Every command is handled with the page surface; the text
            // editor leaves the Future ones.
            #[cfg(not(feature = "page-surface"))]
            _ => return Task::none(),
        }
        self.update_title()
    }

    fn selected_text(&self) -> Option<String> {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            return page_editing::selected_text(&self.document, self.page_selection);
        }
        #[cfg(feature = "rich-editor-prototype")]
        {
            let (start, end) = self.prototype_selection.ordered();
            (start != end).then(|| self.document.plain_text()[start..end].to_owned())
        }
        #[cfg(not(feature = "rich-editor-prototype"))]
        {
            self.editor.selection()
        }
    }

    fn delete_selected_text(&mut self) {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            if self.frame_selection.is_some() {
                self.pending_frame_delete = true;
            } else if !self.page_selection.is_caret() || self.caret_cell_frame().is_some() {
                drop(self.update_page(PageAction::Delete));
            }
            return;
        }
        #[cfg(feature = "rich-editor-prototype")]
        self.update_prototype(PrototypeAction::Delete);
        #[cfg(not(feature = "rich-editor-prototype"))]
        self.apply_standard_action(text_editor::Action::Edit(text_editor::Edit::Delete));
    }

    fn insert_clipboard_text(&mut self, contents: String) {
        if contents.is_empty() {
            return;
        }
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            // Cells, or tabbed text, into a table (`UI_SPEC.md` §32).
            if self.paste_cells(&contents) {
                return;
            }
            drop(self.update_page(PageAction::Insert(contents)));
            return;
        }
        #[cfg(feature = "rich-editor-prototype")]
        self.update_prototype(PrototypeAction::Insert(contents));
        #[cfg(not(feature = "rich-editor-prototype"))]
        self.apply_standard_action(text_editor::Action::Edit(text_editor::Edit::Paste(
            std::sync::Arc::new(contents),
        )));
    }

    fn select_all_text(&mut self) {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            drop(self.update_page(PageAction::SelectAll));
            return;
        }
        #[cfg(feature = "rich-editor-prototype")]
        self.update_prototype(PrototypeAction::SelectAll);
        #[cfg(not(feature = "rich-editor-prototype"))]
        self.apply_standard_action(text_editor::Action::SelectAll);
    }

    fn apply_standard_action(&mut self, action: text_editor::Action) {
        let is_edit = standard_action_changes_document(&action);
        let cursor_before = self.editor.cursor();
        let single_delete = cursor_before.selection.is_none()
            && matches!(
                action,
                text_editor::Action::Edit(text_editor::Edit::Backspace | text_editor::Edit::Delete)
            );
        self.editor.perform(action);
        if is_edit {
            self.sessions[self.active_session]
                .reconcile_standard_edit(cursor_before, single_delete);
        }
    }

    fn begin_quit(&mut self) -> Task<cosmic::Action<Message>> {
        self.quit_requested = true;
        self.continue_quit()
    }

    /// Reads the clipboard for Paste's state; with `retry`, a read that
    /// finds nothing is tried once more ([`Self::retry_clipboard`]).
    fn clipboard_availability_task(retry: bool) -> Task<cosmic::Action<Message>> {
        cosmic::iced::clipboard::read().map(move |contents| {
            cosmic::Action::from(Message::ClipboardAvailability {
                has_text: contents.is_some_and(|contents| !contents.is_empty()),
                retry,
            })
        })
    }

    /// The clipboard looked at again a moment later. The clipboard's
    /// worker learns of the window's focus on its own, and a read sent as
    /// focus arrives can run first and find nothing, which left Paste
    /// greyed out after a copy in another program (user, 2026-10-18). A
    /// thread waits, not the executor.
    fn retry_clipboard() -> Task<cosmic::Action<Message>> {
        Task::future(async {
            let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(150));
                let _ = done.send(());
            });
            let _ = wait.await;
            cosmic::Action::from(Message::RetryClipboard)
        })
    }

    fn continue_quit(&mut self) -> Task<cosmic::Action<Message>> {
        if let Some(index) = self.sessions.iter().position(|session| session.dirty) {
            self.active_session = index;
            self.pending_close = Some(index);
            self.activate_tab(index)
        } else {
            #[cfg(feature = "page-surface")]
            self.recovery_finish();
            cosmic::iced::exit()
        }
    }

    fn cycle_tab(&mut self, delta: isize) -> Task<cosmic::Action<Message>> {
        if self.sessions.len() < 2 {
            return Task::none();
        }
        let count = self.sessions.len() as isize;
        let index = (self.active_session as isize + delta).rem_euclid(count) as usize;
        self.activate_tab(index)
    }

    fn activate_tab(&mut self, index: usize) -> Task<cosmic::Action<Message>> {
        if index >= self.sessions.len() {
            return Task::none();
        }
        // `update` restores the tab's scroll offset (`follow_active_session`).
        self.active_session = index;
        self.update_title()
    }

    fn request_close_tab(&mut self, index: usize) -> Task<cosmic::Action<Message>> {
        let Some(session) = self.sessions.get(index) else {
            return Task::none();
        };
        if session.dirty {
            self.pending_close = Some(index);
            Task::none()
        } else {
            self.close_tab(index)
        }
    }

    fn close_tab(&mut self, index: usize) -> Task<cosmic::Action<Message>> {
        if index >= self.sessions.len() {
            return Task::none();
        }
        self.sessions.remove(index);
        self.close_after_save = None;
        if self.sessions.is_empty() {
            let session = DocumentSession::new(
                self.next_session_id,
                new_document(),
                DocumentLocation::Untitled,
                "New document".into(),
            );
            self.next_session_id += 1;
            self.sessions.push(session);
            self.active_session = 0;
        } else if index < self.active_session {
            self.active_session -= 1;
        } else if self.active_session >= self.sessions.len() {
            self.active_session = self.sessions.len() - 1;
        }
        if self.quit_requested {
            self.continue_quit()
        } else {
            self.activate_tab(self.active_session)
        }
    }

    /// The folder a chooser with no better start opens in: the active
    /// document's folder, else the Documents preference (DF1).
    fn start_folder(&self) -> Option<PathBuf> {
        file_dialogs::start_folder(
            self.preferences.documents_folder.as_deref(),
            self.location.path(),
        )
    }

    fn open_dialog_task(&self) -> Task<cosmic::Action<Message>> {
        let start = self.start_folder();
        cosmic::task::future(async move {
            let dialog = file_dialogs::open_dialog()
                .title("Open document")
                .directory(start);
            let dialog = document_filters(dialog, true);
            match dialog.open_file().await {
                Ok(response) => response.url().to_file_path().map_or_else(
                    |_| {
                        Message::OpenDialogFinished(FileDialogResult::Failed(
                            "the selected item is not a local file".into(),
                        ))
                    },
                    |path| Message::OpenDialogFinished(FileDialogResult::Selected(path)),
                ),
                Err(file_chooser::Error::Cancelled) => {
                    Message::OpenDialogFinished(FileDialogResult::Cancelled)
                }
                Err(error) => {
                    Message::OpenDialogFinished(FileDialogResult::Failed(error.to_string()))
                }
            }
        })
    }

    /// Insert › File… (`UI_SPEC.md` §31, IM1): the desktop's Open dialog for
    /// the formats read, then Import Options.
    #[cfg(feature = "page-surface")]
    fn insert_file_task(&self) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        let folder = self.start_folder();
        cosmic::task::future(async move {
            let dialog = file_dialogs::open_dialog()
                .title("Insert File")
                .directory(folder);
            let dialog = document_filters(dialog, false);
            let result = match dialog.open_file().await {
                Ok(response) => response.url().to_file_path().map_or_else(
                    |_| FileDialogResult::Failed("the selected item is not a local file".into()),
                    FileDialogResult::Selected,
                ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::InsertFileDialogFinished { session_id, result }
        })
    }

    fn save_dialog_task(&self) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        let suggested_name = self.location.suggested_file_name(&self.document);
        let starting_directory = match &self.location {
            DocumentLocation::LegacySource(path)
            | DocumentLocation::Sam(path)
            | DocumentLocation::Native(path) => path.parent().map(PathBuf::from),
            DocumentLocation::Untitled => self.start_folder(),
        };
        cosmic::task::future(async move {
            // Both formats (decision A): the typed extension decides, then
            // the Format choice, which portals that support choices report.
            let native = FileFilter::new("AmiPro-Linux documents (.amipro)").glob("*.amipro");
            let sam = FileFilter::new("Ami Pro 3.1 documents (.SAM)")
                .glob("*.sam")
                .glob("*.SAM");
            let markdown = FileFilter::new("Markdown (.md)").glob("*.md").glob("*.MD");
            let format = file_chooser::Choice::new("format", "Format", "amipro")
                .insert("amipro", "AmiPro-Linux document (.amipro)")
                .insert("sam", "Ami Pro 3.1 document (.SAM)")
                .insert("markdown", "Markdown (.md)");
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Save document as".to_owned())
                .file_name(suggested_name)
                .filter(native)
                .filter(sam)
                .filter(markdown)
                .choice(format);
            if let Some(directory) = starting_directory {
                dialog = dialog.directory(directory);
            }
            match dialog.save_file().await {
                Ok(response) => {
                    let format = response
                        .choices()
                        .iter()
                        .find(|(id, _)| id == "format")
                        .map(|(_, value)| {
                            if value == "sam" {
                                SaveFormat::Sam
                            } else if value == "markdown" {
                                SaveFormat::Markdown
                            } else {
                                SaveFormat::Native
                            }
                        });
                    response
                        .url()
                        .and_then(|url| url.to_file_path().ok())
                        .map_or_else(
                            || Message::SaveDialogFinished {
                                session_id,
                                result: FileDialogResult::Failed(
                                    "the selected destination is not a local file".into(),
                                ),
                                format,
                            },
                            |path| Message::SaveDialogFinished {
                                session_id,
                                result: FileDialogResult::Selected(path),
                                format,
                            },
                        )
                }
                Err(file_chooser::Error::Cancelled) => Message::SaveDialogFinished {
                    session_id,
                    result: FileDialogResult::Cancelled,
                    format: None,
                },
                Err(error) => Message::SaveDialogFinished {
                    session_id,
                    result: FileDialogResult::Failed(error.to_string()),
                    format: None,
                },
            }
        })
    }

    /// The style of the selected paragraphs when they share one, for the
    /// style field and the Styles panel: on the page surface in whichever
    /// flow holds the selection (frame, header, footer, or cell text).
    fn selected_style(&self) -> Option<StyleId> {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            let (start, end) = self.page_selection.ordered();
            let paragraphs = &self.document.flow(start.flow)?.paragraphs;
            let first = paragraphs.get(start.paragraph)?.style;
            return paragraphs
                .get(start.paragraph..=end.paragraph.min(paragraphs.len() - 1))?
                .iter()
                .all(|paragraph| paragraph.style == first)
                .then_some(first);
        }
        selected_paragraph_style(&self.document, self.editor.cursor())
    }

    /// The paragraph that holds the caret (the selection's start): in any
    /// flow on the page surface, in the main text in the text editor.
    fn caret_paragraph(&self) -> Option<std::sync::Arc<amipro_document::Paragraph>> {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            let start = self.page_selection.ordered().0;
            return self
                .document
                .flow(start.flow)
                .and_then(|flow| flow.paragraphs.get(start.paragraph))
                .cloned();
        }
        let line = self.editor.cursor().position.line;
        self.document.paragraphs().get(line).cloned()
    }

    /// The style-sheet folders: `AMIPRO_STYLES`'s, then the remembered
    /// ones.
    fn style_folders(&self) -> Vec<PathBuf> {
        let mut folders: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();
        for folder in &self.preferences.style_folders {
            if !folders.contains(folder) {
                folders.push(folder.clone());
            }
        }
        folders
    }

    /// The folders Use Another Style Sheet lists: the style-sheet folders,
    /// and a `styles` folder beside the document or one level up, where
    /// Ami Pro installs its sheets.
    fn style_sheet_folders(&self) -> Vec<PathBuf> {
        let mut folders = self.style_folders();
        if let DocumentLocation::LegacySource(path)
        | DocumentLocation::Sam(path)
        | DocumentLocation::Native(path) = &self.location
            && let Some(parent) = path.parent()
        {
            for base in [Some(parent), parent.parent()].into_iter().flatten() {
                let Ok(entries) = std::fs::read_dir(base) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && entry
                            .file_name()
                            .to_string_lossy()
                            .eq_ignore_ascii_case("styles")
                        && !folders.contains(&path)
                    {
                        folders.push(path);
                    }
                }
            }
        }
        folders
    }

    /// The document's style sheet as read from its file, found as the
    /// importer finds it; `None` without a sheet or a file.
    fn document_style_sheet(&self) -> Option<amipro_document::StyleSheet> {
        let link = self.document.style_sheet()?;
        let document_path = match &self.location {
            DocumentLocation::LegacySource(path)
            | DocumentLocation::Sam(path)
            | DocumentLocation::Native(path) => Some(path.as_path()),
            DocumentLocation::Untitled => None,
        };
        let path =
            amipro_formats::find_style_sheet(document_path, &link.name, &self.style_folders())?;
        let bytes = std::fs::read(path).ok()?;
        amipro_formats::read_style_sheet(&bytes, link.name.clone()).ok()
    }

    /// Style Properties' actions: each change is one `ModifyStyle` command
    /// and one undo step; Save as New Style makes a style at once (one undo
    /// step) and shows it.
    #[cfg(feature = "page-surface")]
    fn update_style_properties(&mut self, action: style_properties::StyleAction) {
        use style_properties::Outcome;
        let session = &mut self.sessions[self.active_session];
        let Some(state) = session.style_properties.as_mut() else {
            return;
        };
        match state.update(action, session.document.styles()) {
            Outcome::Stay => {}
            Outcome::Back => session.style_properties = None,
            Outcome::Close => {
                session.style_properties = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            Outcome::Apply(style) => {
                let name = style.name.clone();
                if self.execute_document_command(&amipro_editing::Command::ModifyStyle {
                    style: *style,
                }) {
                    self.status = format!("Modified style {name}");
                }
            }
            Outcome::SaveAs {
                name,
                based_on,
                definition,
            } => {
                if self.execute_document_command(&amipro_editing::Command::CreateStyle {
                    name: name.clone(),
                    based_on,
                }) && let Some(made) = self
                    .document
                    .styles()
                    .values()
                    .find(|style| style.name == name)
                    .cloned()
                {
                    let style = ParagraphStyle {
                        id: made.id,
                        name: made.name.clone(),
                        origin: made.origin,
                        kept: made.kept.clone(),
                        ..*definition
                    };
                    let session = &mut self.sessions[self.active_session];
                    let cursor = session.history_caret();
                    if let Err(error) = session.history.amend(
                        &mut session.document,
                        cursor,
                        &amipro_editing::Command::ModifyStyle { style },
                    ) {
                        session.status = format!("Edit failed: {error}");
                    }
                    session.refresh_rich_layout();
                    if let Some(state) = session.style_properties.as_mut() {
                        state.show(session.document.styles(), made.id);
                    }
                    session.status = format!("Created style {name}");
                }
            }
        }
    }

    /// Opens Style Properties in the rail on `style`, else the caret
    /// paragraph's style, else the first listed.
    #[cfg(feature = "page-surface")]
    fn open_style_properties(&mut self, style: Option<StyleId>) {
        let caret = self.caret_paragraph().map(|paragraph| paragraph.style);
        let styles = self.document.styles();
        let Some(style) = style
            .or(caret)
            .filter(|style| styles.contains_key(style))
            .or_else(|| {
                style_dialogs::listed_styles(styles)
                    .first()
                    .map(|(id, _)| *id)
            })
        else {
            return;
        };
        let state = style_properties::StyleProperties::new(
            styles,
            style,
            caret,
            page_layout::installed_families(),
        );
        self.close_rail_panels();
        self.style_properties = state;
        self.open_rail();
    }

    /// The layout of the page holding the caret.
    #[cfg(feature = "page-surface")]
    fn caret_layout(&self) -> Option<amipro_document::LayoutId> {
        let page = self.caret_page_number() as usize;
        self.pages
            .layout()
            .pages
            .get(page.saturating_sub(1))
            .map(|page| page.layout)
    }

    /// The header or footer whose text holds the caret, if any.
    #[cfg(feature = "page-surface")]
    fn caret_band(&self) -> Option<amipro_editing::layout_setup::Band> {
        use amipro_document::{FrameContent, Placement};
        use amipro_editing::layout_setup::Band;
        let flow = self.page_selection.flow();
        self.document
            .frames()
            .find_map(|frame| match (&frame.content, frame.placement) {
                (FrameContent::Text(text), Placement::Header { .. }) if *text == flow => {
                    Some(Band::Header)
                }
                (FrameContent::Text(text), Placement::Footer { .. }) if *text == flow => {
                    Some(Band::Footer)
                }
                _ => None,
            })
    }

    /// Edit › Go To Header and Go To Footer: the caret at the start of the
    /// caret page's fixed header or footer, as Ami Pro's Header/Footer
    /// dialog puts it there (`UI_SPEC.md` §18).
    #[cfg(feature = "page-surface")]
    fn go_to_band(&mut self, band: amipro_editing::layout_setup::Band) {
        use amipro_document::{FrameContent, Placement};
        use amipro_editing::layout_setup::Band;
        let number = self.caret_page_number();
        let Some(page) = self.pages.layout().pages.get(number as usize - 1).cloned() else {
            return;
        };
        let name = match band {
            Band::Header => "header",
            Band::Footer => "footer",
        };
        // The fixed header or footer whose text is laid out on the page
        // (none on the layout's first page with Begin on second page).
        let flow = page.frames.iter().find_map(|frame_box| {
            let frame = self.document.frame(frame_box.frame)?;
            let shown = page
                .lines
                .iter()
                .any(|line| line.container == amipro_layout::Container::Frame(frame.id));
            match (band, frame.placement, &frame.content) {
                (Band::Header, Placement::Header { .. }, FrameContent::Text(flow))
                | (Band::Footer, Placement::Footer { .. }, FrameContent::Text(flow))
                    if shown =>
                {
                    Some(*flow)
                }
                _ => None,
            }
        });
        let Some(flow) = flow else {
            // Ami Pro's message 3013 when the band begins on the layout's
            // second page.
            let layout = self.document.layout(page.layout);
            let second = layout.is_some_and(|layout| match band {
                Band::Header => layout.header_from_second_page,
                Band::Footer => layout.footer_from_second_page,
            });
            self.status = if second {
                format!("This page has no {name}: it begins on the layout's second page")
            } else {
                format!("This page has no {name}")
            };
            return;
        };
        self.frame_selection = None;
        self.page_selection = PageSelection::caret(
            amipro_editing::TextPosition {
                flow,
                paragraph: 0,
                byte: 0,
            },
            number,
        );
        self.sync_editor_to_page();
        self.status = format!("Page {number} {name}");
    }

    /// Opens Page Layout Properties in the rail on the caret page's
    /// layout (`UI_SPEC.md` §18).
    #[cfg(feature = "page-surface")]
    fn open_page_layout_properties(&mut self) {
        let caret = self.caret_layout();
        let layout = caret.unwrap_or(self.document.document_layout().id);
        let state =
            page_layout_properties::PageLayoutProperties::new(&self.document, layout, caret);
        self.close_rail_panels();
        self.page_layout_properties = state;
        self.open_rail();
    }

    /// Page Layout Properties' actions: each change is one `ModifyLayout`
    /// command and one undo step.
    #[cfg(feature = "page-surface")]
    fn update_page_layout_properties(&mut self, action: page_layout_properties::LayoutAction) {
        use page_layout_properties::Outcome;
        let session = &mut self.sessions[self.active_session];
        let Some(state) = session.page_layout_properties.as_mut() else {
            return;
        };
        match state.update(action, &session.document) {
            Outcome::Stay => {}
            Outcome::Back => session.page_layout_properties = None,
            Outcome::Close => {
                session.page_layout_properties = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            Outcome::Apply(layout, settings) => {
                let name = settings.name.clone();
                if self.execute_document_command(&amipro_editing::Command::ModifyLayout {
                    layout,
                    settings: *settings,
                }) {
                    self.status = format!("Modified page layout {name}");
                }
            }
            Outcome::InsertCopy(layout) => {
                if self.insert_page_break(BreakSwitch::CopyOf(layout)) {
                    self.status = "Inserted a page layout from the next page".into();
                }
            }
            Outcome::InsertExisting(layout) => {
                if self.insert_page_break(BreakSwitch::To(layout)) {
                    self.status = "The next page goes back to an earlier page layout".into();
                }
            }
            Outcome::Remove(layout) => self.remove_inserted_layout(layout),
            Outcome::LineNumbering(numbering) => {
                let on = numbering.is_on();
                if self
                    .execute_document_command(&amipro_editing::Command::SetLineNumbering(numbering))
                {
                    self.status = if on {
                        "Changed line numbering".into()
                    } else {
                        "Line numbering is off".into()
                    };
                }
            }
        }
    }

    /// Insert › Page Break and Page Layout Properties' Insert Page Layout:
    /// a page break at the caret in the main text, without splitting the
    /// paragraph (VM fixture PB1), switching to another layout from the
    /// next page when asked (`InsertLayout` for a new copy, IL1); at the
    /// end of the main text an empty paragraph follows for the next page,
    /// as Ami Pro adds one (PB3, G21, IV1). One undo step.
    #[cfg(feature = "page-surface")]
    fn insert_page_break(&mut self, switch: BreakSwitch) -> bool {
        use amipro_editing::Command;
        let at = self.page_selection.ordered().0;
        let main = self.document.main_flow_id();
        // In a table cell: the row starts the next page, or no longer does
        // (§26, TP1).
        if at.flow != main
            && self.frame_selection.is_none()
            && matches!(switch, BreakSwitch::None)
            && let Some(command) = page_editing::row_break_command(&self.document, at.flow)
        {
            let had = page_editing::row_has_break(&self.document, at.flow);
            if !self.execute_page_command(&command) {
                return false;
            }
            self.status = if had {
                "The row no longer starts a page".into()
            } else {
                "The row starts the next page".into()
            };
            return true;
        }
        if at.flow != main || self.frame_selection.is_some() {
            self.status = "A page break goes in the main text".into();
            return false;
        }
        let at_end = self.document.flow(main).is_some_and(|flow| {
            at.paragraph + 1 == flow.paragraphs.len()
                && at.byte == flow.paragraphs[at.paragraph].text.len()
        });
        let command = match switch {
            BreakSwitch::None | BreakSwitch::To(_) => Command::InsertBreak {
                at,
                kind: amipro_document::BreakKind::Page,
                centre: false,
                layout: match switch {
                    BreakSwitch::To(layout) => Some(layout),
                    _ => None,
                },
            },
            BreakSwitch::CopyOf(from) => Command::InsertLayout { at, from },
        };
        if !self.execute_page_command(&command) {
            return false;
        }
        if at_end {
            let after = self.page_selection.ordered().0;
            self.amend_page_command(&Command::ReplaceText {
                start: after,
                end: after,
                text: "\n".into(),
            });
            let page = self.page_selection.page;
            self.page_selection = PageSelection::caret(
                amipro_editing::TextPosition {
                    flow: main,
                    paragraph: at.paragraph + 1,
                    byte: 0,
                },
                page,
            );
            self.sync_editor_to_page();
        }
        true
    }

    /// Insert › Floating Header/Footer…: inserts one at the caret, the
    /// caret then in it on the next page (Ami Pro, FH1), or removes those
    /// at the caret (FH3); one undo step.
    #[cfg(feature = "page-surface")]
    fn update_floating_dialog(&mut self, action: page_dialogs::FloatingAction) {
        use page_dialogs::FloatingOutcome;
        let Some(dialog) = self.floating_dialog.as_mut() else {
            return;
        };
        match dialog.update(action) {
            FloatingOutcome::Stay => {}
            FloatingOutcome::Cancel => self.floating_dialog = None,
            FloatingOutcome::Insert { kind, pages } => {
                self.floating_dialog = None;
                let at = self.page_selection.ordered().0;
                let page = self.caret_page_number();
                if !self.execute_page_command(
                    &amipro_editing::Command::InsertFloatingHeaderFooter { at, kind, pages },
                ) {
                    return;
                }
                let name = match kind {
                    amipro_document::HeaderFooterKind::Header => "header",
                    amipro_document::HeaderFooterKind::Footer => "footer",
                };
                // The new flow: the object just inserted owns it.
                let flow = self
                    .document
                    .flow(at.flow)
                    .and_then(|flow| flow.paragraphs.get(at.paragraph))
                    .and_then(|paragraph| paragraph.object_at(at.byte))
                    .and_then(amipro_document::InlineObject::owned_flow);
                let shown = flow.is_some_and(|flow| {
                    self.pages.layout().pages.iter().any(|candidate| {
                        candidate.number > page
                            && candidate.lines.iter().any(|line| {
                                line.container == amipro_layout::Container::Floating(flow)
                            })
                    })
                });
                match flow.filter(|_| shown) {
                    Some(flow) => {
                        self.page_selection = PageSelection::caret(
                            amipro_editing::TextPosition {
                                flow,
                                paragraph: 0,
                                byte: 0,
                            },
                            page + 1,
                        );
                        self.sync_editor_to_page();
                        self.status = format!("Floating {name}: it shows from page {}", page + 1);
                    }
                    None => {
                        self.status =
                            format!("Floating {name} inserted: it shows from the next page");
                    }
                }
            }
            FloatingOutcome::Remove(mut marks) => {
                self.floating_dialog = None;
                // From the back, so that each position still holds.
                marks
                    .sort_by_key(|position| std::cmp::Reverse((position.paragraph, position.byte)));
                let commands: Vec<_> = marks
                    .iter()
                    .map(|at| amipro_editing::Command::DeleteCharacter {
                        at: *at,
                        forward: true,
                    })
                    .collect();
                self.execute_page_commands(&commands);
                self.status = "Removed the floating header or footer".into();
            }
        }
    }

    /// Insert › Page Number…: a page-number field at the caret, or the one
    /// there changed; one undo step.
    #[cfg(feature = "page-surface")]
    fn update_page_number_dialog(&mut self, action: page_dialogs::PageNumberAction) {
        use page_dialogs::PageNumberOutcome;
        let Some(dialog) = self.page_number_dialog.as_mut() else {
            return;
        };
        let command = match dialog.update(action) {
            PageNumberOutcome::Stay => return,
            PageNumberOutcome::Cancel => {
                self.page_number_dialog = None;
                return;
            }
            PageNumberOutcome::Insert(field) => amipro_editing::Command::InsertPageNumber {
                at: self.page_selection.ordered().0,
                field,
            },
            PageNumberOutcome::Change(at, field) => {
                amipro_editing::Command::SetPageNumber { at, field }
            }
        };
        self.page_number_dialog = None;
        if self.execute_page_command(&command) {
            self.status = "Page number set".into();
        }
    }

    /// Outline Numbering…: the scheme numbers every levelled style, one
    /// `ModifyStyle` each, as one undo step.
    #[cfg(feature = "page-surface")]
    fn update_outline_numbering(&mut self, action: page_dialogs::OutlineNumberingAction) {
        use page_dialogs::OutlineNumberingOutcome;
        let Some(dialog) = self.outline_numbering.as_mut() else {
            return;
        };
        let scheme = match dialog.update(action) {
            OutlineNumberingOutcome::Stay => return,
            OutlineNumberingOutcome::Cancel => {
                self.outline_numbering = None;
                return;
            }
            OutlineNumberingOutcome::Apply(scheme) => scheme,
        };
        self.outline_numbering = None;
        let commands: Vec<amipro_editing::Command> = self
            .document
            .styles()
            .values()
            .filter_map(|style| scheme.applied(style))
            .map(|style| amipro_editing::Command::ModifyStyle { style })
            .collect();
        let count = commands.len();
        self.execute_page_commands(&commands);
        self.refresh_rich_layout();
        self.status = format!("Outline numbering: {count} styles");
    }

    /// Insert › Bullet…: the chosen bullet replaces the selection or goes
    /// in at the caret, as one undo step, and is offered next time.
    #[cfg(feature = "page-surface")]
    fn update_bullet_dialog(&mut self, action: page_dialogs::BulletAction) {
        use page_dialogs::BulletOutcome;
        let Some(dialog) = self.bullet_dialog.as_mut() else {
            return;
        };
        let code = match dialog.update(action) {
            BulletOutcome::Stay => return,
            BulletOutcome::Cancel => {
                self.bullet_dialog = None;
                return;
            }
            BulletOutcome::Insert(code) => code,
        };
        self.bullet_dialog = None;
        let (start, end) = self.page_selection.ordered();
        let mut commands = Vec::new();
        if start != end {
            commands.push(amipro_editing::Command::ReplaceText {
                start,
                end,
                text: String::new(),
            });
        }
        commands.push(amipro_editing::Command::InsertBullet { at: start, code });
        self.execute_page_commands(&commands);
        self.status = "Bullet inserted".into();
        self.preferences.last_bullet = Some(code);
        self.save_preferences();
    }

    /// Insert › Frame…'s buttons: Create makes the frame, Draw on Page
    /// starts drawing one (`UI_SPEC.md` §20).
    #[cfg(feature = "page-surface")]
    fn update_create_frame_dialog(&mut self, action: page_dialogs::CreateFrameAction) {
        use page_dialogs::CreateFrameOutcome;
        let Some(dialog) = self.create_frame_dialog.as_mut() else {
            return;
        };
        match dialog.update(action) {
            CreateFrameOutcome::Stay => {}
            CreateFrameOutcome::Cancel => self.create_frame_dialog = None,
            CreateFrameOutcome::Create { page, rect } => {
                self.create_frame_dialog = None;
                self.create_frame(page, rect);
            }
            CreateFrameOutcome::Draw => {
                self.create_frame_dialog = None;
                self.start_drawing_frame();
            }
        }
    }

    /// Makes Ami Pro's new frame over `rect` on page `page` (`1260:01cf`):
    /// an empty text frame, Where placed, with the default frame's
    /// settings, on top of the page's Where placed frames, as one undo
    /// step; then selects it, leaving the caret where it was, and remembers
    /// its size and position for the dialog (`UI_SPEC.md` §20).
    #[cfg(feature = "page-surface")]
    fn create_frame(&mut self, page: u32, rect: amipro_document::Rect) {
        let style = self.preferences.frame_defaults.frame_style(rect);
        let command = amipro_editing::Command::CreateFrame {
            placement: amipro_document::Placement::Fixed { page, rect },
            style,
            content: amipro_editing::NewFrameContent::Text,
            anchor: None,
            layout: None,
        };
        if !self.execute_page_command(&command) {
            return;
        }
        let Some(frame) = self.document.frame_stack().last().copied() else {
            return;
        };
        let session = &mut self.sessions[self.active_session];
        session.frame_selection = Some(FrameSelection::one(frame, page));
        session.frame_keys = None;
        session.status =
            "Frame created. Enter or a double-click types in it; Esc returns to the text".into();
        self.preferences.last_frame = Some(
            amipro_editing::frame_creation::FrameGeometry::remembered(rect),
        );
        self.save_preferences();
    }

    /// Insert › Image…: the portal's file chooser, filtered to the formats
    /// it takes (P1), with Link to file (P2).
    #[cfg(feature = "page-surface")]
    fn choose_image_task(&mut self) -> Task<cosmic::Action<Message>> {
        if let Err(problem) = picture_insert::target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            self.status = problem.into();
            return Task::none();
        }
        let session_id = self.id;
        let start = self.start_folder();
        cosmic::task::future(async move {
            let mut filter = FileFilter::new("Pictures");
            for extension in picture_insert::FORMATS {
                filter = filter
                    .glob(&format!("*.{extension}"))
                    .glob(&format!("*.{}", extension.to_ascii_uppercase()));
            }
            let dialog = file_dialogs::open_dialog()
                .title("Insert Image".to_owned())
                .directory(start)
                .filter(filter)
                .choice(file_chooser::Choice::boolean("link", "Link to file", false));
            let (result, link) = match dialog.open_file().await {
                Ok(response) => {
                    let link = response
                        .choices()
                        .iter()
                        .any(|(id, value)| id == "link" && value == "true");
                    let result = response.url().to_file_path().map_or_else(
                        |()| FileDialogResult::Failed("the picture is not a local file".into()),
                        FileDialogResult::Selected,
                    );
                    (result, link)
                }
                Err(file_chooser::Error::Cancelled) => (FileDialogResult::Cancelled, false),
                Err(error) => (FileDialogResult::Failed(error.to_string()), false),
            };
            Message::ImageChosen {
                session_id,
                result,
                link,
            }
        })
    }

    /// Puts the picture in `path` where Insert › Image… puts it
    /// (`picture_insert::target`), copied or linked, as one undo step; the
    /// new or filled frame is selected.
    #[cfg(feature = "page-surface")]
    fn insert_image_file(&mut self, path: &std::path::Path, link: bool) {
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{}", extension.to_ascii_lowercase()))
            .unwrap_or_default();
        if !picture_insert::takes(&extension) {
            self.status = "Insert Image takes BMP, WMF, PNG, JPEG, GIF, TIFF, PCX, and SVG".into();
            return;
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.status = format!("Insert Image could not read the file: {error}");
                return;
            }
        };
        let link = link.then(|| path.to_string_lossy().into_owned());
        self.insert_picture_bytes(bytes, &extension, link);
    }

    /// Puts a picture of `bytes` (a file of `extension`) where Insert ›
    /// Image… puts one, linked to `link` when given (P2), else copied: from
    /// the chooser, the clipboard, or a drop (P7).
    #[cfg(feature = "page-surface")]
    fn insert_picture_bytes(&mut self, bytes: Vec<u8>, extension: &str, link: Option<String>) {
        use amipro_editing::{
            Command, NewFrameContent, NewPicture, NewPictureSource, PictureTarget,
        };
        use picture_insert::Target;
        let extension = extension.to_owned();
        let format = amipro_document::PictureFormat::from_extension(&extension);
        let Some(natural) = pictures::natural_of(format, &bytes) else {
            self.status = "Insert Image could not read this picture".into();
            return;
        };
        let target = match picture_insert::target(
            &self.document,
            self.frame_selection.as_ref(),
            self.page_selection.focus,
        ) {
            Ok(target) => target,
            Err(problem) => {
                self.status = problem.into();
                return;
            }
        };
        let linked = link.is_some();
        let source = if let Some(path) = link {
            NewPictureSource::Linked(path)
        } else {
            NewPictureSource::Embedded(std::sync::Arc::from(bytes))
        };
        let picture = |size| NewPicture {
            source: source.clone(),
            extension: extension.clone(),
            size,
        };
        let page = self.page_selection.page;
        let (command, select) = match target {
            Target::Replace(frame) => (
                Command::ReplacePicture {
                    frame,
                    picture: picture(natural),
                },
                Some(frame),
            ),
            Target::Frame(frame) => (
                Command::InsertPicture {
                    target: PictureTarget::Frame(frame),
                    picture: picture(natural),
                    cell_style: self.preferences.frame_defaults.cell_frame_style(),
                },
                Some(frame),
            ),
            Target::Cell { table, cell } => (
                Command::InsertPicture {
                    target: PictureTarget::Cell { table, cell },
                    picture: picture(natural),
                    cell_style: self.preferences.frame_defaults.cell_frame_style(),
                },
                None,
            ),
            Target::NewFrame(at) => {
                let column = self.caret_column();
                let (frame, area) = picture_insert::new_frame_size(
                    natural,
                    &self.preferences.frame_defaults,
                    column,
                );
                let style = self
                    .preferences
                    .frame_defaults
                    .frame_style(amipro_document::Rect::new(0, 0, frame.width, frame.height));
                let content = match &source {
                    NewPictureSource::Embedded(bytes) => NewFrameContent::EmbeddedPicture {
                        bytes: bytes.clone(),
                        extension: extension.clone(),
                    },
                    NewPictureSource::Linked(path) => NewFrameContent::LinkedPicture {
                        path: path.clone(),
                        extension: extension.clone(),
                    },
                };
                let _ = area; // The picture fills the frame's area (Fit in frame).
                (
                    Command::CreateFrame {
                        placement: amipro_document::Placement::Inline {
                            y_offset: 0,
                            size: frame,
                        },
                        style: amipro_document::FrameStyle {
                            wrap: amipro_document::Wrap::Around,
                            ..style
                        },
                        content,
                        anchor: Some(at),
                        layout: None,
                    },
                    None,
                )
            }
        };
        let before: std::collections::BTreeSet<_> =
            self.document.frames().map(|frame| frame.id).collect();
        if !self.execute_page_command(&command) {
            return;
        }
        let made = self
            .document
            .frames()
            .map(|frame| frame.id)
            .find(|id| !before.contains(id));
        let selected = select.or(match target {
            Target::NewFrame(_) => made,
            _ => None,
        });
        let session = &mut self.sessions[self.active_session];
        if let Some(frame) = selected {
            session.frame_selection = Some(FrameSelection::one(frame, page));
            session.frame_keys = None;
        }
        session.status = match target {
            Target::Replace(_) => "Picture replaced".into(),
            Target::Cell { .. } => "Picture put in the cell".into(),
            _ if linked => "Picture linked to its file".into(),
            _ => "Picture inserted".into(),
        };
    }

    /// The selected frame's picture and its bytes (a linked picture's file
    /// as it is now), for Save Picture As….
    #[cfg(feature = "page-surface")]
    fn selected_picture_bytes(&self) -> Option<(amipro_document::Picture, Vec<u8>)> {
        let (frame, _) = self.inspected_frame()?;
        let amipro_document::FrameContent::Picture(picture) = &self.document.frame(frame)?.content
        else {
            return None;
        };
        let bytes = match &picture.source {
            amipro_document::PictureSource::Embedded(object) => {
                self.document.objects().get(object)?.bytes.to_vec()
            }
            amipro_document::PictureSource::Linked(path) => {
                let (file, _) = pictures::linked_file(path, self.location.folder())?;
                std::fs::read(file).ok()?
            }
        };
        Some((picture.clone(), bytes))
    }

    /// Save Picture As… (P7): the portal's save dialog for the selected
    /// picture, named for its frame and kind.
    /// The document made ready to print with its Print Options, and the
    /// PDF's options; `None` when Update fields first stopped at a field's
    /// question, which must be answered before printing.
    #[cfg(feature = "page-surface")]
    fn prepared_print(&mut self, paper: Option<(f32, f32)>) -> Option<print::PdfOptions> {
        let options = self.print_options;
        if options.update_fields {
            self.run_field_update(
                amipro_editing::fields::UpdateScope::All,
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
            );
            if self.field_prompt.is_some() {
                self.status = "Answer the field's question, then print again".into();
                return None;
            }
        }
        let file = match &self.location {
            DocumentLocation::Untitled => None,
            DocumentLocation::LegacySource(path)
            | DocumentLocation::Sam(path)
            | DocumentLocation::Native(path) => Some(path.as_path()),
        };
        let mut options = options.pdf_options(&self.document, file, paper);
        options.additions = self.preferences.pdf_additions;
        Some(options)
    }

    /// File › Print Options… (`UI_SPEC.md` §29): the Print panel in the
    /// rail; the pages show as they print while it is open.
    #[cfg(feature = "page-surface")]
    fn open_print_panel(&mut self) {
        // The pages show as they print: Draft and Outline give way to
        // Layout mode (their display pages are not the printed ones).
        if self.draft.is_some() || self.outline.is_some() {
            let session = &mut self.sessions[self.active_session];
            session.draft = None;
            session.outline = None;
            session.refresh_rich_layout();
        }
        self.close_rail_panels();
        self.print_panel = Some(print_panel::PrintPanel::default());
        self.open_rail();
    }

    /// The Print panel's switches: Ami Pro's options for the document, the
    /// PDF's additions in the settings.
    #[cfg(feature = "page-surface")]
    fn update_print_panel(&mut self, action: print_panel::PrintAction) {
        use print_panel::PrintAction;
        match action {
            PrintAction::Section(section) => {
                if let Some(panel) = self.print_panel.as_mut() {
                    panel.toggle(section);
                }
            }
            PrintAction::Option(option, on) => option.set(&mut self.print_options, on),
            PrintAction::Addition(addition, on) => {
                addition.set(&mut self.preferences.pdf_additions, on);
            }
            PrintAction::WholeSet(whole) => self.print_this_only = !whole,
            PrintAction::Back => self.print_panel = None,
            PrintAction::Close => {
                self.print_panel = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
        }
    }

    /// File › Export PDF… (`UI_SPEC.md` §29): the desktop's save dialog,
    /// the document's name with `.pdf`, in its folder.
    #[cfg(feature = "page-surface")]
    fn export_pdf_task(&mut self) -> Task<cosmic::Action<Message>> {
        let name = print::pdf_name(&self.location, &self.document.title);
        let folder = self.start_folder();
        let session_id = self.id;
        cosmic::task::future(async move {
            let filter = FileFilter::new("PDF documents (.pdf)")
                .glob("*.pdf")
                .glob("*.PDF");
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Export PDF".to_owned())
                .file_name(name)
                .filter(filter);
            if let Some(folder) = folder {
                dialog = dialog.directory(folder);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || FileDialogResult::Failed("the destination is not a local file".into()),
                        FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::ExportPdfChosen { session_id, result }
        })
    }

    /// Writes the document's PDF to `path` off the interface's thread: the
    /// document and its pages are shared roots, so the copies are cheap and
    /// editing goes on meanwhile.
    #[cfg(feature = "page-surface")]
    fn export_pdf(&mut self, path: PathBuf) -> Task<cosmic::Action<Message>> {
        let Some(options) = self.prepared_print(None) else {
            return Task::none();
        };
        let job = print::ExportJob {
            job: self.pdf_job(options),
            path,
        };
        self.status = "Exporting PDF…".into();
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = print::in_background(move || job.run()).await;
            Message::PdfExported { session_id, result }
        })
    }

    /// File › Export ▸ (`UI_SPEC.md` §30, EX2): the desktop's save dialog
    /// for `format`, the document's name with its extension, in the folder
    /// the last export went to, else beside the document (EX10).
    #[cfg(feature = "page-surface")]
    fn export_task(&mut self, format: amipro_exchange::Format) -> Task<cosmic::Action<Message>> {
        let name = export::file_name(&self.location, &self.document.title, format);
        let folder = self
            .preferences
            .export_folder
            .clone()
            .filter(|folder| folder.is_dir())
            .or_else(|| self.start_folder());
        let session_id = self.id;
        cosmic::task::future(async move {
            let extension = format.extension();
            let lower = format!("*.{extension}");
            let upper = lower.to_uppercase();
            let filter = FileFilter::new(&export::filter_name(format))
                .glob(&lower)
                .glob(&upper);
            let mut dialog = file_chooser::save::Dialog::new()
                .title(format!("Export {}", format.name()))
                .file_name(name)
                .filter(filter);
            if let Some(folder) = folder {
                dialog = dialog.directory(folder);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || FileDialogResult::Failed("the destination is not a local file".into()),
                        FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::ExportChosen {
                session_id,
                format,
                result,
            }
        })
    }

    /// Makes the export of the document in `format` for `path` off the
    /// interface's thread; what the format cannot hold comes back with it.
    #[cfg(feature = "page-surface")]
    fn export(
        &mut self,
        path: PathBuf,
        format: amipro_exchange::Format,
    ) -> Task<cosmic::Action<Message>> {
        let path = if export::has_extension(&path, format) {
            path
        } else {
            let mut named = path.into_os_string();
            named.push(".");
            named.push(format.extension());
            PathBuf::from(named)
        };
        if let Some(folder) = path.parent() {
            self.preferences.export_folder = Some(folder.to_path_buf());
            self.save_preferences();
        }
        let job = export::ExportJob {
            document: self.document.clone(),
            layout: self.pages.layout().clone(),
            folder: self.location.folder().map(std::path::Path::to_path_buf),
            fields: self.field_context(),
            title: export::title(&self.location, &self.document.title),
            format,
            path,
            set: self.print_set(),
        };
        self.status = if job.set.is_empty() {
            format!("Exporting {}…", format.name())
        } else {
            format!(
                "Exporting the master document's {} files as {}…",
                job.set.len(),
                format.name()
            )
        };
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = print::in_background(move || job.run()).await;
            Message::ExportPrepared { session_id, result }
        })
    }

    /// File › Export ▸ Folder… (EX6): the panel in the rail, with the last
    /// folders and format (EX10).
    #[cfg(feature = "page-surface")]
    fn open_export_folder(&mut self) {
        let kept = self.export_folder.take();
        self.close_rail_panels();
        self.export_folder = kept;
        if self.export_folder.is_none() {
            let format = amipro_exchange::Format::ALL
                .into_iter()
                .find(|format| format.extension() == self.preferences.export_format)
                .unwrap_or(amipro_exchange::Format::Docx);
            self.export_folder = Some(export_folder::ExportFolder::new(
                self.preferences.export_from.clone(),
                self.preferences.export_folder.clone(),
                format,
            ));
        }
        self.open_rail();
    }

    #[cfg(feature = "page-surface")]
    fn update_export_folder(
        &mut self,
        action: export_folder::FolderAction,
    ) -> Task<cosmic::Action<Message>> {
        use export_folder::FolderAction;
        match action {
            FolderAction::ChooseFrom => {
                return choose_folder("Export Folder: From", self.start_folder(), false);
            }
            FolderAction::ChooseTo => {
                return choose_folder("Export Folder: To", self.start_folder(), true);
            }
            FolderAction::Chosen { to, folder } => {
                let Some(folder) = folder else {
                    return Task::none();
                };
                if let Some(panel) = self.export_folder.as_mut() {
                    if to {
                        panel.to = Some(folder.clone());
                    } else {
                        panel.from = Some(folder.clone());
                    }
                }
                if to {
                    self.preferences.export_folder = Some(folder);
                } else {
                    self.preferences.export_from = Some(folder);
                }
                self.save_preferences();
            }
            FolderAction::Subfolders(on) => {
                if let Some(panel) = self.export_folder.as_mut() {
                    panel.subfolders = on;
                }
            }
            FolderAction::Format(format) => {
                if let Some(panel) = self.export_folder.as_mut() {
                    panel.format = format;
                    self.preferences.export_format = format.extension().to_owned();
                    self.save_preferences();
                }
            }
            FolderAction::Export => return self.start_export_folder(),
            FolderAction::Stop => {
                if let Some(running) = self
                    .export_folder
                    .as_ref()
                    .and_then(|panel| panel.running.as_ref())
                {
                    running
                        .stop
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            FolderAction::OpenReport => {
                if let Some(report) = self
                    .export_folder
                    .as_ref()
                    .and_then(|panel| panel.finished.as_ref())
                    .and_then(|finished| finished.report.clone())
                {
                    return cosmic::task::future(async move {
                        Message::ExportedOpened(export::open_file(report).await)
                    });
                }
            }
            FolderAction::Back => self.export_folder = None,
            FolderAction::Close => {
                self.export_folder = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
        }
        Task::none()
    }

    /// Starts the folder's export on a thread of its own; its reports come
    /// back as messages.
    #[cfg(feature = "page-surface")]
    fn start_export_folder(&mut self) -> Task<cosmic::Action<Message>> {
        let styles = self.style_sheet_folders();
        let Some(panel) = self.export_folder.as_mut() else {
            return Task::none();
        };
        let (Some(from), Some(to)) = (panel.from.clone(), panel.to.clone()) else {
            return Task::none();
        };
        if panel.running.is_some() {
            return Task::none();
        }
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        panel.running = Some(export_folder::Running {
            done: 0,
            total: 0,
            stop: stop.clone(),
        });
        panel.finished = None;
        let job = export_folder::FolderJob {
            from,
            subfolders: panel.subfolders,
            to,
            format: panel.format,
            styles,
            stop,
        };
        self.status = format!("Exporting a folder as {}…", job.format.name());
        let session_id = self.id;
        let (sender, receiver) = cosmic::iced::futures::channel::mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("export folder".into())
            .spawn(move || {
                let report = |progress| {
                    let _ = sender.unbounded_send(progress);
                };
                let result = job.run(&report);
                report(export_folder::Progress::Finished(result));
            });
        if let Err(error) = spawned {
            if let Some(panel) = self.export_folder.as_mut() {
                panel.running = None;
            }
            self.status = format!("Export Folder failed: {error}");
            return Task::none();
        }
        cosmic::task::stream(cosmic::iced::futures::StreamExt::map(
            receiver,
            move |progress| Message::ExportFolderProgress {
                session_id,
                progress,
            },
        ))
    }

    /// File › Import Folder… (IM11): the panel in the rail, with the last
    /// folders and style option.
    #[cfg(feature = "page-surface")]
    fn open_import_folder(&mut self) {
        let kept = self.import_folder.take();
        self.close_rail_panels();
        self.import_folder = kept;
        if self.import_folder.is_none() {
            let styles = import::options_from(&self.preferences.import_styles, "", false).styles;
            self.import_folder = Some(import_folder::ImportFolder::new(
                self.preferences.import_from.clone(),
                self.preferences.import_to.clone(),
                styles,
            ));
        }
        self.open_rail();
    }

    #[cfg(feature = "page-surface")]
    fn update_import_folder(
        &mut self,
        action: import_folder::FolderAction,
    ) -> Task<cosmic::Action<Message>> {
        use import_folder::FolderAction;
        let chosen = |to| move |folder| Message::ImportFolder(FolderAction::Chosen { to, folder });
        match action {
            FolderAction::ChooseFrom => {
                return chosen_folder_in("Import Folder: From", self.start_folder(), chosen(false));
            }
            FolderAction::ChooseTo => {
                return chosen_folder_in("Import Folder: To", self.start_folder(), chosen(true));
            }
            FolderAction::Chosen { to, folder } => {
                let Some(folder) = folder else {
                    return Task::none();
                };
                if let Some(panel) = self.import_folder.as_mut() {
                    if to {
                        panel.to = Some(folder.clone());
                    } else {
                        panel.from = Some(folder.clone());
                    }
                }
                if to {
                    self.preferences.import_to = Some(folder);
                } else {
                    self.preferences.import_from = Some(folder);
                }
                self.save_preferences();
            }
            FolderAction::Subfolders(on) => {
                if let Some(panel) = self.import_folder.as_mut() {
                    panel.subfolders = on;
                }
            }
            FolderAction::Styles(styles) => {
                if let Some(panel) = self.import_folder.as_mut() {
                    panel.styles = styles;
                }
            }
            FolderAction::Import => return self.start_import_folder(),
            FolderAction::Stop => {
                if let Some(running) = self
                    .import_folder
                    .as_ref()
                    .and_then(|panel| panel.running.as_ref())
                {
                    running
                        .stop
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
            FolderAction::OpenReport => {
                if let Some(report) = self
                    .import_folder
                    .as_ref()
                    .and_then(|panel| panel.finished.as_ref())
                    .and_then(|finished| finished.report.clone())
                {
                    return cosmic::task::future(async move {
                        Message::ExportedOpened(export::open_file(report).await)
                    });
                }
            }
            FolderAction::Back => self.import_folder = None,
            FolderAction::Close => {
                self.import_folder = None;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
        }
        Task::none()
    }

    /// Starts the folder's import on a thread of its own; its reports come
    /// back as messages.
    #[cfg(feature = "page-surface")]
    fn start_import_folder(&mut self) -> Task<cosmic::Action<Message>> {
        let Some(panel) = self.import_folder.as_mut() else {
            return Task::none();
        };
        let (Some(from), Some(to)) = (panel.from.clone(), panel.to.clone()) else {
            return Task::none();
        };
        if panel.running.is_some() {
            return Task::none();
        }
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        panel.running = Some(import_folder::Running {
            done: 0,
            total: 0,
            stop: stop.clone(),
        });
        panel.finished = None;
        let job = import_folder::FolderJob {
            from,
            subfolders: panel.subfolders,
            to,
            options: amipro_exchange::import::ImportOptions {
                styles: panel.styles,
                ..amipro_exchange::import::ImportOptions::default()
            },
            stop,
        };
        self.status = "Importing a folder…".into();
        let session_id = self.id;
        let (sender, receiver) = cosmic::iced::futures::channel::mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("import folder".into())
            .spawn(move || {
                let report = |progress| {
                    let _ = sender.unbounded_send(progress);
                };
                let result = job.run(&report);
                report(import_folder::Progress::Finished(result));
            });
        if let Err(error) = spawned {
            if let Some(panel) = self.import_folder.as_mut() {
                panel.running = None;
            }
            self.status = format!("Import Folder failed: {error}");
            return Task::none();
        }
        cosmic::task::stream(cosmic::iced::futures::StreamExt::map(
            receiver,
            move |progress| Message::ImportFolderProgress {
                session_id,
                progress,
            },
        ))
    }

    /// The document's PDF as a job for another thread.
    #[cfg(feature = "page-surface")]
    fn pdf_job(&self, options: print::PdfOptions) -> print::PdfJob {
        print::PdfJob {
            document: self.document.clone(),
            layout: self.output_layout(),
            folder: self.location.folder().map(std::path::Path::to_path_buf),
            set: self.print_set(),
            options,
            labels: None,
        }
    }

    /// The print job's name: the document's file name without its
    /// extension, else its title (Ami Pro names it "Ami Pro - FILE.SAM").
    #[cfg(feature = "page-surface")]
    fn print_title(&self) -> String {
        print::pdf_name(&self.location, &self.document.title)
            .trim_end_matches(".pdf")
            .to_owned()
    }

    /// File › Print… (Ctrl+P), and Print Current Page… with `only_current`:
    /// the desktop's print dialog, set to the document's paper.
    #[cfg(feature = "page-surface")]
    fn print_task(&mut self, only_current: bool) -> Task<cosmic::Action<Message>> {
        let layout = self.pages.layout();
        let first = layout
            .pages
            .first()
            .map_or((12_240.0, 15_840.0), |page| (page.width, page.height));
        let count = layout.pages.len();
        let current = (self.page_selection.page as usize).saturating_sub(1);
        let title = self.print_title();
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = print_portal::prepare(title, first, count, current, only_current).await;
            Message::PrintPrepared { session_id, result }
        })
    }

    /// Prints the pages the dialog chose: the PDF drawn in the background,
    /// then sent to the printer it chose.
    #[cfg(feature = "page-surface")]
    fn print_prepared(
        &mut self,
        prepared: print_portal::Prepared,
    ) -> Task<cosmic::Action<Message>> {
        let Some(mut options) = self.prepared_print(prepared.paper) else {
            return Task::none();
        };
        if prepared.pages.is_empty() {
            self.status = "No pages to print".into();
            return Task::none();
        }
        let pages = prepared.pages.len();
        options.pages = Some(prepared.pages);
        let job = self.pdf_job(options);
        let title = self.print_title();
        let token = prepared.token;
        self.status = if pages == 1 {
            "Printing 1 page…".into()
        } else {
            format!("Printing {pages} pages…")
        };
        let session_id = self.id;
        cosmic::task::future(async move {
            let result = match print::in_background(move || job.render()).await {
                Ok(pdf) => print_portal::send(title, pdf, token).await.map(|()| pages),
                Err(error) => Err(error),
            };
            Message::Printed { session_id, result }
        })
    }

    #[cfg(feature = "page-surface")]
    fn save_picture_task(&mut self) -> Task<cosmic::Action<Message>> {
        let Some((picture, _)) = self.selected_picture_bytes() else {
            self.status = "This picture's file cannot be found".into();
            return Task::none();
        };
        let extension = picture
            .extension
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let name = format!("Picture.{extension}");
        let session_id = self.id;
        let start = self.start_folder();
        cosmic::task::future(async move {
            let filter = FileFilter::new(picture.format().name())
                .glob(&format!("*.{extension}"))
                .glob(&format!("*.{}", extension.to_ascii_uppercase()));
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Save Picture As".to_owned())
                .file_name(name)
                .filter(filter);
            if let Some(start) = start {
                dialog = dialog.directory(start);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || FileDialogResult::Failed("the destination is not a local file".into()),
                        FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::SavePictureChosen { session_id, result }
        })
    }

    /// Writes the selected picture's bytes to `path` as they are.
    #[cfg(feature = "page-surface")]
    fn save_picture_file(&mut self, path: &std::path::Path) {
        let Some((_, bytes)) = self.selected_picture_bytes() else {
            self.status = "This picture's file cannot be found".into();
            return;
        };
        self.status = match std::fs::write(path, bytes) {
            Ok(()) => format!("Picture saved as {}", path.display()),
            Err(error) => format!("Save Picture As failed: {error}"),
        };
    }

    /// The width and height of the column the caret is in, twips: where a
    /// new picture's frame must fit (P3).
    #[cfg(feature = "page-surface")]
    fn caret_column(&self) -> amipro_document::Size {
        let caret = self.page_selection.focus;
        let layout = self.pages.layout();
        let column = layout.pages.iter().find_map(|page| {
            let line = page.lines.iter().find(|line| {
                line.flow == caret.flow
                    && line.paragraph_index == caret.paragraph
                    && line.bytes.start <= caret.byte
                    && caret.byte <= line.bytes.end
            })?;
            page.columns
                .iter()
                .find(|column| column.x <= line.x + 1.0 && line.x < column.right())
                .or(page.columns.first())
                .copied()
        });
        column.map_or(
            amipro_document::Size {
                width: 9_360,
                height: 12_960,
            },
            |column| amipro_document::Size {
                width: column.width as i32,
                height: column.height as i32,
            },
        )
    }

    /// Insert › Draw Frame, or the dialog's Draw on Page: until the next
    /// press on a page, which draws the frame, or Esc.
    #[cfg(feature = "page-surface")]
    fn start_drawing_frame(&mut self) {
        let session = &mut self.sessions[self.active_session];
        session.drawing_frame = true;
        session.status = "Drag on a page to draw a frame. Esc cancels".into();
    }

    /// Insert › Column Break: Ami Pro puts the mark at the start of the
    /// caret's paragraph, and the column ends after the paragraph (VM
    /// fixtures CB1, V22). One per paragraph.
    #[cfg(feature = "page-surface")]
    fn insert_column_break(&mut self) {
        use amipro_document::{BreakKind, InlineObject};
        let at = self.page_selection.ordered().0;
        let main = self.document.main_flow_id();
        if at.flow != main || self.frame_selection.is_some() {
            self.status = "A column break goes in the main text".into();
            return;
        }
        let has_one = self
            .document
            .flow(main)
            .and_then(|flow| flow.paragraphs.get(at.paragraph))
            .is_some_and(|paragraph| {
                paragraph.objects.iter().any(|object| {
                    matches!(
                        object,
                        InlineObject::Break {
                            kind: BreakKind::Column,
                            ..
                        }
                    )
                })
            });
        if has_one {
            self.status = "This paragraph already ends its column".into();
            return;
        }
        let start = amipro_editing::TextPosition { byte: 0, ..at };
        let page = self.page_selection.page;
        if self.execute_page_command(&amipro_editing::Command::InsertBreak {
            at: start,
            kind: BreakKind::Column,
            centre: false,
            layout: None,
        }) {
            // The caret stays before the same character.
            let shifted = amipro_editing::TextPosition {
                byte: at.byte + amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
                ..at
            };
            self.page_selection = PageSelection::caret(shifted, page);
            self.sync_editor_to_page();
            self.status = "The column ends after this paragraph".into();
        }
    }

    /// Page Layout Properties' Remove: the page break that switches to an
    /// inserted layout (the last before the caret, else the first) and,
    /// when no other break names it, the layout, in one undo step, the
    /// caret where the break was (VM fixture IR1).
    #[cfg(feature = "page-surface")]
    fn remove_inserted_layout(&mut self, layout: amipro_document::LayoutId) {
        use amipro_document::InlineObject;
        use amipro_editing::{Command, TextPosition};
        if layout == self.document.document_layout().id {
            self.status = "The document's own page layout stays".into();
            return;
        }
        let main = self.document.main_flow_id();
        let Some(flow) = self.document.flow(main) else {
            return;
        };
        let breaks: Vec<TextPosition> = flow
            .paragraphs
            .iter()
            .enumerate()
            .flat_map(|(index, paragraph)| {
                paragraph
                    .object_offsets()
                    .zip(&paragraph.objects)
                    .filter(|(_, object)| {
                        matches!(object, InlineObject::Break { layout: Some(named), .. } if *named == layout)
                    })
                    .map(move |(byte, _)| TextPosition {
                        flow: main,
                        paragraph: index,
                        byte,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let caret = self.page_selection.ordered().0;
        let key = |position: &TextPosition| (position.paragraph, position.byte);
        let chosen = breaks
            .iter()
            .rev()
            .find(|position| caret.flow != main || key(position) < key(&caret))
            .or(breaks.first())
            .copied();
        let mut commands = Vec::new();
        if let Some(at) = chosen {
            commands.push(Command::DeleteCharacter { at, forward: true });
        }
        if breaks.len() <= 1 {
            commands.push(Command::RemoveLayout { layout });
        }
        let page = self.page_selection.page;
        self.execute_page_commands(&commands);
        if let Some(at) = chosen {
            self.page_selection = PageSelection::caret(at, page);
            self.sync_editor_to_page();
        }
        self.status = "Removed the inserted page layout".into();
    }

    /// Keeps Page Layout Properties on the caret page's layout and the
    /// document's settings after every message; it goes with the rail.
    #[cfg(feature = "page-surface")]
    fn sync_page_layout_properties(&mut self) {
        if self.page_layout_properties.is_none() {
            return;
        }
        if !self.rail_open() {
            self.page_layout_properties = None;
            return;
        }
        let (caret, band) = if self.frame_selection.is_some() {
            (None, None)
        } else {
            (self.caret_layout(), self.caret_band())
        };
        let session = &mut self.sessions[self.active_session];
        if let Some(state) = session.page_layout_properties.as_mut()
            && !state.sync(&session.document, caret, band)
        {
            session.page_layout_properties = None;
        }
    }

    /// Keeps Style Properties on the caret's style and the document's
    /// definitions after every message; it goes with the rail.
    #[cfg(feature = "page-surface")]
    fn sync_style_properties(&mut self) {
        if self.style_properties.is_none() {
            return;
        }
        if !self.rail_open() {
            self.style_properties = None;
            return;
        }
        let caret = if self.frame_selection.is_some() {
            None
        } else {
            self.caret_paragraph().map(|paragraph| paragraph.style)
        };
        let session = &mut self.sessions[self.active_session];
        if let Some(state) = session.style_properties.as_mut()
            && !state.sync(session.document.styles(), caret)
        {
            session.style_properties = None;
        }
    }

    /// A Styles panel row's context menu.
    fn style_row(
        &mut self,
        style: StyleId,
        action: StyleRowAction,
    ) -> Task<cosmic::Action<Message>> {
        let Some(definition) = self.document.styles().get(&style).cloned() else {
            return Task::none();
        };
        match action {
            StyleRowAction::Apply => {
                return cosmic::Application::update(self, Message::ApplyStyle(style));
            }
            StyleRowAction::Modify => {
                #[cfg(feature = "page-surface")]
                self.open_style_properties(Some(style));
            }
            StyleRowAction::CreateBasedOn => {
                let mut state =
                    style_dialogs::CreateStyle::new(self.document.styles(), Some(style));
                state.selected_text = false;
                self.create_style = Some(state);
            }
            StyleRowAction::Rename => self.rename_style = Some((style, definition.name)),
            StyleRowAction::Remove => {
                let sheet = self.document_style_sheet();
                self.style_management = Some(style_dialogs::StyleManagement {
                    selected: Some(style),
                    sheet,
                });
                self.update_style_management(style_dialogs::ManagementAction::Remove);
                self.style_management = None;
            }
        }
        Task::none()
    }

    /// Style Management's actions, each one undo step (VM fixture ST1).
    fn update_style_management(&mut self, action: style_dialogs::ManagementAction) {
        use style_dialogs::ManagementAction;
        let Some(state) = self.style_management.clone() else {
            return;
        };
        if let ManagementAction::Select(id) = action {
            if let Some(state) = self.style_management.as_mut() {
                state.selected = Some(id);
            }
            return;
        }
        if action == ManagementAction::Close {
            self.style_management = None;
            return;
        }
        let Some(style) = state
            .selected
            .and_then(|id| self.document.styles().get(&id))
            .cloned()
        else {
            return;
        };
        // The sheet's version, its next style named in the document's IDs.
        let sheet_version = state.sheet_version(&style).map(|version| {
            let next_name = version.next.and_then(|next| {
                state
                    .sheet
                    .as_ref()?
                    .styles
                    .iter()
                    .find(|candidate| candidate.id == next)
                    .map(|candidate| candidate.name.to_lowercase())
            });
            let next = next_name.and_then(|name| {
                self.document
                    .styles()
                    .values()
                    .find(|candidate| candidate.name.to_lowercase() == name)
                    .map(|candidate| candidate.id)
            });
            ParagraphStyle {
                next,
                ..version.clone()
            }
        });
        let command = match action {
            ManagementAction::MoveToDocument => amipro_editing::Command::ModifyStyle {
                style: style.clone(),
            },
            ManagementAction::Revert => match sheet_version {
                Some(version) => amipro_editing::Command::DeleteStyle {
                    style: style.id,
                    sheet_version: Some(version),
                },
                None => return,
            },
            ManagementAction::Remove => amipro_editing::Command::DeleteStyle {
                style: style.id,
                sheet_version,
            },
            ManagementAction::Select(_) | ManagementAction::Close => return,
        };
        if self.execute_document_command(&command) {
            self.status = format!(
                "{}: {}",
                style.name,
                match action {
                    ManagementAction::MoveToDocument => "now stored in the document",
                    ManagementAction::Revert => "the style sheet's again",
                    _ => "removed",
                }
            );
        }
    }

    /// Use Another Style Sheet's actions.
    fn update_use_style_sheet(
        &mut self,
        action: style_dialogs::UseSheetAction,
    ) -> Task<cosmic::Action<Message>> {
        use style_dialogs::UseSheetAction;
        let Some(state) = self.use_sheet.as_mut() else {
            return Task::none();
        };
        match action {
            UseSheetAction::Select(index) => {
                state.selected = (index < state.sheets.len()).then_some(index);
            }
            UseSheetAction::ByDescription(on) => state.by_description = on,
            UseSheetAction::Cancel => self.use_sheet = None,
            UseSheetAction::Apply => {
                let Some(path) = state
                    .selected
                    .and_then(|index| state.sheets.get(index))
                    .map(|(path, _)| path.clone())
                else {
                    return Task::none();
                };
                self.use_sheet = None;
                self.use_style_sheet_file(&path);
            }
            UseSheetAction::Browse => {
                self.use_sheet = None;
                return self.choose_style_sheet_task();
            }
        }
        Task::none()
    }

    /// Applies the style sheet at `path` (Use Another Style Sheet, ST1B) as
    /// one undo step, and remembers its folder.
    fn use_style_sheet_file(&mut self, path: &std::path::Path) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let sheet = std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                amipro_formats::read_style_sheet(&bytes, name.clone())
                    .map_err(|error| error.to_string())
            });
        match sheet {
            Ok(sheet) => {
                if self.execute_document_command(&amipro_editing::Command::UseStyleSheet { sheet })
                {
                    self.status = format!("Using style sheet {name}");
                }
                if let Some(folder) = path.parent().map(PathBuf::from)
                    && !self.preferences.style_folders.contains(&folder)
                {
                    self.preferences.style_folders.push(folder);
                }
            }
            Err(error) => self.status = format!("Use Another Style Sheet failed: {error}"),
        }
    }

    /// The portal's open dialog for a style sheet, in the first style-sheet
    /// folder.
    fn choose_style_sheet_task(&self) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        let starting_directory = self.style_sheet_folders().into_iter().next();
        cosmic::task::future(async move {
            let filter = FileFilter::new("Ami Pro style sheets")
                .glob("*.sty")
                .glob("*.STY");
            let dialog = file_dialogs::open_dialog()
                .title("Use Another Style Sheet".to_owned())
                .filter(filter)
                .directory(starting_directory);
            let result = match dialog.open_file().await {
                Ok(response) => response.url().to_file_path().map_or_else(
                    |()| {
                        FileDialogResult::Failed(
                            "the selected style sheet is not a local file".into(),
                        )
                    },
                    FileDialogResult::Selected,
                ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::SheetChosen { session_id, result }
        })
    }

    /// Create Style's actions. Create makes the style from the chosen one,
    /// or from the caret paragraph with its own paragraph formatting, as one
    /// undo step; the paragraph keeps its style, as in Ami Pro (SS2).
    fn update_create_style(&mut self, action: style_dialogs::CreateAction) {
        use style_dialogs::CreateAction;
        let paragraph = self.caret_paragraph();
        let refused = self.create_style.as_ref().is_some_and(|state| {
            state.name.is_empty()
                || amipro_editing::style_name_problem(self.document.styles(), &state.name, None)
                    .is_some()
        });
        let Some(state) = self.create_style.as_mut() else {
            return;
        };
        match action {
            CreateAction::Name(name) => state.name = name,
            CreateAction::BasedOn(index) => {
                state.based_on = index;
                state.selected_text = false;
            }
            CreateAction::SelectedText(selected) => state.selected_text = selected,
            CreateAction::Cancel => self.create_style = None,
            // Enter in the name box acts as Create does: a name that cannot
            // be used waits, its problem shown under the box.
            CreateAction::Create | CreateAction::Modify if refused => {}
            CreateAction::Create | CreateAction::Modify => {
                let then_modify = action == CreateAction::Modify;
                let state = state.clone();
                let base = if state.selected_text {
                    paragraph.as_ref().map(|paragraph| paragraph.style)
                } else {
                    state.base()
                };
                let Some(base) = base else {
                    return;
                };
                let name = state.name.clone();
                if !self.execute_document_command(&amipro_editing::Command::CreateStyle {
                    name: name.clone(),
                    based_on: base,
                }) {
                    return;
                }
                // Selected text: the paragraph's own formatting joins the
                // new style, in the same undo step.
                if state.selected_text
                    && let Some(paragraph) =
                        paragraph.filter(|paragraph| paragraph.format != Default::default())
                    && let Some(mut style) = self
                        .document
                        .styles()
                        .values()
                        .find(|style| style.name == name)
                        .cloned()
                {
                    amipro_document::ParagraphFormat::apply(&paragraph.format, &mut style);
                    let session = &mut self.sessions[self.active_session];
                    let cursor = session.history_caret();
                    if let Err(error) = session.history.amend(
                        &mut session.document,
                        cursor,
                        &amipro_editing::Command::ModifyStyle { style },
                    ) {
                        session.status = format!("Edit failed: {error}");
                    }
                    session.refresh_rich_layout();
                }
                self.create_style = None;
                self.status = format!("Created style {name}");
                #[cfg(feature = "page-surface")]
                if then_modify {
                    let made = self
                        .document
                        .styles()
                        .values()
                        .find(|style| style.name == name)
                        .map(|style| style.id);
                    self.open_style_properties(made);
                }
                #[cfg(not(feature = "page-surface"))]
                let _ = then_modify;
            }
        }
    }

    /// The portal's save dialog for a style sheet, suggesting an 8.3 name
    /// Ami Pro can list, in the style-sheet folder when one is set.
    fn style_sheet_dialog_task(
        &self,
        description: String,
        contents: bool,
    ) -> Task<cosmic::Action<Message>> {
        let session_id = self.id;
        let suggested_name = suggested_style_sheet_name(&self.location, &self.document);
        let starting_directory = std::env::var_os("AMIPRO_STYLES")
            .and_then(|paths| std::env::split_paths(&paths).next())
            .or_else(|| match &self.location {
                DocumentLocation::LegacySource(path)
                | DocumentLocation::Sam(path)
                | DocumentLocation::Native(path) => path.parent().map(PathBuf::from),
                DocumentLocation::Untitled => None,
            });
        cosmic::task::future(async move {
            let filter = FileFilter::new("Ami Pro style sheets")
                .glob("*.sty")
                .glob("*.STY");
            let mut dialog = file_chooser::save::Dialog::new()
                .title("Save as a Style Sheet".to_owned())
                .file_name(suggested_name)
                .filter(filter);
            if let Some(directory) = starting_directory {
                dialog = dialog.directory(directory);
            }
            let result = match dialog.save_file().await {
                Ok(response) => response
                    .url()
                    .and_then(|url| url.to_file_path().ok())
                    .map_or_else(
                        || {
                            FileDialogResult::Failed(
                                "the selected destination is not a local file".into(),
                            )
                        },
                        FileDialogResult::Selected,
                    ),
                Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                Err(error) => FileDialogResult::Failed(error.to_string()),
            };
            Message::SheetDialogFinished {
                session_id,
                description,
                contents,
                result,
            }
        })
    }

    /// Saving as SAM (decision B): the loss check first; with nothing lost
    /// the document is written at once (`Some(true)` when written,
    /// `Some(false)` when that failed), otherwise the dialog asks (`None`).
    /// With `replace` it is written over the file at `path`, the original
    /// kept as a backup the first time (user, 2026-11-04).
    fn begin_sam_save(&mut self, path: PathBuf, replace: bool) -> Option<bool> {
        self.hide_codes_for_save();
        let saved_at = clock::ami_pro_now();
        let options = self.sam_options(saved_at);
        let source = match self.sam_source() {
            Ok(source) => source,
            Err(error) => {
                self.status = format!("Not saved: {error}");
                self.save_problem = Some(SaveProblem::not_written(&path, &error));
                return Some(false);
            }
        };
        let losses: Vec<String> = match amipro_formats::sam_losses(&source, &options) {
            Ok(losses) => losses.iter().map(amipro_formats::Loss::message).collect(),
            Err(error) => vec![format!("The written file could not be read back: {error}")],
        };
        if losses.is_empty() {
            return Some(self.write_sam(&path, saved_at, replace));
        }
        self.sam_save = Some(PendingSamSave {
            path,
            losses,
            saved_at,
            replace,
        });
        None
    }

    /// Tools › Preferences… (CM6): each value applies as it is set.
    fn update_preferences(
        &mut self,
        action: preferences_dialog::PreferencesAction,
    ) -> Task<cosmic::Action<Message>> {
        use preferences_dialog::{PreferencesAction as A, ViewSetting};
        let Some(state) = self.preferences_dialog.as_mut() else {
            return Task::none();
        };
        match action {
            A::Page(page) => state.page = page,
            A::UndoLevels(text) => {
                if let Some(levels) =
                    preferences_dialog::number_in(&text, &preferences_dialog::UNDO_LEVELS)
                {
                    self.preferences.undo_levels = levels;
                }
                state.undo_levels = text;
            }
            A::CustomZoom(text) => {
                if let Some(percent) =
                    preferences_dialog::number_in(&text, &preferences_dialog::CUSTOM_ZOOM)
                {
                    self.preferences.custom_zoom = percent as u16;
                }
                state.custom_zoom = text;
            }
            A::RecentFiles(text) => {
                if let Some(count) =
                    preferences_dialog::number_in(&text, &preferences_dialog::RECENT_FILES)
                {
                    self.preferences.recent_count = count;
                }
                state.recent_files = text;
            }
            A::KeepBackup(on) => self.preferences.keep_backup = on,
            A::CheckUpdates(on) => self.preferences.check_updates_on_start = on,
            A::KeepRecovery(on) => self.preferences.keep_recovery = on,
            A::RecoveryMinutes(minutes) => self.preferences.recovery_minutes = minutes,
            A::DefaultSheet(name) => self.preferences.default_sheet = name,
            A::AddFolder => {
                return chosen_folder("Add a style-sheet folder", |folder| {
                    Message::Preferences(A::FolderChosen(folder))
                });
            }
            A::ChooseDocuments => {
                let start = self.start_folder();
                return chosen_folder_in("Choose the Documents folder", start, |folder| {
                    Message::Preferences(A::DocumentsChosen(folder))
                });
            }
            A::DocumentsChosen(folder) => {
                if folder.is_some() {
                    self.preferences.documents_folder = folder;
                }
            }
            A::ClearDocuments => self.preferences.documents_folder = None,
            A::FolderChosen(folder) => {
                if let Some(folder) = folder
                    && !self.preferences.style_folders.contains(&folder)
                {
                    self.preferences.style_folders.push(folder);
                }
            }
            A::RemoveFolder(index) => {
                if index < self.preferences.style_folders.len() {
                    self.preferences.style_folders.remove(index);
                }
            }
            A::View(setting, on) => {
                let (shown, command) = match setting {
                    ViewSetting::Toolbar => {
                        (self.preferences.toolbar_visible, CommandId::ShowToolbar)
                    }
                    ViewSetting::Ruler => (self.preferences.ruler_visible, CommandId::Ruler),
                    ViewSetting::FieldShading => {
                        (self.preferences.field_shading, CommandId::FieldShading)
                    }
                    ViewSetting::Notes => (self.preferences.show_notes, CommandId::ShowNotes),
                    ViewSetting::Bookmarks => {
                        (self.preferences.show_bookmarks, CommandId::ShowBookmarks)
                    }
                    ViewSetting::TabsReturns => {
                        (self.preferences.tabs_returns, CommandId::ShowTabsReturns)
                    }
                    ViewSetting::ColumnGuides => {
                        (self.preferences.column_guides, CommandId::ColumnGuides)
                    }
                    ViewSetting::MarginsInColour => (
                        self.preferences.margins_in_colour,
                        CommandId::MarginsInColour,
                    ),
                    ViewSetting::Pictures => {
                        (self.preferences.show_pictures, CommandId::ShowPictures)
                    }
                    ViewSetting::TableAddresses => {
                        (self.preferences.table_addresses, CommandId::TableAddresses)
                    }
                };
                if shown != on {
                    return self.handle_message(Message::Command(command));
                }
            }
            A::CustomizeToolbar => {
                self.preferences_dialog = None;
                return self.handle_message(Message::Command(CommandId::CustomizeToolbar));
            }
            A::Close => self.preferences_dialog = None,
        }
        Task::none()
    }

    /// File › New… (CM1, CM2).
    fn update_new_document(&mut self, action: new_document_dialog::NewAction) {
        use new_document_dialog::NewAction as A;
        let Some(state) = self.new_document_dialog.as_mut() else {
            return;
        };
        match action {
            A::Select(index) if index < state.sheets.len() => {
                state.selected = Some(index);
                state.with_contents = state.sheets[index].has_contents;
                Self::preview_sheet(state);
            }
            A::Select(_) => {}
            A::ByDescription(on) => state.by_description = on,
            A::WithContents(on) => {
                state.with_contents = on;
                Self::preview_sheet(state);
            }
            A::Cancel => self.new_document_dialog = None,
            A::Create => {
                let Some(sheet) = state.selected_sheet().cloned() else {
                    return;
                };
                let with_contents = state.with_contents;
                self.new_document_dialog = None;
                match new_document_dialog::document_from_sheet(&sheet, with_contents) {
                    Ok(document) => {
                        let mut status = format!("A new document from {}", sheet.name);
                        if let Some(name) = &sheet.run_macro {
                            status.push_str(&format!(
                                "; its macro {name} is not run (AmiPro-Linux runs no macros)"
                            ));
                        }
                        let session = DocumentSession::new(
                            self.next_session_id,
                            document,
                            DocumentLocation::Untitled,
                            status,
                        );
                        self.next_session_id += 1;
                        self.sessions.push(session);
                        self.active_session = self.sessions.len() - 1;
                        new_document_dialog::remember(
                            &mut self.preferences.recent_sheets,
                            &sheet.name,
                        );
                        #[cfg(feature = "page-surface")]
                        self.auto_run_fields();
                    }
                    Err(error) => {
                        self.status = format!("{} could not be used: {error}", sheet.name);
                    }
                }
            }
        }
    }

    /// The selected sheet's first page as File › New… would make it.
    fn preview_sheet(state: &mut new_document_dialog::NewDocumentDialog) {
        #[cfg(feature = "page-surface")]
        {
            state.preview = state.selected_sheet().and_then(|sheet| {
                let document =
                    new_document_dialog::document_from_sheet(sheet, state.with_contents).ok()?;
                let mut pages = page_layout::SessionPages::default();
                pages.update(&document);
                page_preview::Miniature::first_page(pages.layout())
            });
        }
        #[cfg(not(feature = "page-surface"))]
        {
            state.preview = None;
        }
    }

    /// Show Clean Screen on or off: the chrome hidden and the window filling
    /// the screen, or both back.
    fn set_clean_screen(&mut self, on: bool) -> Task<cosmic::Action<Message>> {
        self.clean_screen = on;
        self.status = if on {
            "Clean Screen: Esc or Return shows the menus again".into()
        } else {
            "Clean Screen off".into()
        };
        let Some(id) = self.core.main_window_id() else {
            return Task::none();
        };
        let mode = if on {
            cosmic::iced::window::Mode::Fullscreen
        } else {
            cosmic::iced::window::Mode::Windowed
        };
        cosmic::iced::window::set_mode(id, mode)
    }

    /// Every document keeps the undo levels Preferences sets.
    fn apply_undo_levels(&mut self) {
        let levels = self.preferences.undo_levels;
        for session in &mut self.sessions {
            if session.history.limit() != levels {
                session.history.set_limit(levels);
            }
        }
    }

    /// A file opened or saved goes to the top of the File menu's list.
    fn remember_recent(&mut self, path: &std::path::Path) {
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        preferences_dialog::remember(&mut self.preferences.recent_files, &path);
    }

    /// Help › Compatibility and Fidelity (CM12): the loss check Save As SAM
    /// runs, for the active document, without writing anything.
    fn compatibility_check(&self) -> help_drawers::HelpDrawer {
        let session = &self.sessions[self.active_session];
        let options = self.sam_options(clock::ami_pro_now());
        let losses = self.sam_source().and_then(|source| {
            amipro_formats::sam_losses(&source, &options)
                .map(|losses| losses.iter().map(amipro_formats::Loss::message).collect())
                .map_err(|error| error.to_string())
        });
        help_drawers::HelpDrawer::Compatibility {
            session: session.id,
            name: session.tab_label(),
            losses,
        }
    }

    /// The document Save writes as SAM: the session's, or a delimited data
    /// file's table in its own form again (MR1).
    fn sam_source(&self) -> Result<Document, String> {
        #[cfg(feature = "page-surface")]
        if let Some(document) = self.data_for_sam()? {
            return Ok(document);
        }
        Ok(self.document.clone())
    }

    /// Show Power Fields goes off before a save, as Ami Pro switches it off
    /// (command `0xe1`).
    fn hide_codes_for_save(&mut self) {
        // The merge preview's values are never saved (MG5).
        #[cfg(feature = "page-surface")]
        self.leave_merge_preview();
        #[cfg(feature = "page-surface")]
        if self.codes_shown() {
            self.swap_codes(false);
        }
    }

    /// Saving a SAM at `saved_at`: the minutes of editing since the
    /// document was opened.
    fn sam_options(&self, saved_at: u32) -> amipro_formats::SamOptions {
        amipro_formats::SamOptions {
            editing_minutes: clock::editing_minutes(self.opened_at.elapsed()),
            statistics: Some(self.statistics()),
            ..amipro_formats::SamOptions::at(saved_at)
        }
    }

    /// Doc Info's statistics now, as Ami Pro's Update counts them: the
    /// pages of the last layout and the words and characters of every text
    /// stream; the size is the written file's.
    fn statistics(&self) -> amipro_document::Statistics {
        let (words, characters) = amipro_document::count_words_and_characters(&self.document);
        amipro_document::Statistics {
            pages: u32::try_from(self.pages.page_count()).unwrap_or(u32::MAX),
            words,
            characters,
            ..self.document.info().statistics
        }
    }

    fn write_sam(&mut self, path: &std::path::Path, saved_at: u32, replace: bool) -> bool {
        self.hide_codes_for_save();
        let source = match self.sam_source() {
            Ok(source) => source,
            Err(error) => {
                self.status = format!("Not saved: {error}");
                self.save_problem = Some(SaveProblem::not_written(path, &error));
                return false;
            }
        };
        #[cfg(feature = "page-surface")]
        let written = pictures::for_ami_pro(&source, self.location.folder());
        #[cfg(not(feature = "page-surface"))]
        let written = source;
        let options = self.sam_options(saved_at);
        let mut bytes = amipro_formats::sam_document(&written, &options);
        // A password set by File › Password… or given at File › Open: the
        // file is written protected, as Ami Pro's Save keeps it (PWSV2).
        if let Some(password) = &self.password {
            bytes = amipro_formats::protect::protect(&bytes, password);
        }
        let result = if replace {
            file_workflow::replace_sam_bytes(path, &bytes, self.preferences.keep_backup)
        } else {
            file_workflow::write_new_file_safely(path, &bytes).map(|()| {
                file_workflow::SamReplaced {
                    bytes: bytes.len(),
                    backup: None,
                }
            })
        };
        match result {
            Ok(file_workflow::SamReplaced { bytes, backup }) => {
                self.saved_at = Some(saved_at);
                // Save writes over it from now on (user, 2026-11-04).
                self.location = DocumentLocation::Sam(path.to_path_buf());
                self.status = match backup {
                    Some(backup) => format!(
                        "Saved Ami Pro document {} ({bytes} bytes); the original is kept as {}",
                        path.display(),
                        file_name(&backup)
                    ),
                    None => format!("Saved Ami Pro document {} ({bytes} bytes)", path.display()),
                };
                self.clean_document = self.document.clone();
                self.dirty = false;
                self.remember_recent(path);
                true
            }
            Err(error) => {
                self.status = format!("Save As failed: {error}");
                self.save_problem = Some(SaveProblem::write_failed(path, &error));
                false
            }
        }
    }

    fn save_to_native(&mut self, path: PathBuf) -> bool {
        self.hide_codes_for_save();
        // The file's Doc Info has this save's time and the minutes of
        // editing so far, as Ami Pro's files have (`11c8:342b`).
        let saved_at = clock::ami_pro_now();
        let mut written = self.document.clone();
        let statistics = self.statistics();
        let info = written.info_mut();
        info.saved = Some(saved_at);
        info.editing_minutes = info
            .editing_minutes
            .saturating_add(clock::editing_minutes(self.opened_at.elapsed()));
        info.statistics = statistics;
        match save_native_document(&written, &path) {
            Ok(()) => {
                self.saved_at = Some(saved_at);
                self.location = DocumentLocation::Native(path.clone());
                self.status = format!("Saved {}", path.display());
                self.clean_document = self.document.clone();
                self.dirty = false;
                self.remember_recent(&path);
                true
            }
            Err(error) => {
                self.status = format!("Save failed: {error}");
                self.save_problem = Some(SaveProblem::write_failed(&path, &error));
                false
            }
        }
    }

    fn toggle_selected_character_trait(&mut self, attribute: CharacterAttribute) {
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            self.toggle_page_attribute(attribute);
            return;
        }
        let cursor = self.editor.cursor();
        let Some(selection) = cursor.selection else {
            self.status = "Select text before applying character formatting".into();
            return;
        };
        let (start_position, end_position) = if (selection.line, selection.column)
            <= (cursor.position.line, cursor.position.column)
        {
            (selection, cursor.position)
        } else {
            (cursor.position, selection)
        };
        let Some(last_paragraph) = self.document.paragraphs().get(end_position.line) else {
            self.status = "Selection is outside the document model".into();
            return;
        };
        if end_position.column > last_paragraph.text.len() {
            self.status = "Selection is outside the document model".into();
            return;
        }
        let mut ranges = Vec::new();
        for paragraph_index in start_position.line..=end_position.line {
            let paragraph = &self.document.paragraphs()[paragraph_index];
            let start = if paragraph_index == start_position.line {
                start_position.column
            } else {
                0
            };
            let end = if paragraph_index == end_position.line {
                end_position.column
            } else {
                paragraph.text.len()
            };
            if start < end {
                ranges.push((paragraph_index, start, end));
            }
        }
        if ranges.is_empty() {
            self.status = "Select text before applying character formatting".into();
            return;
        }
        let enable = !ranges.iter().all(|(paragraph_index, start, end)| {
            let paragraph = &self.document.paragraphs()[*paragraph_index];
            let style = &self.document.styles()[&paragraph.style];
            range_has_character_trait(paragraph, style, *start, *end, attribute)
        });
        let mut formatted = self.document.clone();
        for (paragraph_index, start, end) in ranges {
            let style = formatted.styles()[&formatted.paragraphs()[paragraph_index].style].clone();
            if let Err(error) = formatted
                .paragraph_mut(paragraph_index)
                .modify_character_format(start, end, |format| attribute.set(format, &style, enable))
            {
                self.status = format!("Character formatting failed: {error}");
                return;
            }
        }
        self.push_undo();
        let next_id = self.document.next_id();
        self.document = formatted;
        self.document.raise_id_counter(next_id);
        self.refresh_rich_layout();
        self.dirty = true;
        let action = if enable { "Applied" } else { "Removed" };
        self.status = format!("{action} {}", attribute_label(attribute));
    }

    fn push_undo(&mut self) {
        self.sessions[self.active_session].push_undo();
    }

    fn restore_undo(&mut self) {
        #[cfg(feature = "page-surface")]
        if self.preview_refuses_edit() {
            return;
        }
        let cursor = self.history_caret();
        let session = &mut self.sessions[self.active_session];
        let previous = session.document.clone();
        if let Some(cursor) = session.history.undo(&mut session.document, cursor) {
            self.restore_history(&previous, cursor);
            self.status = "Undid edit".into();
        }
    }

    fn restore_redo(&mut self) {
        #[cfg(feature = "page-surface")]
        if self.preview_refuses_edit() {
            return;
        }
        let cursor = self.history_caret();
        let session = &mut self.sessions[self.active_session];
        let previous = session.document.clone();
        if let Some(cursor) = session.history.redo(&mut session.document, cursor) {
            self.restore_history(&previous, cursor);
            self.status = "Redid edit".into();
        }
    }

    /// Shows a root that history has just restored in place of `previous`.
    fn restore_history(&mut self, previous: &Document, caret: HistoryCaret) {
        let session = &mut self.sessions[self.active_session];
        synchronize_editor_text(&mut session.editor, previous, &session.document);
        self.refresh_rich_layout();
        #[cfg(feature = "page-surface")]
        if self.page_surface {
            let session = &mut self.sessions[self.active_session];
            session.restore_page_selection(&caret);
            session.frame_selection = caret
                .frames
                .as_ref()
                .and_then(|selection| selection.valid(&session.document));
            session.frame_keys = None;
            session.typing_run = None;
        }
        #[cfg(feature = "rich-editor-prototype")]
        {
            self.prototype_selection = PrototypeSelection::caret(0);
        }
        self.restore_history_cursor(caret.cursor);
        self.refresh_dirty();
    }

    fn restore_history_cursor(&mut self, cursor: text_editor::Cursor) {
        #[cfg(not(feature = "rich-editor-prototype"))]
        self.editor.move_to(cursor);
        #[cfg(feature = "rich-editor-prototype")]
        {
            move_prototype_fallback_cursor(&mut self.editor, cursor);
            if let Some(selection) =
                prototype_selection_from_text_cursor(&self.document, self.editor.cursor())
            {
                self.prototype_selection = selection;
            }
        }
    }

    #[cfg(feature = "rich-editor-prototype")]
    fn update_prototype(&mut self, action: PrototypeAction) {
        match action {
            PrototypeAction::Undo => {
                self.restore_undo();
                return;
            }
            PrototypeAction::Redo => {
                self.restore_redo();
                return;
            }
            _ => {}
        }

        let history_selection = self.prototype_selection;
        let mut selection = self.prototype_selection;
        let command =
            match rich_editor_prototype::action_command(&self.document, &mut selection, action) {
                Ok(command) => command,
                Err(error) => {
                    self.status = format!("Prototype edit failed: {error}");
                    return;
                }
            };
        if let Some(command) = command {
            let history_cursor = HistoryCaret {
                cursor: prototype_text_cursor(&self.document, history_selection)
                    .unwrap_or_else(|| self.editor.cursor()),
                ..self.history_caret()
            };
            let session = &mut self.sessions[self.active_session];
            let previous = session.document.clone();
            let executed =
                match session
                    .history
                    .execute(&mut session.document, history_cursor, &command)
                {
                    Ok(executed) => executed,
                    Err(error) => {
                        self.status = format!("Prototype edit failed: {error}");
                        return;
                    }
                };
            if !executed.report.is_empty() {
                synchronize_editor_text(&mut session.editor, &previous, &session.document);
                session.refresh_rich_layout();
                session.dirty = true;
                session.status = "Modified in isolated rich-editor prototype".into();
            }
            if let Some(caret) = executed.caret {
                match rich_editor_prototype::offset_for_cursor(&self.document, caret.into()) {
                    Ok(offset) => selection = PrototypeSelection::caret(offset),
                    Err(error) => {
                        self.status = format!("Prototype edit failed: {error}");
                        return;
                    }
                }
            }
        }
        self.prototype_selection = selection;
        if let Some(cursor) = prototype_text_cursor(&self.document, self.prototype_selection) {
            move_prototype_fallback_cursor(&mut self.editor, cursor);
        }
    }

    fn refresh_rich_layout(&mut self) {
        self.sessions[self.active_session].refresh_rich_layout();
    }

    fn refresh_dirty(&mut self) {
        self.dirty = self.document != self.clean_document;
    }

    /// Mirrors session order, labels, dirty marks, and the active session into
    /// the native tab model. Sessions remain the only source of truth.
    fn sync_tab_model(&mut self) {
        let session_ids: Vec<u64> = self.sessions.iter().map(|session| session.id).collect();
        let stale: Vec<_> = self
            .tab_model
            .iter()
            .filter(|entity| {
                self.tab_model
                    .data::<TabSession>(*entity)
                    .is_none_or(|tab| !session_ids.contains(&tab.0))
            })
            .collect();
        for entity in stale {
            self.tab_model.remove(entity);
        }
        for (position, session) in self.sessions.iter().enumerate() {
            let label = session.tab_label();
            let entity = match tab_entity(&self.tab_model, session.id) {
                Some(entity) => {
                    if self.tab_model.text(entity) != Some(label.as_str()) {
                        self.tab_model.text_set(entity, label);
                    }
                    entity
                }
                None => self
                    .tab_model
                    .insert()
                    .text(label)
                    .icon(widget::icon::from_name("text-x-generic-symbolic").icon())
                    .closable()
                    .data(TabSession(session.id))
                    .id(),
            };
            let position = u16::try_from(position).unwrap_or(u16::MAX);
            if self.tab_model.position(entity) != Some(position) {
                self.tab_model.position_set(entity, position);
            }
        }
        let active_id = self.sessions[self.active_session].id;
        if let Some(entity) = tab_entity(&self.tab_model, active_id)
            && !self.tab_model.is_active(entity)
        {
            self.tab_model.activate(entity);
        }
    }

    fn session_index_for_tab(&self, entity: segmented_button::Entity) -> Option<usize> {
        let TabSession(id) = *self.tab_model.data::<TabSession>(entity)?;
        self.sessions.iter().position(|session| session.id == id)
    }

    /// Import Options for a file about to be read (IM2), the last choice
    /// preselected.
    fn ask_import(
        &mut self,
        source: import::Source,
        format: amipro_exchange::import::SourceFormat,
        purpose: import::Purpose,
    ) {
        let options = import::options_from(
            &self.preferences.import_styles,
            &self.preferences.import_line_ends,
            self.preferences.import_keep_names,
        );
        self.import_dialog = Some(import::OptionsDialog::new(source, format, purpose, options));
    }

    /// Insert › File…'s file (IM3): Import Options, then the file at the
    /// caret of the tab it was chosen in.
    #[cfg(feature = "page-surface")]
    fn ask_insert_file(&mut self, session_id: u64, path: PathBuf) {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return;
        };
        self.active_session = index;
        // Ami Pro refuses a protected file here without asking (3148).
        if std::fs::read(&path).is_ok_and(|bytes| amipro_formats::protect::is_protected(&bytes)) {
            self.status = format!(
                "{}: {}",
                file_name(&path),
                amipro_formats::ImportError::PasswordProtected
            );
            return;
        }
        let Some(format) = import::source_format(&path) else {
            self.status = format!(
                "{} is not a Word, OpenDocument, Rich Text, Markdown, or text file",
                file_name(&path)
            );
            return;
        };
        let purpose = self.insert_purpose(false);
        self.ask_import(import::Source::File(path), format, purpose);
    }

    /// Where an inserted document goes: the caret, or the selection's
    /// start (Ami Pro's Insert leaves the insertion point where it was).
    #[cfg(feature = "page-surface")]
    fn insert_purpose(&self, paste: bool) -> import::Purpose {
        let (at, _) = self.page_selection.ordered();
        import::Purpose::Insert {
            session: self.id,
            at,
            first_page: self.page_selection.page,
            paste,
        }
    }

    /// Import Options' choices: Open (or Insert) reads the file off the
    /// interface's thread and keeps the choice for next time.
    fn update_import_options(
        &mut self,
        action: import::OptionsAction,
    ) -> Task<cosmic::Action<Message>> {
        let Some(dialog) = self.import_dialog.as_mut() else {
            return Task::none();
        };
        match action {
            import::OptionsAction::Cancel => {
                let open = dialog.purpose == import::Purpose::Open;
                self.import_dialog = None;
                self.status = if open {
                    "Open cancelled".into()
                } else {
                    "Insert File cancelled".into()
                };
                Task::none()
            }
            import::OptionsAction::Open => {
                let Some(dialog) = self.import_dialog.take() else {
                    return Task::none();
                };
                let (styles, line_ends, keep_names) = import::words_of(&dialog.options);
                self.preferences.import_styles = styles.into();
                self.preferences.import_line_ends = line_ends.into();
                self.preferences.import_keep_names = keep_names;
                self.save_preferences();
                self.start_import(dialog.source, dialog.options, dialog.purpose)
            }
            action => {
                dialog.act(action);
                Task::none()
            }
        }
    }

    /// Reads and realises a document on a thread of its own; `Imported`
    /// brings it back.
    fn start_import(
        &mut self,
        source: import::Source,
        options: amipro_exchange::import::ImportOptions,
        purpose: import::Purpose,
    ) -> Task<cosmic::Action<Message>> {
        let (base, into_text) = match purpose {
            import::Purpose::Open => (new_document(), false),
            import::Purpose::Insert { session, at, .. } => {
                let Some(session) = self.sessions.iter().find(|open| open.id == session) else {
                    return Task::none();
                };
                (
                    session.document.clone(),
                    at.flow != session.document.main_flow_id(),
                )
            }
        };
        self.status = format!("Reading {}…", source.name());
        let job = import::ImportJob {
            source,
            options,
            purpose,
            base,
            into_text,
            now: clock::ami_pro_now(),
        };
        cosmic::task::future(async move {
            let result = import::in_background(move || job.run()).await;
            Message::Imported(Box::new(result))
        })
    }

    /// A document read: a new tab, its source protected (Open), or the
    /// text at the caret as one undo step (Insert › File…, Paste).
    fn imported(&mut self, result: Result<import::Imported, String>) {
        let imported = match result {
            Ok(imported) => imported,
            Err(error) => {
                self.status = format!("Import failed: {error}");
                return;
            }
        };
        let status = import::status(&imported);
        match imported.purpose {
            import::Purpose::Open => {
                let import::Source::File(path) = imported.source else {
                    return;
                };
                if let Some(index) = session_index_for_path(&self.sessions, &path) {
                    self.active_session = index;
                    self.status = format!("Activated {}", path.display());
                    return;
                }
                let mut session = DocumentSession::new(
                    self.next_session_id,
                    imported.document,
                    DocumentLocation::LegacySource(path),
                    status,
                );
                session.import_report =
                    (!imported.report.findings.is_empty()).then_some(imported.report);
                self.next_session_id += 1;
                self.sessions.push(session);
                self.active_session = self.sessions.len() - 1;
                #[cfg(feature = "page-surface")]
                self.show_import_report();
                #[cfg(feature = "page-surface")]
                self.auto_run_fields();
            }
            #[cfg(feature = "page-surface")]
            import::Purpose::Insert {
                session,
                at,
                first_page,
                paste,
            } => {
                let Some(index) = self.sessions.iter().position(|open| open.id == session) else {
                    return;
                };
                self.active_session = index;
                // Paste replaces the selection it began at.
                let mut commands = Vec::new();
                let (start, end) = self.page_selection.ordered();
                if paste && start == at && start != end {
                    commands.push(amipro_editing::Command::ReplaceText {
                        start,
                        end,
                        text: String::new(),
                    });
                }
                commands.push(amipro_editing::Command::InsertDocument {
                    at,
                    source: amipro_editing::InsertedDocument(Arc::new(imported.document)),
                    first_page,
                });
                let depth = self.history.undo_len();
                self.execute_page_commands(&commands);
                if self.history.undo_len() != depth {
                    self.status = status;
                    // The jumps are the realised document's places, not
                    // the inserted text's.
                    let mut report = imported.report;
                    report.markers.clear();
                    self.import_report = (!report.findings.is_empty()).then_some(report);
                    self.show_import_report();
                }
            }
            #[cfg(not(feature = "page-surface"))]
            import::Purpose::Insert { .. } => {}
        }
    }

    /// Opens the Import Report in the rail when the last import listed
    /// anything (IM8).
    #[cfg(feature = "page-surface")]
    fn show_import_report(&mut self) {
        if !self.page_surface || self.import_report.is_none() {
            return;
        }
        self.close_rail_panels();
        self.import_report_shown = true;
        self.open_rail();
    }

    #[cfg(feature = "page-surface")]
    fn update_import_report(
        &mut self,
        action: import_report::ReportAction,
    ) -> Task<cosmic::Action<Message>> {
        use import_report::ReportAction;
        match action {
            ReportAction::Show => self.show_import_report(),
            ReportAction::Back => self.import_report_shown = false,
            ReportAction::Close => {
                self.import_report_shown = false;
                if self.rail_open() {
                    self.toggle_rail();
                }
            }
            ReportAction::Copy => {
                if let Some(text) = self.import_report.as_ref().map(import_report::text) {
                    self.status = "Import Report copied".into();
                    return cosmic::iced::clipboard::write(text);
                }
            }
            ReportAction::GoTo(index) => return self.go_to_finding(index),
        }
        Task::none()
    }

    /// Selects the next place of finding `index` that is still in the
    /// document, on the page that shows it.
    #[cfg(feature = "page-surface")]
    fn go_to_finding(&mut self, index: usize) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        let Some(report) = session.import_report.as_mut() else {
            return Task::none();
        };
        let Some(finding) = report.findings.iter().nth(index) else {
            return Task::none();
        };
        let places: Vec<amipro_editing::TextPosition> = finding
            .markers
            .iter()
            .filter_map(|marker| report.markers.get(marker))
            .filter_map(|place| {
                let flow = session.document.flow(place.flow)?;
                let paragraph = flow
                    .paragraphs
                    .iter()
                    .position(|paragraph| paragraph.id == place.paragraph)?;
                let text = &flow.paragraphs[paragraph].text;
                let mut byte = place.byte.min(text.len());
                while !text.is_char_boundary(byte) {
                    byte -= 1;
                }
                Some(amipro_editing::TextPosition {
                    flow: place.flow,
                    paragraph,
                    byte,
                })
            })
            .collect();
        if places.is_empty() {
            session.status = "Its places are no longer in the document".into();
            return Task::none();
        }
        if report.next.len() <= index {
            report.next.resize(index + 1, 0);
        }
        let shown = report.next[index] % places.len();
        report.next[index] = shown + 1;
        let at = places[shown];
        let selection = PageSelection::caret(at, session.page_selection.page);
        let page = page_surface::caret_page(&session.document, session.pages.layout(), selection);
        session.frame_selection = None;
        session.page_selection = PageSelection::caret(at, page);
        session.sync_editor_to_page();
        session.status = format!("{} of {}", shown + 1, places.len());
        session.scroll_to_caret()
    }

    fn open_document_path(&mut self, path: PathBuf) {
        if let Some(index) = session_index_for_path(&self.sessions, &path) {
            self.active_session = index;
            self.status = format!("Activated {}", path.display());
            return;
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        let is_sam = extension.as_deref() == Some("sam");
        // Other programs' documents, known by their contents (IM1): Import
        // Options first, as in Ami Pro.
        if !is_sam
            && extension.as_deref() != Some("amipro")
            && let Some(format) = import::source_format(&path)
        {
            self.ask_import(import::Source::File(path), format, import::Purpose::Open);
            return;
        }
        if is_sam
            && std::fs::read(&path).is_ok_and(|bytes| amipro_formats::protect::is_protected(&bytes))
        {
            // Ami Pro's Password Protect box (dialog 1042).
            self.password_dialog = Some(password_dialog::PasswordDialog::open(path));
            return;
        }
        if is_sam {
            self.open_sam(path, None);
        } else {
            match Document::load_native(&path) {
                Ok(mut document) => {
                    // A file written before the fields run kept Doc Info as
                    // sections; opening counts a revision, as Ami Pro counts
                    // one for its files (`1058:2732`).
                    amipro_formats::take_doc_info(&mut document);
                    let info = document.info_mut();
                    info.revisions = info.revisions.saturating_add(1);
                    let session = DocumentSession::new(
                        self.next_session_id,
                        document,
                        DocumentLocation::Native(path.clone()),
                        format!("Opened {}", path.display()),
                    );
                    self.next_session_id += 1;
                    self.sessions.push(session);
                    self.active_session = self.sessions.len() - 1;
                    self.remember_recent(&path);
                    #[cfg(feature = "page-surface")]
                    self.auto_run_fields();
                }
                Err(error) => self.status = format!("Open failed: {error}"),
            }
        }
    }

    /// Opens an Ami Pro document, with its password when it is protected.
    fn open_sam(&mut self, path: PathBuf, password: Option<String>) {
        match amipro_formats::import_sam_path_with_password(
            &path,
            &self.style_folders(),
            password.as_deref(),
        ) {
            Ok(import) => {
                let style_status = import.report.associated_style.as_deref().map_or(
                    "embedded styles".to_owned(),
                    |name| {
                        if import.report.associated_style_resolved {
                            format!("resolved {name}")
                        } else {
                            format!("missing {name}; fallback styles")
                        }
                    },
                );
                let status = format!(
                    "Imported {} paragraphs, {} styles ({style_status})",
                    import.report.paragraphs_imported, import.report.styles_imported
                );
                let session = DocumentSession::new(
                    self.next_session_id,
                    import.document,
                    DocumentLocation::Sam(path.clone()),
                    status,
                );
                self.next_session_id += 1;
                self.sessions.push(session);
                self.active_session = self.sessions.len() - 1;
                self.sessions[self.active_session].password = password;
                self.remember_recent(&path);
                // Auto run fields update as the document opens (FL4).
                #[cfg(feature = "page-surface")]
                self.auto_run_fields();
            }
            Err(amipro_formats::ImportError::WrongPassword) => {
                // Ami Pro's message 3147; the box stays for another try.
                let mut dialog = password_dialog::PasswordDialog::open(path);
                dialog.error = Some(amipro_formats::ImportError::WrongPassword.to_string());
                self.password_dialog = Some(dialog);
            }
            Err(error) => self.status = format!("SAM import failed: {error}"),
        }
    }

    /// The password box's actions (`password_dialog`).
    fn update_password(&mut self, action: password_dialog::PasswordAction) {
        let Some(dialog) = self.password_dialog.as_mut() else {
            return;
        };
        match dialog.update(action) {
            password_dialog::Outcome::Stay => {}
            password_dialog::Outcome::Cancel => self.password_dialog = None,
            password_dialog::Outcome::Open(path, password) => {
                self.password_dialog = None;
                self.open_sam(path, Some(password));
            }
            password_dialog::Outcome::Set(password) => {
                self.password_dialog = None;
                let session = &mut self.sessions[self.active_session];
                if session.password != password {
                    session.status = if password.is_some() {
                        "Saves of this document as an Ami Pro document are password protected"
                            .into()
                    } else {
                        "The document is saved without a password from now on".into()
                    };
                    session.password = password;
                    session.dirty = true;
                }
            }
        }
    }

    /// Shows the title in the window's own header bar, which libcosmic
    /// draws from the header title, and gives it to the compositor for the
    /// task bar and window switcher (user, 2026-10-12: no title was visible).
    fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let title = self.window_title();
        self.set_header_title(title.clone());
        self.core
            .main_window_id()
            .map_or_else(Task::none, |id| self.set_window_title(title, id))
    }

    fn window_title(&self) -> String {
        format!("{} — AmiPro-Linux", self.display_name())
    }
}

/// The Open dialog's filters (`UI_SPEC.md` §31): all the documents read
/// first, then each format; File › Open also lists Ami Pro's and the app's
/// own documents.
fn document_filters(dialog: file_dialogs::OpenDialog, ami_pro: bool) -> file_dialogs::OpenDialog {
    let ami_pro_patterns: &[(&str, &[&str])] = if ami_pro {
        &[
            ("Ami Pro documents", &["sam"]),
            ("AmiPro-Linux documents", &["amipro"]),
        ]
    } else {
        &[]
    };
    let formats: Vec<(&str, &[&str])> = ami_pro_patterns
        .iter()
        .copied()
        .chain(import::PATTERNS)
        .collect();
    let globs = |filter: FileFilter, extensions: &[&str]| {
        extensions.iter().fold(filter, |filter, extension| {
            filter
                .glob(&format!("*.{extension}"))
                .glob(&format!("*.{}", extension.to_ascii_uppercase()))
        })
    };
    let all = formats.iter().fold(
        FileFilter::new("All documents"),
        |filter, (_, extensions)| globs(filter, extensions),
    );
    formats
        .iter()
        .fold(dialog.filter(all), |dialog, (name, extensions)| {
            dialog.filter(globs(FileFilter::new(name), extensions))
        })
}

/// Brings `editor` from `current`'s text to `replacement`'s by replacing only
/// the changed span, so history restores and fallback synchronization do not
/// rebuild and reshape every line. The delta is derived from the semantic
/// texts because Iced's grapheme deletion and newline handling differ from the
/// model's character and CRLF/CR normalization semantics.
fn synchronize_editor_text(
    editor: &mut text_editor::Content,
    current: &Document,
    replacement: &Document,
) {
    synchronize_editor_plain_text(editor, &current.plain_text(), &replacement.plain_text());
}

/// Brings `editor`, which shows `current_text`, to `replacement_text` by
/// replacing only the changed span.
fn synchronize_editor_plain_text(
    editor: &mut text_editor::Content,
    current_text: &str,
    replacement_text: &str,
) {
    if current_text == replacement_text {
        return;
    }
    let (prefix, suffix) = amipro_editing::changed_span(current_text, replacement_text);
    let end = current_text.len() - suffix;
    let (Some(start_position), Some(end_position)) = (
        text_position_at_offset(current_text, prefix),
        text_position_at_offset(current_text, end),
    ) else {
        *editor = text_editor::Content::with_text(replacement_text);
        return;
    };
    editor.move_to(text_editor::Cursor {
        position: end_position,
        selection: (prefix != end).then_some(start_position),
    });
    if prefix == end {
        editor.perform(text_editor::Action::ClearSelection);
    }
    editor.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
        Arc::new(replacement_text[prefix..replacement_text.len() - suffix].to_owned()),
    )));
}

#[cfg(feature = "rich-editor-prototype")]
fn move_prototype_fallback_cursor(editor: &mut text_editor::Content, cursor: text_editor::Cursor) {
    editor.move_to(cursor);
    if cursor.selection.is_none() {
        editor.perform(text_editor::Action::ClearSelection);
    }
}

pub(crate) fn session_index_for_path(
    sessions: &[DocumentSession],
    path: &std::path::Path,
) -> Option<usize> {
    sessions.iter().position(|session| {
        // A recovered copy is not the file it came from.
        #[cfg(feature = "page-surface")]
        if session.recovered {
            return false;
        }
        matches!(
            &session.location,
            DocumentLocation::LegacySource(open)
            | DocumentLocation::Sam(open)
            | DocumentLocation::Native(open)
                if open == path
        )
    })
}

fn next_untitled_title(sessions: &[DocumentSession]) -> String {
    (1..)
        .map(|number| {
            if number == 1 {
                "Untitled".to_owned()
            } else {
                format!("Untitled {number}")
            }
        })
        .find(|candidate| {
            sessions.iter().all(|session| {
                !matches!(session.location, DocumentLocation::Untitled)
                    || session.document.title != *candidate
            })
        })
        .expect("the set of open untitled document names is finite")
}

#[cfg(all(test, feature = "rich-editor-prototype"))]
fn document_text_width(document: &Document) -> f32 {
    let [_, right, _, left] = document.page_setup().margin_points;
    (document.page_setup().width_points - left - right).max(1.0)
        * workspace_geometry::SCREEN_PIXELS_PER_POINT
}

#[cfg_attr(
    feature = "rich-editor-prototype",
    allow(
        dead_code,
        reason = "the accepted editor is retained for non-prototype builds"
    )
)]
fn standard_editor<'a>(
    document: &'a Document,
    editor: &'a text_editor::Content,
    editor_size: f32,
) -> Element<'a, Message> {
    cosmic::iced::widget::text_editor(editor)
        .placeholder("Start writing…")
        .font(default_document_font())
        .size(editor_size)
        .line_height(LineHeight::Relative(1.2))
        .highlight_with::<DocumentHighlighter>(
            DocumentHighlightSettings::from_document(document),
            document_highlight_format,
        )
        .key_binding(standard_editor_key_binding)
        .on_action(Message::Edit)
        .height(Length::Fill)
        .into()
}

fn standard_editor_key_binding(
    key_press: text_editor::KeyPress,
) -> Option<text_editor::Binding<Message>> {
    let shortcut = command::command_for_shortcut(
        key_press.modifiers,
        &key_press.key,
        Some(&key_press.physical_key),
    )
    .filter(|command| {
        command::metadata(*command).implementation == Implementation::Implemented
            && *command != CommandId::Delete
    });
    shortcut.map_or_else(
        || text_editor::Binding::from_key_press(key_press),
        |command| Some(text_editor::Binding::Custom(Message::Command(command))),
    )
}

fn standard_action_changes_document(action: &text_editor::Action) -> bool {
    action.is_edit()
}

#[cfg(feature = "rich-editor-prototype")]
fn prototype_text_cursor(
    document: &Document,
    selection: PrototypeSelection,
) -> Option<text_editor::Cursor> {
    rich_editor_prototype::cursor_at_offset(document, selection.focus).ok()?;
    rich_editor_prototype::cursor_at_offset(document, selection.anchor).ok()?;
    let text = document.plain_text();
    let focus = text_position_at_offset(&text, selection.focus)?;
    let anchor = text_position_at_offset(&text, selection.anchor)?;
    Some(text_editor::Cursor {
        position: focus,
        selection: (!selection.is_caret()).then_some(anchor),
    })
}

#[cfg(feature = "rich-editor-prototype")]
fn prototype_selection_from_text_cursor(
    document: &Document,
    cursor: text_editor::Cursor,
) -> Option<PrototypeSelection> {
    let text = document.plain_text();
    let offset = |position: text_editor::Position| text_offset_for_position(&text, position);
    let focus = offset(cursor.position)?;
    let anchor = cursor.selection.and_then(offset).unwrap_or(focus);
    rich_editor_prototype::cursor_at_offset(document, focus).ok()?;
    rich_editor_prototype::cursor_at_offset(document, anchor).ok()?;
    Some(PrototypeSelection { anchor, focus })
}

fn text_position_at_offset(text: &str, offset: usize) -> Option<text_editor::Position> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let mut line = 0;
    for (range, ending) in cosmic_text::LineIter::new(text) {
        if offset <= range.end {
            return Some(text_editor::Position {
                line,
                column: offset.checked_sub(range.start)?,
            });
        }
        let after_ending = range.end + ending.as_str().len();
        if offset < after_ending {
            return None;
        }
        line += 1;
    }
    (offset == text.len()).then_some(text_editor::Position { line, column: 0 })
}

/// The byte offset of `position` in `text`, whose lines end with `\n` only.
/// Line starts are found with `memchr`, so this stays fast on long documents.
fn line_text_offset(text: &str, position: text_editor::Position) -> Option<usize> {
    let line_start = match position.line {
        0 => 0,
        line => text.match_indices('\n').nth(line - 1)?.0 + 1,
    };
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |end| line_start + end);
    let offset = line_start.checked_add(position.column)?;
    (offset <= line_end && text.is_char_boundary(offset)).then_some(offset)
}

#[cfg(feature = "rich-editor-prototype")]
fn text_offset_for_position(text: &str, position: text_editor::Position) -> Option<usize> {
    let mut line_count = 0;
    for (range, _ending) in cosmic_text::LineIter::new(text) {
        if line_count == position.line {
            let offset = range.start.checked_add(position.column)?;
            return (offset <= range.end && text.is_char_boundary(offset)).then_some(offset);
        }
        line_count += 1;
    }
    (position.line == line_count && position.column == 0).then_some(text.len())
}

/// How the status line names a character attribute.
fn attribute_label(attribute: CharacterAttribute) -> &'static str {
    match attribute {
        CharacterAttribute::Bold => "bold",
        CharacterAttribute::Italic => "italic",
        CharacterAttribute::Underline => "underline",
        CharacterAttribute::WordUnderline => "word underline",
        CharacterAttribute::DoubleUnderline => "double underline",
        CharacterAttribute::Strikethrough => "strikethrough",
        CharacterAttribute::Superscript => "superscript",
        CharacterAttribute::Subscript => "subscript",
        CharacterAttribute::Caps(Caps::Upper) => "upper case",
        CharacterAttribute::Caps(Caps::Lower) => "lower case",
        CharacterAttribute::Caps(Caps::Initial) => "initial caps",
        CharacterAttribute::Caps(Caps::Small) => "small caps",
        CharacterAttribute::Caps(Caps::Normal) => "normal case",
        CharacterAttribute::Protected => "protected text",
        CharacterAttribute::NoHyphenation => "no hyphenation",
        CharacterAttribute::Overstrike(_) => "overstrike",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToggleState {
    Off,
    On,
    Mixed,
    Unavailable,
}

fn selected_character_toggle_state(
    document: &Document,
    cursor: text_editor::Cursor,
    attribute: CharacterAttribute,
) -> ToggleState {
    let Some(selection) = cursor.selection else {
        return ToggleState::Unavailable;
    };
    let (start_position, end_position) =
        if (selection.line, selection.column) <= (cursor.position.line, cursor.position.column) {
            (selection, cursor.position)
        } else {
            (cursor.position, selection)
        };
    range_character_state(
        document,
        document.paragraphs(),
        (start_position.line, start_position.column),
        (end_position.line, end_position.column),
        attribute,
    )
}

/// Whether `attribute` is on, off, or mixed over `paragraphs` (one
/// flow's) from `start` to `end`, each a (paragraph, byte) pair.
fn range_character_state(
    document: &Document,
    paragraphs: &[Arc<Paragraph>],
    start: (usize, usize),
    end: (usize, usize),
    attribute: CharacterAttribute,
) -> ToggleState {
    let (start_position, end_position) = (start, end);
    let mut any_enabled = false;
    let mut any_disabled = false;
    for paragraph_index in start_position.0..=end_position.0 {
        let Some(paragraph) = paragraphs.get(paragraph_index) else {
            return ToggleState::Unavailable;
        };
        let start = if paragraph_index == start_position.0 {
            start_position.1
        } else {
            0
        };
        let end = if paragraph_index == end_position.0 {
            end_position.1
        } else {
            paragraph.text.len()
        };
        if start >= end || end > paragraph.text.len() {
            continue;
        }
        let style = &document.styles()[&paragraph.style];
        let base = attribute.is_on(&CharacterFormat::default(), style);
        let mut boundaries = vec![start, end];
        for run in &paragraph.runs {
            if run.end > start && run.start < end {
                boundaries.push(run.start.max(start));
                boundaries.push(run.end.min(end));
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        for segment in boundaries.windows(2) {
            if segment[0] == segment[1] {
                continue;
            }
            let enabled = paragraph
                .runs
                .iter()
                .find(|run| run.start <= segment[0] && run.end >= segment[1])
                .map_or(base, |run| attribute.is_on(&run.format, style));
            any_enabled |= enabled;
            any_disabled |= !enabled;
        }
    }

    match (any_enabled, any_disabled) {
        (true, true) => ToggleState::Mixed,
        (true, false) => ToggleState::On,
        (false, true) => ToggleState::Off,
        (false, false) => ToggleState::Unavailable,
    }
}

fn range_has_character_trait(
    paragraph: &Paragraph,
    style: &ParagraphStyle,
    start: usize,
    end: usize,
    attribute: CharacterAttribute,
) -> bool {
    if start >= end || end > paragraph.text.len() {
        return false;
    }
    let base = attribute.is_on(&CharacterFormat::default(), style);
    let mut cursor = start;
    for run in paragraph
        .runs
        .iter()
        .filter(|run| run.end > start && run.start < end)
    {
        if cursor < run.start.min(end) && !base {
            return false;
        }
        let value = attribute.is_on(&run.format, style);
        if !value {
            return false;
        }
        cursor = cursor.max(run.end.min(end));
    }
    cursor >= end || base
}

fn ruler_view(geometry: PageViewGeometry) -> Element<'static, Message> {
    let ticks: Vec<Element<'static, Message>> = geometry
        .ruler_segments()
        .into_iter()
        .map(|segment| {
            widget::container(widget::text(format!("{}│", segment.inch)))
                .width(segment.width)
                .height(30)
                .into()
        })
        .collect();
    let tick_row = widget::row::with_children(ticks).width(geometry.page_width);
    let marker_row = widget::row::with_children(vec![
        widget::container(widget::text("▼"))
            .align_right(geometry.margin_left)
            .height(30)
            .into(),
        widget::container(widget::space::horizontal())
            .width(geometry.text_width())
            .height(30)
            .into(),
        widget::container(widget::text("▼"))
            .width(geometry.margin_right)
            .height(30)
            .into(),
    ]);
    cosmic::iced::widget::stack(vec![tick_row.into(), marker_row.into()])
        .width(geometry.page_width)
        .height(30)
        .into()
}

fn selected_paragraph_range(
    paragraph_count: usize,
    cursor: text_editor::Cursor,
) -> Option<RangeInclusive<usize>> {
    if paragraph_count == 0 {
        return None;
    }
    let position = cursor.position.line.min(paragraph_count - 1);
    let selection = cursor.selection.map_or(position, |selection| {
        selection.line.min(paragraph_count - 1)
    });
    Some(position.min(selection)..=position.max(selection))
}

fn selected_paragraph_style(document: &Document, cursor: text_editor::Cursor) -> Option<StyleId> {
    let mut paragraphs = selected_paragraph_range(document.paragraphs().len(), cursor)?;
    let first = document.paragraphs()[*paragraphs.start()].style;
    paragraphs
        .all(|index| document.paragraphs()[index].style == first)
        .then_some(first)
}

/// "Intro é" / "a\u{FFFC}b\u{FFFC}c 🙂" (Heading 1) / "Tail": a page break
/// and a floating header that owns a flow, made through commands.
#[cfg(test)]
fn object_fixture_document() -> Document {
    use amipro_document::{BreakKind, HeaderFooterKind, PageSides};
    use amipro_editing::{Command, TextPosition, apply};

    let document = Document::with_paragraphs([
        ("Intro é", StyleId(1)),
        ("abc 🙂", StyleId(2)),
        ("Tail", StyleId(1)),
    ]);
    let at = |byte| TextPosition {
        flow: document.main_flow_id(),
        paragraph: 1,
        byte,
    };
    let document = apply(
        &document,
        &Command::InsertBreak {
            at: at(1),
            kind: BreakKind::Page,
            centre: false,
            layout: None,
        },
    )
    .unwrap()
    .document;
    let document = apply(
        &document,
        &Command::InsertFloatingHeaderFooter {
            at: at(5),
            kind: HeaderFooterKind::Header,
            pages: PageSides::All,
        },
    )
    .unwrap()
    .document;
    document.validate().unwrap();
    document
}

/// A user's initials from their account name, for new notes.
fn references_initials(name: &str) -> String {
    name.split(|character: char| !character.is_alphanumeric())
        .filter_map(|part| part.chars().next())
        .take(3)
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    use super::*;
    use amipro_document::{CharacterFormat, CharacterRun};
    use cosmic::iced::advanced::layout;
    use cosmic::iced::advanced::widget::Tree;

    fn test_app(document: Document, location: DocumentLocation) -> AmiProApp {
        let mut app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![DocumentSession::new(1, document, location, "Ready".into())],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        app.sync_tab_model();
        app
    }

    /// In the text editor a first look at the clipboard that finds no text
    /// is tried once more; Paste reads the clipboard whatever its state.
    #[test]
    fn the_text_editor_looks_at_the_clipboard_again() {
        let mut app = test_app(
            Document::from_plain_text("Text"),
            DocumentLocation::Untitled,
        );
        let update = |app: &mut AmiProApp, message| {
            cosmic::iced::runtime::task::into_stream(cosmic::Application::update(app, message))
        };
        assert!(
            update(
                &mut app,
                Message::ClipboardAvailability {
                    has_text: false,
                    retry: true,
                },
            )
            .is_some()
        );
        assert!(
            update(
                &mut app,
                Message::ClipboardAvailability {
                    has_text: false,
                    retry: false,
                },
            )
            .is_none()
        );
        assert!(!app.command_enabled(CommandId::Paste));
        assert!(update(&mut app, Message::Command(CommandId::Paste)).is_some());
    }

    /// Runs `message` and the app messages its tasks publish, as the
    /// runtime does (no portal is reached).
    fn run_messages(app: &mut AmiProApp, message: Message) {
        let mut messages = vec![message];
        while let Some(message) = messages.pop() {
            let task = cosmic::Application::update(app, message);
            let Some(mut stream) = cosmic::iced::runtime::task::into_stream(task) else {
                continue;
            };
            while let Some(action) = cosmic::iced::futures::executor::block_on(
                cosmic::iced::futures::StreamExt::next(&mut stream),
            ) {
                if let cosmic::iced::runtime::Action::Output(cosmic::Action::App(message)) = action
                {
                    messages.push(message);
                }
            }
        }
    }

    #[test]
    fn both_builds_open_other_programs_documents() {
        // `UI_SPEC.md` §31: the text-editor build reads the same formats
        // through the same Import Options.
        let folder = scratch_directory("import-open");
        let path = folder.join("Letter.rtf");
        std::fs::write(
            &path,
            br"{\rtf1\ansi{\stylesheet{\s0 Normal;}}{\s0 Dear reader,\par}{\s0 Yours\par}}",
        )
        .unwrap();
        let mut app = test_app(new_document(), DocumentLocation::Untitled);
        run_messages(
            &mut app,
            Message::OpenDialogFinished(FileDialogResult::Selected(path.clone())),
        );
        assert!(app.import_dialog.is_some());
        assert!(cosmic::Application::dialog(&app).is_some());
        run_messages(
            &mut app,
            Message::ImportOptions(import::OptionsAction::Open),
        );
        assert_eq!(app.sessions.len(), 2, "{}", app.status);
        assert_eq!(app.location, DocumentLocation::LegacySource(path));
        assert_eq!(app.document.plain_text(), "Dear reader,\nYours");
        assert_eq!(app.editor.text().trim_end(), "Dear reader,\nYours");
        let _ = std::fs::remove_dir_all(folder);
    }

    /// `AMIPRO_TRACE_DROP` keeps libcosmic's drop and clipboard traces and
    /// the runtime's drop events, not its other events or other crates'.
    #[test]
    fn the_drop_trace_keeps_drops_and_the_clipboard() {
        let path = std::env::temp_dir().join(format!("amipro-drop-trace-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let trace = DropTrace(path.clone().into_os_string());
        let record = |target: &'static str, line: std::fmt::Arguments<'_>| {
            log::Log::log(
                &trace,
                &log::Record::builder()
                    .target(target)
                    .level(log::Level::Trace)
                    .args(line)
                    .build(),
            );
        };
        record(
            "libcosmic::widget::dnd_destination",
            format_args!("offer drop id=7"),
        );
        record("iced::winit::program", format_args!("Dnd event Offer"));
        record("iced::winit::program", format_args!("Redraw"));
        record("wgpu_core::device", format_args!("buffer"));
        record(
            "iced::winit::clipboard",
            format_args!("register destination"),
        );
        let written = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let lines: Vec<&str> = written.lines().collect();
        assert_eq!(lines.len(), 3, "{written}");
        assert!(lines[0].ends_with("libcosmic::widget::dnd_destination: offer drop id=7"));
        assert!(lines[1].ends_with("iced::winit::program: Dnd event Offer"));
        assert!(lines[2].ends_with("iced::winit::clipboard: register destination"));
    }

    #[test]
    fn save_as_a_style_sheet_writes_a_new_sheet_and_links_the_document_sd1() {
        let directory = std::env::temp_dir().join(format!(
            "amipro-save-sheet-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("REPORT.SAM");
        let mut app = test_app(
            Document::from_plain_text("Text"),
            DocumentLocation::LegacySource(source.clone()),
        );
        assert!(app.command_enabled(CommandId::SaveAsStyleSheet));
        send_message(&mut app, Message::Command(CommandId::SaveAsStyleSheet));
        assert_eq!(app.save_sheet.as_deref(), Some(""));
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(
            &mut app,
            Message::SaveSheet(SaveSheetAction::Description("Report <sheet>".into())),
        );
        send_message(&mut app, Message::SaveSheet(SaveSheetAction::Choose));
        assert_eq!(
            app.save_sheet, None,
            "the dialog closes for the file chooser"
        );
        // The portal returns a name without an extension: .STY is added.
        let session_id = app.id;
        send_message(
            &mut app,
            Message::SheetDialogFinished {
                contents: false,
                session_id,
                description: "Report <sheet>".into(),
                result: FileDialogResult::Selected(directory.join("REPORT")),
            },
        );
        let written = directory.join("REPORT.STY");
        let bytes = std::fs::read(&written).unwrap();
        let sheet = amipro_formats::read_style_sheet(&bytes, "REPORT.STY").unwrap();
        assert_eq!(sheet.styles.len(), app.document.styles().len());
        assert!(String::from_utf8_lossy(&bytes).contains("[desc]\r\n\tReport <<sheet>\r\n"));
        // Ami Pro links a saved document to the sheet it wrote (SD1).
        assert_eq!(
            app.document.style_sheet().map(|link| link.name.as_str()),
            Some("REPORT.STY")
        );
        assert!(
            app.document
                .styles()
                .values()
                .all(|style| style.origin == amipro_document::StyleOrigin::Sheet)
        );
        assert!(app.dirty);
        assert!(app.status.starts_with("Saved style sheet"));
        // One undo step takes the link back; the sheet stays written.
        send_message(&mut app, Message::Command(CommandId::Undo));
        assert_eq!(app.document.style_sheet(), None);
        assert!(written.exists());
        // An existing file is never replaced.
        send_message(
            &mut app,
            Message::SheetDialogFinished {
                contents: false,
                session_id,
                description: "Again".into(),
                result: FileDialogResult::Selected(written.clone()),
            },
        );
        assert_eq!(std::fs::read(&written).unwrap(), bytes);
        assert!(app.status.contains("already exists"));
        // A dialog says so, and offers the file chooser again with the
        // same description and With contents.
        let problem = app.save_problem.take().expect("a dialog says why");
        assert!(
            problem.title.ends_with("already exists"),
            "{}",
            problem.title
        );
        assert_eq!(
            problem.retry,
            SaveRetry::StyleSheet {
                description: "Again".into(),
                contents: false
            }
        );
        // Neither is a SAM.
        send_message(
            &mut app,
            Message::SheetDialogFinished {
                contents: false,
                session_id,
                description: String::new(),
                result: FileDialogResult::Selected(source.clone()),
            },
        );
        assert!(!source.exists());
        assert!(app.status.contains("ends in .sty"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn create_style_bases_a_new_style_on_a_style_or_the_caret_paragraph() {
        use style_dialogs::CreateAction;
        let mut document =
            Document::with_paragraphs([("Body", StyleId(1)), ("Heading", StyleId(2))]);
        document.paragraph_mut(1).format.indents = Some(amipro_document::Indents {
            all: 720,
            first: 0,
            rest: 0,
            right: 0,
        });
        let mut app = test_app(document, DocumentLocation::Untitled);
        app.editor.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 1, column: 2 },
            selection: None,
        });
        let heading = app.document.styles()[&StyleId(2)].clone();
        let count = app.document.styles().len();
        send_message(&mut app, Message::Command(CommandId::CreateStyle));
        let state = app.create_style.clone().unwrap();
        assert_eq!(
            state.base(),
            Some(StyleId(2)),
            "the caret paragraph's style"
        );
        // A taken name, letter case aside, is refused before Create.
        send_message(
            &mut app,
            Message::CreateStyle(CreateAction::Name(heading.name.to_uppercase())),
        );
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(&mut app, Message::CreateStyle(CreateAction::Create));
        assert_eq!(app.document.styles().len(), count);
        send_message(
            &mut app,
            Message::CreateStyle(CreateAction::Name("Mine".into())),
        );
        send_message(&mut app, Message::CreateStyle(CreateAction::Create));
        assert_eq!(app.create_style, None);
        let mine = app
            .document
            .styles()
            .values()
            .find(|style| style.name == "Mine")
            .unwrap()
            .clone();
        assert_eq!(mine.font_size_points, heading.font_size_points);
        assert_eq!(mine.indents, heading.indents);
        assert_eq!(mine.origin, amipro_document::StyleOrigin::Document);
        assert_eq!(
            app.document.paragraphs()[1].style,
            StyleId(2),
            "kept, as in Ami Pro"
        );
        // Selected text: the caret paragraph's own indents join the style,
        // in the same undo step.
        send_message(&mut app, Message::Command(CommandId::CreateStyle));
        send_message(
            &mut app,
            Message::CreateStyle(CreateAction::Name("From text".into())),
        );
        send_message(
            &mut app,
            Message::CreateStyle(CreateAction::SelectedText(true)),
        );
        send_message(&mut app, Message::CreateStyle(CreateAction::Create));
        let from_text = app
            .document
            .styles()
            .values()
            .find(|style| style.name == "From text")
            .unwrap();
        assert_eq!(from_text.indents.all, 720);
        send_message(&mut app, Message::Command(CommandId::Undo));
        assert!(
            !app.document
                .styles()
                .values()
                .any(|style| style.name == "From text")
        );
        assert!(
            app.document
                .styles()
                .values()
                .any(|style| style.name == "Mine")
        );
    }

    #[test]
    fn saving_sam_writes_over_with_a_backup_and_lists_what_ami_pro_cannot_store() {
        let directory = scratch_directory("save-sam");
        let mut app = test_app(
            Document::from_plain_text("First line\nSecond line"),
            DocumentLocation::Untitled,
        );
        app.dirty = true;
        let session_id = app.id;
        let finished = |path: PathBuf, format| Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(path),
            format,
        };

        // The Format choice adds .SAM; nothing is lost, so it is written at
        // once, and Save writes over it from then on (user, 2026-11-04).
        send_message(
            &mut app,
            finished(directory.join("LETTER"), Some(SaveFormat::Sam)),
        );
        let written = directory.join("LETTER.SAM");
        let bytes = std::fs::read(&written).unwrap();
        let copy = amipro_formats::import_sam(&bytes, None, "LETTER")
            .unwrap()
            .document;
        assert_eq!(copy.plain_text(), app.document.plain_text());
        assert_eq!(app.sam_save, None);
        assert!(!app.dirty);
        assert_eq!(app.location, DocumentLocation::Sam(written.clone()));
        assert_eq!(
            app.location.save_request(),
            SaveRequest::SaveSam(written.clone())
        );
        assert!(
            app.status.starts_with("Saved Ami Pro document"),
            "{}",
            app.status
        );
        let read = |path: &std::path::Path| {
            amipro_formats::import_sam(&std::fs::read(path).unwrap(), None, "LETTER")
                .unwrap()
                .document
                .plain_text()
        };

        // Save writes over the file, keeping the original the first time.
        let backup = directory.join("LETTER.SAM.bak");
        app.document = Document::from_plain_text("First line\nSecond line\nThird line");
        app.dirty = true;
        send_message(&mut app, Message::Command(CommandId::Save));
        assert!(!app.dirty, "{}", app.status);
        assert_eq!(read(&written), "First line\nSecond line\nThird line");
        assert_eq!(std::fs::read(&backup).unwrap(), bytes);
        assert!(app.status.contains("LETTER.SAM.bak"), "{}", app.status);
        // Later saves leave the backup as the first original.
        app.document = Document::from_plain_text("Only line");
        app.dirty = true;
        send_message(&mut app, Message::Command(CommandId::Save));
        assert_eq!(read(&written), "Only line");
        assert_eq!(std::fs::read(&backup).unwrap(), bytes);
        assert!(!app.status.contains(".bak"), "{}", app.status);

        // Save As onto an existing Ami Pro document asks first; Cancel
        // leaves it, Replace writes over it.
        app.dirty = true;
        app.document = Document::from_plain_text("Replaced");
        let other = directory.join("OTHER.SAM");
        std::fs::write(&other, &bytes).unwrap();
        send_message(&mut app, finished(other.clone(), None));
        assert_eq!(app.overwrite_confirmation.pending(), Some(other.as_path()));
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(&mut app, Message::CancelOverwrite);
        assert_eq!(std::fs::read(&other).unwrap(), bytes);
        assert!(app.dirty);
        send_message(&mut app, finished(other.clone(), None));
        send_message(&mut app, Message::ConfirmOverwrite);
        assert_eq!(read(&other), "Replaced");
        assert_eq!(
            std::fs::read(directory.join("OTHER.SAM.bak")).unwrap(),
            bytes
        );
        assert_eq!(app.location, DocumentLocation::Sam(other.clone()));
        assert!(!app.dirty);

        // A refused save opens a dialog: the status bar alone was easy to
        // miss (user, 2026-10-13). Choose Another Name… opens the file
        // chooser again; Cancel closes it.
        send_message(&mut app, finished(directory.join("STYLES.STY"), None));
        let problem = app.save_problem.clone().expect("a dialog says why");
        assert_eq!(problem.retry, SaveRetry::SaveAs);
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(&mut app, Message::SaveProblemClose);
        assert_eq!(app.save_problem, None);
        assert!(cosmic::Application::dialog(&app).is_none());
        send_message(&mut app, finished(directory.join("STYLES.STY"), None));
        drop(cosmic::Application::update(
            &mut app,
            Message::SaveProblemRetry,
        ));
        assert_eq!(app.save_problem, None);
        // A .sty name is not a document's.
        send_message(&mut app, finished(directory.join("STYLES.STY"), None));
        assert_eq!(
            app.save_problem
                .as_ref()
                .map(|problem| problem.title.as_str()),
            Some("Not a document name")
        );
        send_message(&mut app, Message::SaveProblemClose);

        // A typed .sam decides the format whatever the choice says.
        send_message(
            &mut app,
            finished(directory.join("memo.sam"), Some(SaveFormat::Native)),
        );
        assert!(directory.join("memo.sam").exists());
        assert!(!directory.join("memo.amipro").exists());

        // What Ami Pro cannot store is listed first (decision B): Cancel
        // writes nothing, Save Anyway writes the file.
        let mut app = test_app(
            Document::from_plain_text("Arrows \u{2192} and \u{2190}"),
            DocumentLocation::Untitled,
        );
        app.dirty = true;
        let session_id = app.id;
        let finished = |path: PathBuf| Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(path),
            format: None,
        };
        let lossy = directory.join("ARROWS.SAM");
        send_message(&mut app, finished(lossy.clone()));
        let pending = app.sam_save.clone().expect("the loss check asks");
        assert_eq!(pending.path, lossy);
        assert!(!pending.losses.is_empty());
        assert!(!lossy.exists());
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(&mut app, Message::CancelSamSave);
        assert_eq!(app.sam_save, None);
        assert!(!lossy.exists());
        assert!(app.dirty);
        assert!(cosmic::Application::dialog(&app).is_none());

        send_message(&mut app, finished(lossy.clone()));
        send_message(&mut app, Message::SaveSamAnyway);
        assert_eq!(app.sam_save, None);
        let copy = amipro_formats::import_sam(&std::fs::read(&lossy).unwrap(), None, "ARROWS")
            .unwrap()
            .document;
        assert_eq!(copy.plain_text(), "Arrows ? and ?");
        assert!(!app.dirty);

        // Closing with unsaved changes: the tab closes once Save Anyway has
        // written the file, and stays open when the user cancels.
        app.dirty = true;
        app.close_after_save = Some(session_id);
        let second = directory.join("SECOND.SAM");
        send_message(&mut app, finished(second.clone()));
        send_message(&mut app, Message::CancelSamSave);
        assert_eq!(app.close_after_save, None);
        assert!(app.sessions.iter().any(|session| session.id == session_id));
        app.close_after_save = Some(session_id);
        send_message(&mut app, finished(second.clone()));
        send_message(&mut app, Message::SaveSamAnyway);
        assert!(second.exists());
        assert!(!app.sessions.iter().any(|session| session.id == session_id));

        // Save over an Ami Pro document runs the loss check too; from the
        // close prompt, the tab closes once Save Anyway has written it.
        let mut app = test_app(
            Document::from_plain_text("Arrows \u{2192} again"),
            DocumentLocation::Sam(second.clone()),
        );
        app.dirty = true;
        let session_id = app.id;
        let before = std::fs::read(&second).unwrap();
        send_message(&mut app, Message::Command(CommandId::Save));
        let pending = app.sam_save.clone().expect("the loss check asks");
        assert!(pending.replace);
        send_message(&mut app, Message::CancelSamSave);
        assert_eq!(std::fs::read(&second).unwrap(), before);
        app.pending_close = Some(app.active_session);
        send_message(&mut app, Message::SaveAndClose);
        assert!(app.sam_save.is_some());
        assert_eq!(std::fs::read(&second).unwrap(), before);
        send_message(&mut app, Message::SaveSamAnyway);
        assert_eq!(
            std::fs::read(directory.join("SECOND.SAM.bak")).unwrap(),
            before
        );
        assert!(!app.sessions.iter().any(|session| session.id == session_id));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn save_as_a_style_sheet_writes_its_contents_by_default() {
        let directory = scratch_directory("sheet-contents");
        let mut app = test_app(
            Document::from_plain_text("The sheet's own text"),
            DocumentLocation::LegacySource(directory.join("MEMO.SAM")),
        );
        let session_id = app.id;
        send_message(&mut app, Message::Command(CommandId::SaveAsStyleSheet));
        assert!(
            app.save_sheet_contents,
            "Ami Pro's default is With contents"
        );
        // Turned off and on again: what the dialog has when Save… is
        // pressed goes with the chosen file.
        send_message(
            &mut app,
            Message::SaveSheet(SaveSheetAction::Contents(false)),
        );
        assert!(!app.save_sheet_contents);
        send_message(
            &mut app,
            Message::SaveSheet(SaveSheetAction::Contents(true)),
        );
        send_message(&mut app, Message::SaveSheet(SaveSheetAction::Choose));
        for (name, contents) in [("WITH", true), ("WITHOUT", false)] {
            send_message(
                &mut app,
                Message::SheetDialogFinished {
                    session_id,
                    description: String::new(),
                    contents,
                    result: FileDialogResult::Selected(directory.join(name)),
                },
            );
            let bytes = std::fs::read(directory.join(format!("{name}.STY"))).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            // A new document made from the sheet starts with its text.
            let made = amipro_formats::import_sam(&bytes, None, "From sheet")
                .unwrap()
                .document;
            if contents {
                assert!(
                    text.contains("[edoc]\r\nThe sheet's own text\r\n"),
                    "{text}"
                );
                assert!(text.contains("[Embedded]\r\n"));
                assert_eq!(made.plain_text(), "The sheet's own text");
            } else {
                assert!(text.ends_with("[edoc]\r\n"), "{text}");
                assert_eq!(made.plain_text(), "");
            }
        }
        // Opening the dialog again starts with contents on.
        send_message(&mut app, Message::Command(CommandId::SaveAsStyleSheet));
        send_message(
            &mut app,
            Message::SaveSheet(SaveSheetAction::Contents(false)),
        );
        send_message(&mut app, Message::SaveSheet(SaveSheetAction::Cancel));
        send_message(&mut app, Message::Command(CommandId::SaveAsStyleSheet));
        assert!(app.save_sheet_contents);
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A scratch directory for one test.
    fn scratch_directory(label: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "amipro-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    /// A style sheet file whose Body Text is set in `font` and whose second
    /// style is named `second` at 24 pt.
    fn write_sheet(path: &std::path::Path, font: &str, second: &str, description: &str) {
        let mut styles = amipro_document::default_styles();
        styles.get_mut(&StyleId(1)).unwrap().font_family = font.into();
        let style = styles.get_mut(&StyleId(2)).unwrap();
        style.name = second.into();
        style.font_size_points = 24.0;
        let sheet = amipro_formats::style_sheet(
            &Document::empty("sheet", styles),
            &amipro_formats::SheetOptions {
                contents: false,
                description: description.into(),
                saved_at: 0,
            },
        );
        std::fs::write(path, sheet).unwrap();
    }

    #[test]
    fn style_management_reverts_removes_and_moves_styles_st1() {
        use amipro_document::StyleOrigin;
        use style_dialogs::ManagementAction;
        let directory = scratch_directory("management");
        write_sheet(&directory.join("SHEET.STY"), "Arial", "Title", "Sheet");
        let mut styles = amipro_document::default_styles();
        let style = |id: u64, name: &str, size: f32, origin| {
            let mut style = styles[&StyleId(1)].clone();
            style.id = StyleId(id);
            style.name = name.into();
            style.font_size_points = size;
            style.origin = origin;
            style
        };
        let changed_title = style(2, "Title", 30.0, StyleOrigin::Document);
        let mine = style(4, "Mine", 12.0, StyleOrigin::Document);
        let bullet = style(5, "Bullet", 12.0, StyleOrigin::Sheet);
        styles.get_mut(&StyleId(1)).unwrap().origin = StyleOrigin::Sheet;
        for style in [changed_title, mine, bullet] {
            styles.insert(style.id, style);
        }
        styles.remove(&StyleId(3));
        let mut document = Document::empty("ST1", styles);
        for id in [1, 2, 4, 5] {
            document.push_paragraph("para", StyleId(id));
        }
        document.set_style_sheet(Some(amipro_document::StyleSheetLink {
            name: "sheet.sty".into(),
            layout_from_sheet: true,
        }));
        document.validate().unwrap();
        let mut app = test_app(
            document,
            DocumentLocation::LegacySource(directory.join("ST1.SAM")),
        );
        send_message(&mut app, Message::Command(CommandId::StyleManagement));
        let state = app.style_management.clone().unwrap();
        assert!(state.sheet.is_some(), "found beside the document, any case");
        assert!(cosmic::Application::dialog(&app).is_some());
        let origin = |app: &AmiProApp, id: u64| app.document.styles()[&StyleId(id)].origin;
        // Revert: the changed Title is the sheet's again.
        send_message(
            &mut app,
            Message::StyleManagement(ManagementAction::Select(StyleId(2))),
        );
        send_message(&mut app, Message::StyleManagement(ManagementAction::Revert));
        assert_eq!(origin(&app, 2), StyleOrigin::Sheet, "{}", app.status);
        assert_eq!(app.document.styles()[&StyleId(2)].font_size_points, 24.0);
        // Remove: Mine is defined nowhere and formats as Body Text.
        send_message(
            &mut app,
            Message::StyleManagement(ManagementAction::Select(StyleId(4))),
        );
        send_message(&mut app, Message::StyleManagement(ManagementAction::Remove));
        assert_eq!(origin(&app, 4), StyleOrigin::Missing);
        // << Move <<: Bullet is stored in the document.
        send_message(
            &mut app,
            Message::StyleManagement(ManagementAction::Select(StyleId(5))),
        );
        send_message(
            &mut app,
            Message::StyleManagement(ManagementAction::MoveToDocument),
        );
        assert_eq!(origin(&app, 5), StyleOrigin::Document);
        // Each was one undo step.
        send_message(&mut app, Message::Command(CommandId::Undo));
        send_message(&mut app, Message::Command(CommandId::Undo));
        assert_eq!(origin(&app, 4), StyleOrigin::Document);
        send_message(&mut app, Message::StyleManagement(ManagementAction::Close));
        assert_eq!(app.style_management, None);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn use_another_style_sheet_lists_sheets_by_description_and_applies_one_st1b() {
        use style_dialogs::UseSheetAction;
        let directory = scratch_directory("use-sheet");
        write_sheet(&directory.join("A.STY"), "Arial", "Title", "Zulu sheet");
        write_sheet(
            &directory.join("B.STY"),
            "Courier New",
            "Heading",
            "Alpha sheet",
        );
        std::fs::write(directory.join("NOTES.TXT"), b"not a sheet").unwrap();
        // A document made from a style sheet: its styles are the sheet's.
        let mut document = Document::default();
        for style in document.styles_mut().values_mut() {
            style.origin = amipro_document::StyleOrigin::Sheet;
        }
        document.set_style_sheet(Some(amipro_document::StyleSheetLink {
            name: "OLD.STY".into(),
            layout_from_sheet: true,
        }));
        let mut app = test_app(document, DocumentLocation::Untitled);
        app.preferences.style_folders = vec![directory.clone()];
        send_message(&mut app, Message::Command(CommandId::UseAnotherStyleSheet));
        let state = app.use_sheet.clone().unwrap();
        assert_eq!(state.sheets.len(), 2);
        assert!(
            state
                .sheets
                .iter()
                .any(|(_, description)| description == "Alpha sheet")
        );
        assert!(cosmic::Application::dialog(&app).is_some());
        let b = state
            .sheets
            .iter()
            .position(|(path, _)| path.ends_with("B.STY"))
            .unwrap();
        send_message(&mut app, Message::UseStyleSheet(UseSheetAction::Select(b)));
        send_message(&mut app, Message::UseStyleSheet(UseSheetAction::Apply));
        assert_eq!(app.use_sheet, None);
        assert_eq!(
            app.document.style_sheet().map(|link| link.name.as_str()),
            Some("B.STY")
        );
        let body = app
            .document
            .styles()
            .values()
            .find(|style| style.name == "Body Text")
            .unwrap();
        assert_eq!(body.font_family, "Courier New");
        send_message(&mut app, Message::Command(CommandId::Undo));
        assert_eq!(
            app.document.style_sheet().map(|link| link.name.as_str()),
            Some("OLD.STY")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn styles_panel_rows_apply_create_rename_and_remove_styles() {
        use amipro_document::StyleOrigin;
        let mut document = Document::with_paragraphs([("One", StyleId(1)), ("Two", StyleId(1))]);
        document.styles_mut().get_mut(&StyleId(2)).unwrap().origin = StyleOrigin::Document;
        let mut app = test_app(document, DocumentLocation::Untitled);
        let name = |app: &AmiProApp, id: u64| app.document.styles()[&StyleId(id)].name.clone();
        // As the panel's right-click menu does: the right press marks the
        // row, then the entry acts on it.
        let row = |app: &mut AmiProApp, id: u64, action| {
            send_message(app, Message::StyleRowContext(Some(StyleId(id))));
            send_message(app, Message::StyleRowMenu(action));
            assert_eq!(app.rail.context_style, None);
        };
        // Apply: the caret paragraph takes the style.
        row(&mut app, 2, StyleRowAction::Apply);
        assert_eq!(app.document.paragraphs()[0].style, StyleId(2));
        // Create Style Based on This: the dialog starts from the row's style.
        row(&mut app, 3, StyleRowAction::CreateBasedOn);
        assert_eq!(app.create_style.as_ref().unwrap().base(), Some(StyleId(3)));
        send_message(
            &mut app,
            Message::CreateStyle(style_dialogs::CreateAction::Cancel),
        );
        // Rename: the dialog refuses a taken name, then renames.
        let taken = name(&app, 1);
        row(&mut app, 2, StyleRowAction::Rename);
        send_message(&mut app, Message::RenameStyle(RenameAction::Name(taken)));
        send_message(&mut app, Message::RenameStyle(RenameAction::Rename));
        assert!(app.rename_style.is_some(), "a taken name is refused");
        send_message(
            &mut app,
            Message::RenameStyle(RenameAction::Name("Renamed".into())),
        );
        assert!(cosmic::Application::dialog(&app).is_some());
        send_message(&mut app, Message::RenameStyle(RenameAction::Rename));
        assert_eq!(app.rename_style, None);
        assert_eq!(name(&app, 2), "Renamed");
        // Remove: a document style is defined nowhere and formats as Body
        // Text; undo brings it back.
        row(&mut app, 2, StyleRowAction::Remove);
        assert_eq!(
            app.document.styles()[&StyleId(2)].origin,
            StyleOrigin::Missing
        );
        send_message(&mut app, Message::Command(CommandId::Undo));
        assert_eq!(
            app.document.styles()[&StyleId(2)].origin,
            StyleOrigin::Document
        );
        // The panel marks the document style with Ami Pro's bullet.
        assert_eq!(
            style_dialogs::listed_name(&app.document.styles()[&StyleId(2)]),
            "• Renamed"
        );
        let _ = cosmic::Application::view(&app);
    }

    #[test]
    fn save_as_a_style_sheet_detaches_an_untitled_document_ss2() {
        let directory =
            std::env::temp_dir().join(format!("amipro-save-sheet-untitled-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory).unwrap();
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        send_message(&mut app, Message::Command(CommandId::SaveAsStyleSheet));
        send_message(&mut app, Message::SaveSheet(SaveSheetAction::Cancel));
        assert_eq!(app.save_sheet, None);
        let session_id = app.id;
        send_message(
            &mut app,
            Message::SheetDialogFinished {
                contents: false,
                session_id,
                description: String::new(),
                result: FileDialogResult::Selected(directory.join("NEW.sty")),
            },
        );
        assert!(directory.join("NEW.sty").exists());
        assert_eq!(app.document.style_sheet(), None);
        assert!(
            app.document
                .styles()
                .values()
                .all(|style| style.origin == amipro_document::StyleOrigin::Document)
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn select_first_five_bytes(app: &mut AmiProApp) {
        app.editor.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 0, column: 5 },
            selection: Some(text_editor::Position { line: 0, column: 0 }),
        });
        #[cfg(feature = "rich-editor-prototype")]
        {
            app.prototype_selection = PrototypeSelection {
                anchor: 0,
                focus: 5,
            };
        }
    }

    #[test]
    fn command_enablement_tracks_selection_history_save_target_and_tabs() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "alpha beta".into();
        let mut app = test_app(
            document,
            DocumentLocation::Native(PathBuf::from("/tmp/current.amipro")),
        );

        assert!(!app.command_enabled(CommandId::Save));
        assert!(!app.command_enabled(CommandId::Undo));
        assert!(!app.command_enabled(CommandId::Cut));
        assert!(app.command_enabled(CommandId::Paste));
        app.clipboard_has_text = Some(false);
        assert!(!app.command_enabled(CommandId::Paste));
        app.clipboard_has_text = Some(true);
        assert!(app.command_enabled(CommandId::Paste));
        assert!(app.command_enabled(CommandId::SelectAll));
        // Find works on the page surface (`UI_SPEC.md` §27); the text
        // editor keeps it for later.
        assert_eq!(
            app.command_enabled(CommandId::Find),
            cfg!(feature = "page-surface")
        );
        assert!(!app.command_enabled(CommandId::NextDocument));

        select_first_five_bytes(&mut app);
        assert!(app.command_enabled(CommandId::Cut));
        assert!(app.command_enabled(CommandId::Copy));
        assert!(app.command_enabled(CommandId::Delete));
        assert!(app.command_enabled(CommandId::Bold));

        app.dirty = true;
        app.push_undo();
        app.sessions.push(DocumentSession::new(
            2,
            Document::default(),
            DocumentLocation::Untitled,
            "Second".into(),
        ));
        assert!(app.command_enabled(CommandId::Save));
        assert!(app.command_enabled(CommandId::Undo));
        assert!(app.command_enabled(CommandId::NextDocument));

        app.location = DocumentLocation::LegacySource(PathBuf::from("/tmp/README31.SAM"));
        app.dirty = false;
        assert!(app.command_enabled(CommandId::Save));
        assert_eq!(app.location.save_request(), SaveRequest::ChooseDestination);
    }

    #[test]
    fn native_window_title_uses_active_filename_and_exact_application_name() {
        let mut document = Document::default();
        document.title = "Semantic title".into();
        document.paragraph_mut(0).text = "content".into();
        let mut app = test_app(
            document,
            DocumentLocation::LegacySource(PathBuf::from("/tmp/README31.SAM")),
        );

        assert_eq!(app.window_title(), "README31.SAM — AmiPro-Linux");

        app.location = DocumentLocation::Native(PathBuf::from("/tmp/Draft.amipro"));
        assert_eq!(
            app.window_title(),
            "Draft.amipro — AmiPro-Linux",
            "Save As must change the native title to the destination filename"
        );
    }

    #[test]
    fn window_title_tracks_new_switch_and_close_transitions() {
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        assert_eq!(app.window_title(), "Untitled — AmiPro-Linux");

        // The header bar shows what libcosmic's header title holds (user,
        // 2026-10-12: the title was missing from the window).
        let header = |app: &AmiProApp| app.core.window.header_title.clone();
        drop(app.update_title());
        assert_eq!(header(&app), "Untitled — AmiPro-Linux");

        drop(app.dispatch_command(CommandId::NewDocument));
        assert_eq!(app.document.title, "Untitled 2");
        assert_eq!(app.window_title(), "Untitled 2 — AmiPro-Linux");
        assert_eq!(header(&app), "Untitled 2 — AmiPro-Linux");

        drop(app.activate_tab(0));
        assert_eq!(app.window_title(), "Untitled — AmiPro-Linux");
        assert_eq!(header(&app), "Untitled — AmiPro-Linux");

        drop(app.close_tab(0));
        assert_eq!(app.window_title(), "Untitled 2 — AmiPro-Linux");
        assert_eq!(header(&app), "Untitled 2 — AmiPro-Linux");
    }

    #[test]
    fn new_documents_have_ami_pros_doc_info_and_saves_count_the_editing_time() {
        let document = new_document();
        let info = document.info();
        assert_eq!((info.revisions, info.flags), (1, 1));
        assert!(info.created.is_some_and(|created| created > 1_700_000_000));
        // A save adds the rounded minutes since the document was opened.
        let mut app = test_app(document, DocumentLocation::Untitled);
        app.opened_at = std::time::Instant::now() - std::time::Duration::from_secs(150);
        let options = app.sam_options(1_792_520_631);
        assert_eq!(options.saved_at, 1_792_520_631);
        assert_eq!(options.editing_minutes, 3);
        let written =
            String::from_utf8_lossy(&amipro_formats::sam_document(&app.document, &options))
                .into_owned();
        let created = app.document.info().created.unwrap();
        assert!(written.contains(&format!(
            "[desc]\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t1792520631\r\n\t1\r\n\t{created}\r\n\t3\r\n"
        )));
    }

    #[test]
    fn untitled_names_reuse_only_names_not_currently_open() {
        let mut second = Document::default();
        second.title = "Untitled 2".into();
        let sessions = vec![
            DocumentSession::new(
                1,
                Document::default(),
                DocumentLocation::Untitled,
                "Ready".into(),
            ),
            DocumentSession::new(2, second, DocumentLocation::Untitled, "Ready".into()),
        ];

        assert_eq!(next_untitled_title(&sessions), "Untitled 3");
        assert_eq!(next_untitled_title(&sessions[1..]), "Untitled");
    }

    fn root_layout(app: &AmiProApp, size: cosmic::iced::Size) -> layout::Node {
        let renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
            cosmic::iced::Font::default(),
            cosmic::iced::Pixels(14.0),
        ));
        let mut root = cosmic::Application::view(app);
        let mut tree = Tree::new(&root);
        root.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(cosmic::iced::Size::ZERO, size),
        )
    }

    fn send_message(app: &mut AmiProApp, message: Message) {
        drop(cosmic::Application::update(app, message));
    }

    /// The action bar's row of groups in the laid-out window: below the
    /// action-bar node, the first node with several children.
    fn action_row(root: &layout::Node) -> layout::Node {
        let mut node = root.children()[1].clone();
        while node.children().len() == 1 {
            node = node.children()[0].clone();
        }
        node
    }

    #[test]
    fn menus_with_a_check_line_up_every_label() {
        // User, 2026-10-12: View › Styles Panel looked indented because only
        // it had the check column.
        let app = test_app(Document::default(), DocumentLocation::Untitled);
        for (root, layout) in MENU_LAYOUTS {
            let entries = menu_entries(&app, layout);
            let checks: Vec<Option<bool>> = entries
                .iter()
                .filter_map(|entry| match entry {
                    MenuEntry::Item { checked, .. } => Some(*checked),
                    // A submenu's label takes the check column's space.
                    MenuEntry::Folder { indented, .. } => Some(indented.then_some(false)),
                    MenuEntry::Divider => None,
                })
                .collect();
            let any = checks.iter().any(Option::is_some);
            assert!(
                checks.iter().all(|checked| checked.is_some() == any),
                "{root:?}: every label lines up"
            );
            if matches!(root, command::MenuRoot::View | command::MenuRoot::Window) {
                assert!(any, "{root:?} has a check");
            }
            if matches!(root, command::MenuRoot::File | command::MenuRoot::Edit) {
                assert!(!any, "{root:?} has none");
            }
        }
    }

    #[test]
    fn view_toolbar_hides_the_bar_and_is_checked_in_the_view_menu() {
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        let size = cosmic::iced::Size::new(1280.0, 800.0);
        let view_menu = |app: &AmiProApp| {
            let (_, layout) = MENU_LAYOUTS
                .iter()
                .find(|(root, _)| *root == command::MenuRoot::View)
                .unwrap();
            menu_entries(app, layout)
                .into_iter()
                .find_map(|entry| match entry {
                    MenuEntry::Item {
                        command: MenuCommand::Command(CommandId::ShowToolbar),
                        checked,
                        ..
                    } => checked,
                    _ => None,
                })
        };
        assert_eq!(view_menu(&app), Some(true));
        assert_eq!(root_layout(&app, size).children().len(), 5);
        send_message(&mut app, Message::Command(CommandId::ShowToolbar));
        assert_eq!(view_menu(&app), Some(false));
        let root = root_layout(&app, size);
        let rows: Vec<_> = root.children().iter().map(layout::Node::bounds).collect();
        let [menu, tabs, _, _] = rows[..] else {
            panic!("expected menu, tabs, document row, and status bar: {rows:?}");
        };
        assert_eq!(tabs.y, menu.height);
        assert_eq!(tabs.height, TAB_STRIP_HEIGHT);
        // Customize Toolbar shows the bar again, so changes can be seen.
        send_message(&mut app, Message::Command(CommandId::CustomizeToolbar));
        assert!(app.preferences.toolbar_visible);
    }

    #[test]
    fn customize_toolbar_changes_the_bar_and_the_next_start_keeps_it() {
        let directory =
            std::env::temp_dir().join(format!("amipro-toolbar-settings-{}", std::process::id()));
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        let (settings, preferences, styles_visible) = Settings::load(config());
        assert_eq!(preferences, Preferences::default());
        assert_eq!(styles_visible, None);
        app.settings = Some(settings);
        app.styles_visible = true;

        send_message(&mut app, Message::Command(CommandId::CustomizeToolbar));
        assert!(app.toolbar_editor.is_some());
        let save_as = ToolbarItem::Command(CommandId::SaveAs);
        for edit in [
            ToolbarEdit::SelectChoice(save_as),
            ToolbarEdit::SelectItem(2),
            ToolbarEdit::Add,
        ] {
            send_message(&mut app, Message::Toolbar(edit));
        }
        assert_eq!(app.preferences.toolbar[3], save_as);
        // The dialog lays out with the change.
        let renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
            cosmic::iced::Font::default(),
            cosmic::iced::Pixels(14.0),
        ));
        let mut dialog = cosmic::Application::dialog(&app).expect("the dialog is open");
        let mut tree = Tree::new(&dialog);
        let node = dialog.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(
                cosmic::iced::Size::ZERO,
                cosmic::iced::Size::new(1280.0, 800.0),
            ),
        );
        assert!(node.bounds().width >= 700.0, "{:?}", node.bounds());
        drop(dialog);
        // The bar shows the new button.
        let groups = action_bar_layout(&app.preferences.toolbar, 1280.0).visible_groups;
        assert_eq!(groups[0].last(), Some(&save_as));
        send_message(&mut app, Message::CloseToolbarEditor);
        assert!(app.toolbar_editor.is_none());
        send_message(&mut app, Message::Command(CommandId::ShowToolbar));
        send_message(&mut app, Message::Command(CommandId::StylesPanel));

        // The next start reads what was saved.
        let (_, preferences, styles_visible) = Settings::load(config());
        assert_eq!(preferences.toolbar, app.preferences.toolbar);
        assert!(!preferences.toolbar_visible);
        assert_eq!(styles_visible, Some(false));
        // View › Ruler is saved the same way (`UI_SPEC.md` §19).
        #[cfg(feature = "page-surface")]
        {
            assert!(preferences.ruler_visible);
            send_message(&mut app, Message::Command(CommandId::Ruler));
            let (_, preferences, _) = Settings::load(config());
            assert!(!preferences.ruler_visible);
            // So is View › Status Bar (shown by default).
            assert!(preferences.status_bar_visible);
            send_message(&mut app, Message::Command(CommandId::StatusBar));
            let (_, preferences, _) = Settings::load(config());
            assert!(!preferences.status_bar_visible);
        }

        // Reset to Standard is saved too.
        send_message(&mut app, Message::Command(CommandId::CustomizeToolbar));
        send_message(&mut app, Message::Toolbar(ToolbarEdit::Reset));
        let (_, preferences, _) = Settings::load(config());
        assert_eq!(preferences.toolbar, chrome::standard_toolbar());
        assert!(preferences.toolbar_visible);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// View › Show ▸ Table row/column headings (TD.7, TN9): off at first,
    /// toggled from the menu or the View preferences, kept for the next
    /// start; its check mark follows.
    #[cfg(feature = "page-surface")]
    #[test]
    fn the_table_headings_preference_is_kept_for_the_next_start_td7() {
        use preferences_dialog::{PreferencesAction as A, ViewSetting};
        let directory =
            std::env::temp_dir().join(format!("amipro-headings-settings-{}", std::process::id()));
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        let (settings, preferences, _) = Settings::load(config());
        assert!(!preferences.table_addresses, "off by default");
        app.settings = Some(settings);
        assert_eq!(menu_check(&app, CommandId::TableAddresses), Some(false));
        send_message(&mut app, Message::Command(CommandId::TableAddresses));
        assert!(app.preferences.table_addresses);
        assert_eq!(menu_check(&app, CommandId::TableAddresses), Some(true));
        let (_, preferences, _) = Settings::load(config());
        assert!(preferences.table_addresses, "kept for the next start");
        // The View preferences switch the same setting.
        send_message(&mut app, Message::Command(CommandId::Preferences));
        send_message(
            &mut app,
            Message::Preferences(A::View(ViewSetting::TableAddresses, false)),
        );
        assert!(!app.preferences.table_addresses);
        let (_, preferences, _) = Settings::load(config());
        assert!(!preferences.table_addresses);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// The last frame made and the default for new frames outlive the
    /// session, as Ami Pro keeps them in AMIPRO.INI (`frmcrt`, `FrDefs`;
    /// decisions F1, F2).
    #[cfg(feature = "page-surface")]
    #[test]
    fn the_last_frame_and_the_frame_default_are_saved_for_the_next_start() {
        use amipro_editing::frame_creation::{FrameDefaults, FrameGeometry};
        let directory =
            std::env::temp_dir().join(format!("amipro-frame-settings-{}", std::process::id()));
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(
            Document::from_plain_text("One.\nTwo."),
            DocumentLocation::Untitled,
        );
        let (settings, preferences, _) = Settings::load(config());
        assert_eq!(preferences.last_frame, None);
        assert_eq!(preferences.frame_defaults, FrameDefaults::AMI_PRO);
        app.settings = Some(settings);
        send_message(&mut app, Message::Command(CommandId::Frame));
        send_message(
            &mut app,
            Message::CreateFrame(page_dialogs::CreateFrameAction::Create),
        );
        let frame = *app.document.frame_stack().last().unwrap();
        let mut style = app.document.frame(frame).unwrap().style.clone();
        style.opaque = false;
        style.corners = 40;
        assert!(
            app.execute_page_command(&amipro_editing::Command::SetFrameStyle {
                frame,
                style: style.clone(),
            })
        );
        send_message(&mut app, Message::NewFrameDefault(NewFrameDefault::Make));
        let (_, preferences, _) = Settings::load(config());
        assert_eq!(preferences.last_frame, Some(FrameGeometry::FIRST));
        assert_eq!(
            preferences.frame_defaults,
            FrameDefaults::from_style(&style)
        );
        send_message(&mut app, Message::NewFrameDefault(NewFrameDefault::Restore));
        let (_, preferences, _) = Settings::load(config());
        assert_eq!(preferences.frame_defaults, FrameDefaults::AMI_PRO);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// Insert › Bullet…'s last bullet in the settings (decision B4).
    #[cfg(feature = "page-surface")]
    #[test]
    fn the_last_bullet_is_saved_for_the_next_start() {
        let directory =
            std::env::temp_dir().join(format!("amipro-bullet-settings-{}", std::process::id()));
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(
            Document::from_plain_text("One.\nTwo."),
            DocumentLocation::Untitled,
        );
        let (settings, preferences, _) = Settings::load(config());
        assert_eq!(preferences.last_bullet, None);
        app.settings = Some(settings);
        send_message(&mut app, Message::Command(CommandId::InsertBullet));
        send_message(
            &mut app,
            Message::InsertBullet(page_dialogs::BulletAction::Choose(14)),
        );
        send_message(
            &mut app,
            Message::InsertBullet(page_dialogs::BulletAction::Insert),
        );
        let (_, preferences, _) = Settings::load(config());
        assert_eq!(preferences.last_bullet, Some('\u{D}'));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// Tools › Preferences… (CM6): every value set in the dialog is kept for
    /// the next start, in a scratch configuration.
    #[test]
    fn preferences_set_in_the_dialog_are_saved_for_the_next_start() {
        use preferences_dialog::{Page, PreferencesAction as A, ViewSetting};
        let directory =
            std::env::temp_dir().join(format!("amipro-preferences-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(
            Document::from_plain_text("One."),
            DocumentLocation::Untitled,
        );
        let (settings, preferences, _) = Settings::load(config());
        assert_eq!(preferences, Preferences::default());
        app.settings = Some(settings);
        send_message(&mut app, Message::Command(CommandId::Preferences));
        assert!(cosmic::Application::dialog(&app).is_some());
        let folder = directory.join("sheets");
        for action in [
            A::UndoLevels("25".into()),
            A::RecentFiles("2".into()),
            A::KeepBackup(false),
            A::Page(Page::Folders),
            A::FolderChosen(Some(folder.clone())),
            A::FolderChosen(Some(folder.clone())),
            A::DocumentsChosen(Some(folder.clone())),
            A::Page(Page::View),
            A::View(ViewSetting::Toolbar, false),
        ] {
            send_message(&mut app, Message::Preferences(action));
        }
        // A number outside its range is kept as typed but not applied.
        send_message(&mut app, Message::Preferences(A::UndoLevels("0".into())));
        assert_eq!(app.preferences_dialog.as_ref().unwrap().undo_levels, "0");
        assert_eq!(app.history.limit(), 25);
        let (_, saved, _) = Settings::load(config());
        assert_eq!(saved.undo_levels, 25);
        assert_eq!(saved.recent_count, 2);
        assert!(!saved.keep_backup);
        assert_eq!(saved.style_folders, vec![folder.clone()]);
        assert_eq!(saved.documents_folder, Some(folder.clone()));
        assert!(!saved.toolbar_visible);
        // A chooser that was cancelled keeps the folder; Clear unsets it.
        send_message(&mut app, Message::Preferences(A::DocumentsChosen(None)));
        assert_eq!(app.preferences.documents_folder, Some(folder.clone()));
        // A folder that no longer exists is kept but unused.
        assert_eq!(app.start_folder(), None);
        send_message(&mut app, Message::Preferences(A::ClearDocuments));
        let (_, saved, _) = Settings::load(config());
        assert_eq!(saved.documents_folder, None);
        send_message(&mut app, Message::Preferences(A::RemoveFolder(0)));
        send_message(&mut app, Message::Preferences(A::Close));
        assert!(app.preferences_dialog.is_none());
        let (_, saved, _) = Settings::load(config());
        assert!(saved.style_folders.is_empty());
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// Recent colours in the settings (`UI_SPEC.md` §23, C4): a colour
    /// chosen in one session is offered in the next.
    #[cfg(feature = "page-surface")]
    #[test]
    fn recent_colours_are_saved_for_the_next_start() {
        use colour_control::{ColourAction, Target};
        let directory =
            std::env::temp_dir().join(format!("amipro-colour-settings-{}", std::process::id()));
        let config = || {
            cosmic::cosmic_config::Config::with_custom_path(
                APP_ID,
                SETTINGS_VERSION,
                directory.clone(),
            )
            .unwrap()
        };
        let mut app = test_app(
            Document::from_plain_text("One.\nTwo."),
            DocumentLocation::Untitled,
        );
        let (settings, preferences, _) = Settings::load(config());
        assert!(preferences.recent_colours.is_empty());
        app.settings = Some(settings);
        send_message(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
        send_message(
            &mut app,
            Message::Colour(ColourAction::Toggle(Target::StyleText, 0)),
        );
        send_message(&mut app, Message::Colour(ColourAction::Choose(0x0099_6633)));
        let (_, preferences, _) = Settings::load(config());
        assert_eq!(preferences.recent_colours, [0x0099_6633]);
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// The user saw "Restore Ami Pro's Default" cut off in the rail
    /// (2026-10-17): a rail button wraps a label too long for its width,
    /// growing taller, and never lays out wider than the rail gives it.
    #[cfg(feature = "page-surface")]
    #[test]
    fn rail_buttons_wrap_their_labels_rather_than_cut_them_off() {
        let renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
            cosmic::iced::Font::default(),
            cosmic::iced::Pixels(14.0),
        ));
        let laid_out = |label: &'static str, width: f32| {
            let mut button = rail_button(label, Some(Message::Repaint));
            let mut tree = Tree::new(&button);
            button
                .as_widget_mut()
                .layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(
                        cosmic::iced::Size::ZERO,
                        cosmic::iced::Size::new(width, 400.0),
                    ),
                )
                .bounds()
        };
        let one_line = laid_out("Make Default", 200.0);
        let wrapped = laid_out("Restore Ami Pro's Default", 90.0);
        assert!(wrapped.width <= 90.0, "{wrapped:?}");
        assert!(
            wrapped.height > one_line.height + 10.0,
            "{wrapped:?} against {one_line:?}"
        );
    }

    #[test]
    fn the_action_bar_widths_match_the_real_widgets() {
        // chrome::bar_width predicts the space the bar needs; at exactly
        // that width the real widgets end inside the bar, with and without
        // More (the user's report, 2026-10-12: More on a mid-size window).
        let app = test_app(Document::default(), DocumentLocation::Untitled);
        let groups = chrome::toolbar_groups(&chrome::standard_toolbar());
        let needed = chrome::bar_width(&groups, chrome::WIDE_STYLE_FIELD, false);
        let root = root_layout(&app, cosmic::iced::Size::new(needed, 800.0));
        let row = action_row(&root);
        assert_eq!(row.children().len(), 2 * groups.len() - 1, "no More");
        let last = row.children().last().unwrap().bounds();
        // Child bounds are relative to the row inside the bar's padding.
        // Neither short (More too late) nor generous (More too early).
        assert!(
            (last.x + last.width - (needed - 2.0 * chrome::BAR_PADDING)).abs() <= 2.0,
            "{last:?} in {needed}"
        );
        // Narrow enough for More: everything shown still fits.
        let narrow = chrome::bar_width(&groups, chrome::COMPACT_STYLE_FIELD, false) - 1.0;
        let layout = chrome::action_bar_layout(&chrome::standard_toolbar(), narrow);
        assert!(!layout.overflow_commands.is_empty());
        let root = root_layout(&app, cosmic::iced::Size::new(narrow, 800.0));
        let row = action_row(&root);
        assert_eq!(row.children().len(), 2 * layout.visible_groups.len() + 1);
        let last = row.children().last().unwrap().bounds();
        let predicted = chrome::bar_width(&layout.visible_groups, layout.style_width, true);
        assert!(
            (last.x + last.width - (predicted - 2.0 * chrome::BAR_PADDING)).abs() <= 2.0,
            "{last:?}, predicted {predicted}"
        );
        assert!(predicted <= narrow);
        // A mid-size window shows every group.
        let root = root_layout(&app, cosmic::iced::Size::new(1_000.0, 800.0));
        assert_eq!(action_row(&root).children().len(), 2 * groups.len() - 1);
    }

    #[test]
    fn the_rail_docks_at_its_width_and_overlays_narrow_windows() {
        // UI_SPEC.md §4–5, gaps R1, R5, R6: a 216-pixel rail beside the
        // workspace on wide windows; on narrow ones it starts closed and
        // opens over the workspace, which keeps its full width.
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        app.styles_visible = true;
        let size = cosmic::iced::Size::new(1280.0, 800.0);
        let document_row = |app: &AmiProApp, size| root_layout(app, size).children()[3].clone();
        // The row is always the docked slot, then the workspace stacked
        // under the floating slot, so the workspace keeps its place in the
        // tree (and its scroll).
        let docked = document_row(&app, size);
        let [rail, stack] = docked.children() else {
            panic!("a docked rail and the workspace");
        };
        assert_eq!(rail.bounds().width, RAIL_WIDTH);
        assert_eq!(stack.bounds().x, RAIL_WIDTH);
        assert_eq!(stack.children()[0].bounds().width, stack.bounds().width);
        assert_eq!(stack.children()[1].bounds().width, 0.0);

        send_message(&mut app, Message::WindowResized(900.0));
        let narrow = cosmic::iced::Size::new(900.0, 640.0);
        assert!(
            !app.rail_open(),
            "a narrow window starts with the rail closed"
        );
        let closed = document_row(&app, narrow);
        let [slot, stack] = closed.children() else {
            panic!("the empty slot and the workspace");
        };
        assert_eq!(slot.bounds().width, 0.0);
        let full = stack.bounds().width;
        assert_eq!(stack.children()[0].bounds().width, full);
        send_message(&mut app, Message::Command(CommandId::StylesPanel));
        assert!(app.rail_open());
        assert!(
            app.styles_visible,
            "the docked choice is kept for wide windows"
        );
        let overlay = document_row(&app, narrow);
        let [slot, stack] = overlay.children() else {
            panic!("the empty slot and the stacked workspace");
        };
        assert_eq!(slot.bounds().width, 0.0);
        assert_eq!(stack.bounds().width, full, "the workspace is not squeezed");
        assert_eq!(stack.children()[0].bounds().width, full);
        assert_eq!(stack.children()[1].children()[0].bounds().width, RAIL_WIDTH);
        // Widening again shows the docked rail as the user left it.
        send_message(&mut app, Message::WindowResized(1280.0));
        assert!(app.rail_open());
        send_message(&mut app, Message::Command(CommandId::StylesPanel));
        assert!(!app.styles_visible);
    }

    #[test]
    fn the_styles_filter_keeps_matching_names_in_order() {
        let document = Document::with_paragraphs([("One", StyleId(1)), ("Two", StyleId(2))]);
        let names = |filter: &str| -> Vec<String> {
            filtered_styles(&document, filter)
                .into_iter()
                .map(|style| style.name.clone())
                .collect()
        };
        let all = names("");
        assert!(all.len() >= 2);
        assert_eq!(names("  "), all);
        let heading: Vec<String> = all
            .iter()
            .filter(|name| name.to_lowercase().contains("head"))
            .cloned()
            .collect();
        assert_eq!(names("HEAD"), heading);
        assert!(names("no such style").is_empty());
    }

    #[test]
    fn chrome_rows_keep_fixed_heights_and_the_document_row_takes_the_rest() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Body text on a real page".into();
        let app = test_app(document, DocumentLocation::Untitled);

        for size in [
            cosmic::iced::Size::new(1280.0, 800.0),
            cosmic::iced::Size::new(1920.0, 1080.0),
        ] {
            let root = root_layout(&app, size);
            let rows: Vec<_> = root.children().iter().map(layout::Node::bounds).collect();
            let [menu, actions, tabs, document_row, status] = rows[..] else {
                panic!("expected menu, action bar, tabs, document row, and status bar");
            };

            assert_eq!(actions.height, ACTION_BAR_HEIGHT, "action bar at {size:?}");
            assert_eq!(tabs.height, TAB_STRIP_HEIGHT, "tab strip at {size:?}");
            assert_eq!(status.height, STATUS_BAR_HEIGHT, "status bar at {size:?}");
            assert!(menu.height <= 40.0, "menu row at {size:?}: {}", menu.height);
            assert!(
                (status.y + status.height - size.height).abs() < 0.5,
                "status bar must sit at the window bottom at {size:?}"
            );
            let chrome = menu.height + actions.height + tabs.height + status.height;
            assert!(
                (document_row.height - (size.height - chrome)).abs() < 0.5,
                "document row must take all remaining height at {size:?}: {document_row:?}"
            );
            assert_eq!(
                (menu.x, actions.x, tabs.x),
                (0.0, 0.0, 0.0),
                "menu, action bar, and tabs share one left edge without an outer frame"
            );
        }
    }

    fn tab_labels(app: &AmiProApp) -> Vec<String> {
        app.tab_model
            .iter()
            .map(|entity| app.tab_model.text(entity).unwrap_or_default().to_owned())
            .collect()
    }

    fn active_tab_label(app: &AmiProApp) -> String {
        app.tab_model
            .text(app.tab_model.active())
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn native_tabs_mirror_sessions_through_the_real_message_path() {
        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        drop(cosmic::Application::update(
            &mut app,
            Message::Command(CommandId::NewDocument),
        ));
        drop(cosmic::Application::update(
            &mut app,
            Message::Command(CommandId::NewDocument),
        ));
        assert_eq!(tab_labels(&app), ["Untitled", "Untitled 2", "Untitled 3"]);
        assert_eq!(active_tab_label(&app), "Untitled 3");

        let first = app.tab_model.iter().next().unwrap();
        drop(cosmic::Application::update(
            &mut app,
            Message::TabActivated(first),
        ));
        assert_eq!(app.active_session, 0);
        assert!(app.tab_model.is_active(first));

        drop(cosmic::Application::update(
            &mut app,
            Message::Edit(text_editor::Action::Edit(text_editor::Edit::Insert('x'))),
        ));
        assert_eq!(
            tab_labels(&app)[0],
            "Untitled •",
            "dirty mark reaches the tab"
        );

        drop(cosmic::Application::update(
            &mut app,
            Message::TabCloseRequested(first),
        ));
        assert_eq!(app.pending_close, Some(0), "dirty close must prompt first");
        assert_eq!(app.tab_model.len(), 3);

        drop(cosmic::Application::update(
            &mut app,
            Message::ConfirmCloseDiscard,
        ));
        assert_eq!(tab_labels(&app), ["Untitled 2", "Untitled 3"]);
        assert_eq!(active_tab_label(&app), app.display_name());

        let active_before = app.active_session;
        drop(cosmic::Application::update(
            &mut app,
            Message::TabActivated(first),
        ));
        assert_eq!(
            app.active_session, active_before,
            "a removed tab entity must be ignored"
        );

        let last = app.tab_model.iter().last().unwrap();
        drop(cosmic::Application::update(
            &mut app,
            Message::TabCloseRequested(last),
        ));
        assert_eq!(
            tab_labels(&app),
            ["Untitled 2"],
            "clean close needs no prompt"
        );
        assert_eq!(app.pending_close, None);
    }

    #[test]
    fn imported_and_native_tabs_use_file_names_in_session_order() {
        let mut app = test_app(
            Document::default(),
            DocumentLocation::LegacySource(PathBuf::from("/private/README31.SAM")),
        );
        app.sessions.push(DocumentSession::new(
            7,
            Document::default(),
            DocumentLocation::Native(PathBuf::from("/tmp/a-much-longer-native-name.amipro")),
            "Ready".into(),
        ));
        app.active_session = 1;
        app.sync_tab_model();

        assert_eq!(
            tab_labels(&app),
            ["README31.SAM", "a-much-longer-native-name.amipro"]
        );
        assert_eq!(active_tab_label(&app), "a-much-longer-native-name.amipro");
        assert!(
            app.tab_model
                .iter()
                .all(|entity| app.tab_model.is_closable(entity))
        );
    }

    #[test]
    fn new_document_button_follows_tabs_and_stays_reachable_when_they_overflow() {
        let padding = f32::from(CHROME_HORIZONTAL_PADDING);
        let tab_strip_parts = |app: &AmiProApp, window: cosmic::iced::Size| {
            let root = root_layout(app, window);
            let layout = cosmic::iced::advanced::Layout::new(&root);
            let strip = layout.children().nth(2).expect("tab strip row");
            let row = strip.children().next().expect("responsive tab row");
            let parts: Vec<_> = row.children().map(|part| part.bounds()).collect();
            (parts[0], parts[1])
        };

        let mut app = test_app(Document::default(), DocumentLocation::Untitled);
        let (tabs, new_document) = tab_strip_parts(&app, cosmic::iced::Size::new(1280.0, 800.0));
        assert!(
            new_document.x <= padding + 240.0 + 4.0 + 0.5,
            "with one tab, New Document follows it: {tabs:?} {new_document:?}"
        );

        for _ in 0..15 {
            drop(cosmic::Application::update(
                &mut app,
                Message::Command(CommandId::NewDocument),
            ));
        }
        // Collapsed tab bars round to whole tabs, so sweep widths rather than
        // relying on one width that happens to leave room.
        for width in (900..=1920).step_by(7) {
            let window = cosmic::iced::Size::new(width as f32, 800.0);
            let (tabs, new_document) = tab_strip_parts(&app, window);
            assert!(
                new_document.x + new_document.width <= window.width - padding + 0.5,
                "New Document must stay inside a {width}px window: {new_document:?}"
            );
            assert!(
                tabs.x + tabs.width <= new_document.x,
                "overflowing tabs must not cover New Document at {width}px: {tabs:?} {new_document:?}"
            );
            assert_eq!((new_document.width, new_document.height), (32.0, 32.0));
        }
    }

    #[test]
    fn menu_and_keyboard_routes_use_the_same_command_message() {
        assert!(matches!(
            MenuCommand::Command(CommandId::Cut).message(),
            Message::Command(CommandId::Cut)
        ));

        let binding = standard_editor_key_binding(text_editor::KeyPress {
            key: keyboard::Key::Character("ч".into()),
            modified_key: keyboard::Key::Character("ч".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyX),
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            status: text_editor::Status::Focused { is_hovered: true },
        });
        assert!(matches!(
            binding,
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Cut
            )))
        ));
    }

    #[test]
    fn forward_document_shortcut_routes_through_focused_default_editor() {
        let binding = standard_editor_key_binding(text_editor::KeyPress {
            key: keyboard::Key::Named(keyboard::key::Named::PageDown),
            modified_key: keyboard::Key::Named(keyboard::key::Named::PageDown),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::PageDown),
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            status: text_editor::Status::Focused { is_hovered: true },
        });
        assert!(matches!(
            binding,
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::NextDocument
            )))
        ));
    }

    #[test]
    fn command_dispatch_cuts_pastes_and_restores_history_in_the_active_document() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "alpha βeta".into();
        let mut app = test_app(document, DocumentLocation::Untitled);
        select_first_five_bytes(&mut app);

        assert_eq!(app.selected_text().as_deref(), Some("alpha"));
        drop(app.dispatch_command(CommandId::Cut));
        assert_eq!(app.document.plain_text(), " βeta");
        assert_eq!(app.history.undo_len(), 1);

        drop(app.dispatch_command(CommandId::Undo));
        assert_eq!(app.document.plain_text(), "alpha βeta");
        drop(app.dispatch_command(CommandId::Redo));
        assert_eq!(app.document.plain_text(), " βeta");

        let session_id = app.id;
        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::ClipboardRead {
                command: CommandId::Paste,
                session_id,
                contents: Some("Ω".into()),
            },
        ));
        assert_eq!(app.document.plain_text(), "Ω βeta");
        assert_eq!(app.history.undo_len(), 2);
    }

    #[test]
    fn clipboard_completion_cannot_modify_a_different_active_tab() {
        let mut first = Document::default();
        first.paragraph_mut(0).text = "first".into();
        let mut second = Document::default();
        second.paragraph_mut(0).text = "second".into();
        let mut app = test_app(first, DocumentLocation::Untitled);
        let first_id = app.id;
        app.sessions.push(DocumentSession::new(
            2,
            second,
            DocumentLocation::Untitled,
            "Second".into(),
        ));
        app.active_session = 1;

        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::ClipboardRead {
                command: CommandId::Paste,
                session_id: first_id,
                contents: Some("wrong tab".into()),
            },
        ));

        assert_eq!(app.sessions[0].document.plain_text(), "first");
        assert_eq!(app.sessions[1].document.plain_text(), "second");
        assert!(
            app.sessions
                .iter()
                .all(|session| !session.history.can_undo())
        );
    }

    #[test]
    fn quit_and_close_commands_preserve_dirty_document_confirmation() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "unsaved".into();
        let mut app = test_app(document, DocumentLocation::Untitled);
        app.dirty = true;

        drop(app.dispatch_command(CommandId::CloseDocument));
        assert_eq!(app.pending_close, Some(0));
        assert!(!app.quit_requested);
        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::CancelClose,
        ));

        drop(app.dispatch_command(CommandId::Quit));
        assert_eq!(app.pending_close, Some(0));
        assert!(app.quit_requested);
        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::CancelClose,
        ));
        assert_eq!(app.pending_close, None);
        assert!(!app.quit_requested);
        assert_eq!(app.document.plain_text(), "unsaved");
    }

    #[test]
    fn document_sessions_keep_independent_content_history_location_and_scroll() {
        let mut first_document = Document::default();
        first_document.title = "First".into();
        first_document.paragraph_mut(0).text = "alpha".into();
        let mut second_document = Document::default();
        second_document.title = "Second".into();
        second_document.paragraph_mut(0).text = "beta".into();
        let mut sessions = vec![
            DocumentSession::new(
                1,
                first_document,
                DocumentLocation::Native(PathBuf::from("/tmp/first.amipro")),
                "First ready".into(),
            ),
            DocumentSession::new(
                2,
                second_document,
                DocumentLocation::LegacySource(PathBuf::from("/tmp/second.SAM")),
                "Second ready".into(),
            ),
        ];
        sessions[0].push_undo();
        sessions[0].dirty = true;
        sessions[0].editor.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 0, column: 4 },
            selection: Some(text_editor::Position { line: 0, column: 1 }),
        });
        sessions[1].editor.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 0, column: 2 },
            selection: None,
        });
        sessions[0].scroll_offset =
            cosmic::iced::widget::scrollable::AbsoluteOffset { x: 12.0, y: 340.0 };

        assert_eq!(sessions[0].document.plain_text(), "alpha");
        assert_eq!(sessions[1].document.plain_text(), "beta");
        assert_eq!(sessions[0].history.undo_len(), 1);
        assert!(!sessions[1].history.can_undo());
        assert_eq!(sessions[0].editor.cursor().position.column, 4);
        assert_eq!(sessions[0].editor.cursor().selection.unwrap().column, 1);
        assert_eq!(sessions[1].editor.cursor().position.column, 2);
        assert!(sessions[1].editor.cursor().selection.is_none());
        assert_ne!(sessions[0].location, sessions[1].location);
        assert!(sessions[0].dirty);
        assert!(!sessions[1].dirty);
        assert_eq!(sessions[0].scroll_offset.y, 340.0);
        assert_eq!(sessions[1].scroll_offset.y, 0.0);
        assert_eq!(
            session_index_for_path(&sessions, std::path::Path::new("/tmp/first.amipro")),
            Some(0)
        );
        assert_eq!(
            session_index_for_path(&sessions, std::path::Path::new("/tmp/second.SAM")),
            Some(1)
        );
        assert_eq!(sessions[0].tab_label(), "first.amipro •");
        assert_eq!(sessions[1].tab_label(), "second.SAM");
    }

    #[test]
    #[cfg(feature = "rich-editor-prototype")]
    fn document_session_layout_generation_tracks_edits_styles_undo_and_redo_not_view_state() {
        let original = Document::default();
        let mut session = DocumentSession::new(
            7,
            original.clone(),
            DocumentLocation::Untitled,
            "Ready".into(),
        );
        assert_eq!(session.rich_layout_generation, 0);

        session.prototype_selection = PrototypeSelection::caret(0);
        session.scroll_offset =
            cosmic::iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 80.0 };
        assert_eq!(session.rich_layout_generation, 0);

        session.document.paragraph_mut(0).text = "edited".into();
        let edited = session.document.clone();
        session.refresh_rich_layout();
        assert_eq!(session.rich_layout_generation, 1);

        session.document.paragraph_mut(0).style = StyleId(2);
        session.refresh_rich_layout();
        assert_eq!(session.rich_layout_generation, 2);

        session.document = original;
        session.refresh_rich_layout();
        assert_eq!(session.rich_layout_generation, 3, "undo replacement");

        session.document = edited;
        session.refresh_rich_layout();
        assert_eq!(session.rich_layout_generation, 4, "redo replacement");
    }

    fn cursor(position_line: usize, selection_line: Option<usize>) -> text_editor::Cursor {
        text_editor::Cursor {
            position: text_editor::Position {
                line: position_line,
                column: 0,
            },
            selection: selection_line.map(|line| text_editor::Position { line, column: 0 }),
        }
    }

    #[test]
    fn selected_character_trait_respects_direct_overrides() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "bold plain".into();
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        let paragraph = &document.paragraphs()[0];
        let style = &document.styles()[&StyleId(1)];

        assert!(range_has_character_trait(
            paragraph,
            style,
            0,
            4,
            CharacterAttribute::Bold
        ));
        assert!(!range_has_character_trait(
            paragraph,
            style,
            0,
            paragraph.text.len(),
            CharacterAttribute::Bold
        ));
    }

    #[test]
    fn action_bar_character_state_distinguishes_on_off_mixed_and_unavailable() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "bold plain".into();
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::default()
            },
        });
        let selected = |start, end| text_editor::Cursor {
            position: text_editor::Position {
                line: 0,
                column: end,
            },
            selection: Some(text_editor::Position {
                line: 0,
                column: start,
            }),
        };

        assert_eq!(
            selected_character_toggle_state(&document, selected(0, 4), CharacterAttribute::Bold),
            ToggleState::On
        );
        assert_eq!(
            selected_character_toggle_state(&document, selected(5, 10), CharacterAttribute::Bold),
            ToggleState::Off
        );
        assert_eq!(
            selected_character_toggle_state(&document, selected(0, 10), CharacterAttribute::Bold),
            ToggleState::Mixed
        );
        assert_eq!(
            selected_character_toggle_state(
                &document,
                text_editor::Cursor {
                    position: text_editor::Position { line: 0, column: 2 },
                    selection: None,
                },
                CharacterAttribute::Bold,
            ),
            ToggleState::Unavailable
        );
    }

    #[test]
    fn style_state_is_shared_for_uniform_selection_and_mixed_otherwise() {
        let mut document = Document::default();
        document.push_paragraph("Second", StyleId(1));

        assert_eq!(
            selected_paragraph_style(&document, cursor(1, Some(0))),
            Some(StyleId(1))
        );

        document.paragraph_mut(1).style = StyleId(2);
        assert_eq!(
            selected_paragraph_style(&document, cursor(1, Some(0))),
            None
        );
        assert_eq!(
            selected_paragraph_style(&document, cursor(1, None)),
            Some(StyleId(2))
        );
    }

    #[test]
    fn selected_paragraph_range_is_order_independent_and_bounded() {
        assert_eq!(selected_paragraph_range(3, cursor(0, Some(2))), Some(0..=2));
        assert_eq!(
            selected_paragraph_range(3, cursor(99, Some(1))),
            Some(1..=2)
        );
        assert_eq!(selected_paragraph_range(0, cursor(0, None)), None);
    }

    #[test]
    fn baseline_navigation_does_not_trigger_document_reconciliation() {
        assert!(!standard_action_changes_document(
            &text_editor::Action::Move(text_editor::Motion::Right,)
        ));
        assert!(!standard_action_changes_document(
            &text_editor::Action::Select(text_editor::Motion::Right),
        ));
        assert!(standard_action_changes_document(
            &text_editor::Action::Edit(text_editor::Edit::Insert('x'),)
        ));
    }

    #[test]
    fn baseline_editor_routes_undo_and_redo_shortcuts_to_document_history() {
        let key_press =
            |character: &'static str,
             code: cosmic::iced::keyboard::key::Code,
             modifiers: cosmic::iced::keyboard::Modifiers| text_editor::KeyPress {
                key: cosmic::iced::keyboard::Key::Character(character.into()),
                modified_key: cosmic::iced::keyboard::Key::Character(character.into()),
                physical_key: cosmic::iced::keyboard::key::Physical::Code(code),
                modifiers,
                text: None,
                status: text_editor::Status::Focused { is_hovered: true },
            };

        assert!(matches!(
            standard_editor_key_binding(key_press(
                "z",
                cosmic::iced::keyboard::key::Code::KeyZ,
                cosmic::iced::keyboard::Modifiers::CTRL,
            )),
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Undo
            )))
        ));
        assert!(matches!(
            standard_editor_key_binding(key_press(
                "z",
                cosmic::iced::keyboard::key::Code::KeyZ,
                cosmic::iced::keyboard::Modifiers::CTRL | cosmic::iced::keyboard::Modifiers::SHIFT,
            )),
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Redo
            )))
        ));

        assert!(matches!(
            standard_editor_key_binding(key_press(
                "c",
                cosmic::iced::keyboard::key::Code::KeyC,
                cosmic::iced::keyboard::Modifiers::CTRL,
            )),
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Copy
            )))
        ));
        assert!(matches!(
            standard_editor_key_binding(key_press(
                "x",
                cosmic::iced::keyboard::key::Code::KeyX,
                cosmic::iced::keyboard::Modifiers::CTRL,
            )),
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Cut
            )))
        ));
        assert!(matches!(
            standard_editor_key_binding(key_press(
                "v",
                cosmic::iced::keyboard::key::Code::KeyV,
                cosmic::iced::keyboard::Modifiers::CTRL,
            )),
            Some(text_editor::Binding::Custom(Message::Command(
                CommandId::Paste
            )))
        ));
    }

    /// Inline objects in the main flow, and flows other than frame text (the
    /// main flow plus floating headers' flows).
    fn object_counts(document: &Document) -> (usize, usize) {
        (
            document
                .paragraphs()
                .iter()
                .map(|paragraph| paragraph.objects.len())
                .sum(),
            document.flows().count() - document.frames().count(),
        )
    }

    fn assert_editor_matches_model(app: &AmiProApp) {
        assert_eq!(
            app.editor.text(),
            app.document.plain_text(),
            "visible editor text"
        );
        app.document.validate().unwrap();
    }

    /// Delete a range over a break, Undo, type beside objects, Backspace over a
    /// floating header, and paste text holding U+FFFC, all through the
    /// default editor's message path.
    #[test]
    fn default_editor_keeps_inline_objects_intact() {
        let original = object_fixture_document();
        let mut app = test_app(original.clone(), DocumentLocation::Untitled);
        let edit = |app: &mut AmiProApp, edit: text_editor::Edit| {
            drop(cosmic::Application::update(
                app,
                Message::Edit(text_editor::Action::Edit(edit)),
            ));
        };
        let command = |app: &mut AmiProApp, command: CommandId| {
            drop(cosmic::Application::update(app, Message::Command(command)));
        };
        let place = |app: &mut AmiProApp, line: usize, column: usize, anchor: Option<usize>| {
            app.editor.move_to(text_editor::Cursor {
                position: text_editor::Position { line, column },
                selection: anchor.map(|column| text_editor::Position { line, column }),
            });
            if anchor.is_none() {
                // `move_to` keeps an existing selection; a click would clear it.
                app.editor.perform(text_editor::Action::ClearSelection);
            }
        };
        assert_eq!(object_counts(&app.document), (2, 2));

        // "a" and the break.
        place(&mut app, 1, 4, Some(0));
        edit(&mut app, text_editor::Edit::Delete);
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[1].text, "b\u{FFFC}c 🙂");
        assert_eq!(object_counts(&app.document), (1, 2));
        assert!(app.dirty);

        command(&mut app, CommandId::Undo);
        assert_eq!(app.document, original, "undo restores the exact root");
        assert_editor_matches_model(&app);
        assert!(!app.dirty);

        // Type between the floating header's placeholder and "c".
        place(&mut app, 1, 8, None);
        edit(&mut app, text_editor::Edit::Insert('x'));
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[1].text, "a\u{FFFC}b\u{FFFC}xc 🙂");
        assert_eq!(object_counts(&app.document), (2, 2));

        edit(&mut app, text_editor::Edit::Backspace);
        edit(&mut app, text_editor::Edit::Backspace);
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[1].text, "a\u{FFFC}bc 🙂");
        assert_eq!(
            object_counts(&app.document),
            (1, 1),
            "the header's flow is gone"
        );
        command(&mut app, CommandId::Undo);
        assert_editor_matches_model(&app);
        assert_eq!(object_counts(&app.document), (2, 2));

        // A pasted placeholder never becomes an object.
        place(&mut app, 0, "Intro é".len(), None);
        edit(
            &mut app,
            text_editor::Edit::Paste(Arc::new("p\u{FFFC}q\nr".into())),
        );
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[0].text, "Intro ép\u{FFFD}q");
        assert_eq!(app.document.paragraphs()[1].text, "r");
        assert_eq!(object_counts(&app.document), (2, 2));
        command(&mut app, CommandId::Undo);
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[0].text, "Intro é");
    }

    /// The same checks through the rich prototype's message path.
    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn prototype_keeps_inline_objects_intact() {
        let original = object_fixture_document();
        let mut app = test_app(original.clone(), DocumentLocation::Untitled);
        let act = |app: &mut AmiProApp, selection: Option<PrototypeSelection>, action| {
            if let Some(selection) = selection {
                app.prototype_selection = selection;
            }
            drop(cosmic::Application::update(app, Message::Prototype(action)));
        };
        let line_start = "Intro é\n".len();

        act(
            &mut app,
            Some(PrototypeSelection {
                anchor: line_start,
                focus: line_start + 4,
            }),
            PrototypeAction::Delete,
        );
        assert_editor_matches_model(&app);
        assert_eq!(object_counts(&app.document), (1, 2));
        act(&mut app, None, PrototypeAction::Undo);
        assert_eq!(app.document, original);
        assert_editor_matches_model(&app);

        act(
            &mut app,
            Some(PrototypeSelection::caret(line_start + 8)),
            PrototypeAction::Insert("x".into()),
        );
        assert_eq!(app.document.paragraphs()[1].text, "a\u{FFFC}b\u{FFFC}xc 🙂");
        assert_eq!(
            app.prototype_selection,
            PrototypeSelection::caret(line_start + 9)
        );
        act(&mut app, None, PrototypeAction::Backspace);
        act(&mut app, None, PrototypeAction::Backspace);
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[1].text, "a\u{FFFC}bc 🙂");
        assert_eq!(object_counts(&app.document), (1, 1));
        assert_eq!(
            app.prototype_selection,
            PrototypeSelection::caret(line_start + 5)
        );

        act(
            &mut app,
            Some(PrototypeSelection::caret(0)),
            PrototypeAction::Insert("\u{FFFC}🙂".into()),
        );
        assert_editor_matches_model(&app);
        assert!(app.document.paragraphs()[0].text.starts_with("\u{FFFD}🙂"));
        assert_eq!(object_counts(&app.document), (1, 1));
    }

    /// "Lead é 🙂" / "Anchor\u{FFFC} line" with a With para above frame, made
    /// through the create command.
    fn anchored_frame_document() -> (Document, amipro_document::FrameId) {
        use amipro_editing::{Command, NewFrameContent, TextPosition, apply};
        let document = Document::from_plain_text("Lead é 🙂\nAnchor line");
        let applied = apply(
            &document,
            &Command::CreateFrame {
                placement: amipro_document::Placement::BelowParagraph {
                    x_offset: 0,
                    size: amipro_document::Size {
                        width: 2_880,
                        height: 1_440,
                    },
                },
                style: amipro_document::FrameStyle {
                    wrap: amipro_document::Wrap::NoWrapAround,
                    ..amipro_document::FrameStyle::default()
                },
                content: NewFrameContent::Text,
                anchor: Some(TextPosition {
                    flow: document.main_flow_id(),
                    paragraph: 1,
                    byte: "Anchor".len(),
                }),
                layout: None,
            },
        )
        .unwrap();
        let frame = *applied.report.frames.iter().next().unwrap();
        applied.document.validate().unwrap();
        (applied.document, frame)
    }

    /// V05/V06 through the default editor: a single Backspace or Delete onto
    /// a frame anchor does nothing; deleting a selection that holds it deletes
    /// the frame, and one Undo restores both.
    #[test]
    fn default_editor_single_deletes_leave_frame_anchors() {
        let (original, frame) = anchored_frame_document();
        let mut app = test_app(original.clone(), DocumentLocation::Untitled);
        let edit = |app: &mut AmiProApp, edit: text_editor::Edit| {
            drop(cosmic::Application::update(
                app,
                Message::Edit(text_editor::Action::Edit(edit)),
            ));
        };
        let after_anchor = "Anchor\u{FFFC}".len();
        for (column, action) in [
            (after_anchor, text_editor::Edit::Backspace),
            ("Anchor".len(), text_editor::Edit::Delete),
        ] {
            app.editor.move_to(text_editor::Cursor {
                position: text_editor::Position { line: 1, column },
                selection: None,
            });
            app.editor.perform(text_editor::Action::ClearSelection);
            edit(&mut app, action);
            assert_eq!(app.document, original, "the anchor and frame stay");
            assert_editor_matches_model(&app);
            assert!(!app.dirty);
        }

        app.editor.move_to(text_editor::Cursor {
            position: text_editor::Position {
                line: 1,
                column: after_anchor + 1,
            },
            selection: Some(text_editor::Position { line: 1, column: 3 }),
        });
        edit(&mut app, text_editor::Edit::Delete);
        assert_editor_matches_model(&app);
        assert_eq!(app.document.paragraphs()[1].text, "Ancline");
        assert!(app.document.frame(frame).is_none());
        drop(cosmic::Application::update(
            &mut app,
            Message::Command(CommandId::Undo),
        ));
        assert_eq!(app.document, original);
        assert_editor_matches_model(&app);
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn prototype_single_deletes_leave_frame_anchors() {
        let (original, frame) = anchored_frame_document();
        let mut app = test_app(original.clone(), DocumentLocation::Untitled);
        let line_start = "Lead é 🙂\n".len();
        let after_anchor = line_start + "Anchor\u{FFFC}".len();
        for (caret, action) in [
            (after_anchor, PrototypeAction::Backspace),
            (line_start + "Anchor".len(), PrototypeAction::Delete),
        ] {
            app.prototype_selection = PrototypeSelection::caret(caret);
            drop(cosmic::Application::update(
                &mut app,
                Message::Prototype(action),
            ));
            assert_eq!(app.document, original);
            assert_editor_matches_model(&app);
        }
        app.prototype_selection = PrototypeSelection {
            anchor: line_start + 3,
            focus: after_anchor + 1,
        };
        drop(cosmic::Application::update(
            &mut app,
            Message::Prototype(PrototypeAction::Delete),
        ));
        assert!(app.document.frame(frame).is_none());
        assert_editor_matches_model(&app);
        drop(cosmic::Application::update(
            &mut app,
            Message::Prototype(PrototypeAction::Undo),
        ));
        assert_eq!(app.document, original);
    }

    #[test]
    fn history_restores_editor_text_by_delta_and_keeps_it_in_sync() {
        let original = "first paragraph\nsecond 🙂 paragraph\nthird";
        let document = Document::from_plain_text(original);
        let mut app = test_app(document, DocumentLocation::Untitled);
        let edit = |app: &mut AmiProApp, edit: text_editor::Edit| {
            drop(cosmic::Application::update(
                app,
                Message::Edit(text_editor::Action::Edit(edit)),
            ));
        };
        let command = |app: &mut AmiProApp, command: CommandId| {
            drop(cosmic::Application::update(app, Message::Command(command)));
        };
        let assert_in_sync = |app: &AmiProApp, expected: &str| {
            assert_eq!(app.document.plain_text(), expected);
            assert_eq!(app.editor.text(), expected, "visible editor text");
            assert!(app.document.validate().is_ok());
        };

        app.editor.move_to(text_editor::Cursor {
            position: text_editor::Position {
                line: 1,
                column: "second 🙂".len(),
            },
            selection: None,
        });
        edit(
            &mut app,
            text_editor::Edit::Paste(Arc::new(" inserted\nnew line é".into())),
        );
        let pasted = "first paragraph\nsecond 🙂 inserted\nnew line é paragraph\nthird";
        assert_in_sync(&app, pasted);

        app.editor.move_to(text_editor::Cursor {
            position: text_editor::Position { line: 2, column: 3 },
            selection: Some(text_editor::Position { line: 0, column: 5 }),
        });
        edit(&mut app, text_editor::Edit::Delete);
        let joined = "first line é paragraph\nthird";
        assert_in_sync(&app, joined);

        command(&mut app, CommandId::Undo);
        assert_in_sync(&app, pasted);
        command(&mut app, CommandId::Undo);
        assert_in_sync(&app, original);
        command(&mut app, CommandId::Redo);
        assert_in_sync(&app, pasted);
        command(&mut app, CommandId::Redo);
        assert_in_sync(&app, joined);

        command(&mut app, CommandId::Undo);
        command(&mut app, CommandId::Undo);
        edit(&mut app, text_editor::Edit::Insert('!'));
        assert_in_sync(&app, "first paragraph\nsecond 🙂! paragraph\nthird");
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn rich_selection_keeps_toolbar_cursor_and_cross_paragraph_anchor_synchronized() {
        let document = Document::from_plain_text("first\nsecond");

        let cursor = prototype_text_cursor(
            &document,
            PrototypeSelection {
                anchor: 2,
                focus: 9,
            },
        )
        .unwrap();

        assert_eq!(cursor.position.line, 1);
        assert_eq!(cursor.position.column, 3);
        assert_eq!(
            cursor.selection,
            Some(text_editor::Position { line: 0, column: 2 })
        );
        assert_eq!(
            prototype_selection_from_text_cursor(&document, cursor),
            Some(PrototypeSelection {
                anchor: 2,
                focus: 9
            })
        );
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn prototype_edits_incrementally_synchronize_fallback_content_and_history() {
        let document = Document::from_plain_text(
            &(0..320)
                .map(|index| format!("paragraph {index:03} cafe 🙂 with retained text"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let session = DocumentSession::new(1, document, DocumentLocation::Untitled, "Ready".into());
        let mut app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        let middle = rich_editor_prototype::offset_for_cursor(
            &app.document,
            amipro_editing::Cursor {
                paragraph: 160,
                byte: 14,
            },
        )
        .unwrap();
        app.prototype_selection = PrototypeSelection::caret(middle);

        app.update_prototype(PrototypeAction::Insert("é".into()));
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.rich_layout_generation, 1);

        app.prototype_selection = PrototypeSelection {
            anchor: middle,
            focus: middle + "é".len(),
        };
        app.update_prototype(PrototypeAction::Insert("🙂".into()));
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.rich_layout_generation, 2);

        app.update_prototype(PrototypeAction::Backspace);
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Undo);
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Redo);
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.document.paragraphs().len(), 320);

        app.prototype_selection = PrototypeSelection::caret(middle);
        app.update_prototype(PrototypeAction::ParagraphBreak);
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.document.paragraphs().len(), 321);
        app.update_prototype(PrototypeAction::Undo);
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.document.paragraphs().len(), 320);
        app.update_prototype(PrototypeAction::Redo);
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.document.paragraphs().len(), 321);
        app.update_prototype(PrototypeAction::Backspace);
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.document.paragraphs().len(), 320);
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn message_undo_redo_restore_rich_caret_before_followup_typing() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "abcdef".into();
        let session = DocumentSession::new(1, document, DocumentLocation::Untitled, "Ready".into());
        let mut app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        app.prototype_selection = PrototypeSelection::caret(3);

        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::Prototype(PrototypeAction::Insert("X".into())),
        ));
        assert_eq!(app.document.plain_text(), "abcXdef");
        assert_eq!(app.prototype_selection, PrototypeSelection::caret(4));

        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::Command(CommandId::Undo),
        ));
        assert_eq!(app.document.plain_text(), "abcdef");
        assert_eq!(app.prototype_selection, PrototypeSelection::caret(3));

        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::Command(CommandId::Redo),
        ));
        assert_eq!(app.document.plain_text(), "abcXdef");
        assert_eq!(app.prototype_selection, PrototypeSelection::caret(4));

        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::Command(CommandId::Undo),
        ));
        drop(<AmiProApp as cosmic::Application>::update(
            &mut app,
            Message::Prototype(PrototypeAction::Insert("Y".into())),
        ));
        assert_eq!(app.document.plain_text(), "abcYdef");
        assert_eq!(app.editor.text(), app.document.plain_text());
        assert_eq!(app.prototype_selection, PrototypeSelection::caret(4));
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    fn prototype_fallback_tracks_semantic_newlines_and_unicode_boundaries() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "alpha 🙂 omega".into();
        let session = DocumentSession::new(1, document, DocumentLocation::Untitled, "Ready".into());
        let mut app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        app.prototype_selection = PrototypeSelection {
            anchor: "alpha ".len(),
            focus: "alpha 🙂".len(),
        };

        app.update_prototype(PrototypeAction::Insert("é\r\n二\r三\n\r四".into()));
        assert_eq!(app.document.plain_text(), "alpha é\n二\n三\n\n四 omega");
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Undo);
        assert_eq!(app.document.plain_text(), "alpha 🙂 omega");
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Redo);
        assert_eq!(app.document.plain_text(), "alpha é\n二\n三\n\n四 omega");
        assert_eq!(app.editor.text(), app.document.plain_text());

        let mut combining_document = Document::default();
        combining_document.paragraph_mut(0).text = "e\u{301}x".into();
        let session = &mut app.sessions[0];
        let previous = session.document.clone();
        synchronize_editor_text(&mut session.editor, &previous, &combining_document);
        session.document = combining_document;
        session.history.clear();
        app.prototype_selection = PrototypeSelection::caret(0);
        app.update_prototype(PrototypeAction::Delete);
        assert_eq!(app.document.plain_text(), "\u{301}x");
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Undo);
        assert_eq!(app.document.plain_text(), "e\u{301}x");
        assert_eq!(app.editor.text(), app.document.plain_text());
        app.update_prototype(PrototypeAction::Redo);
        assert_eq!(app.document.plain_text(), "\u{301}x");
        assert_eq!(app.editor.text(), app.document.plain_text());
    }

    #[cfg(feature = "rich-editor-prototype")]
    #[test]
    #[ignore = "manual release-mode end-to-end controller performance baseline"]
    fn rich_editor_typing_performance_baseline() {
        let document = performance_fixture();
        let original = document.clone();
        let mut document_clone = Vec::new();
        let mut plain_text = Vec::new();
        let mut fallback_rebuild = Vec::new();
        let mut derived_layout_rebuild = Vec::new();
        for _ in 0..40 {
            document_clone.push(measure(|| {
                black_box(document.clone());
            }));
            plain_text.push(measure(|| {
                black_box(document.plain_text());
            }));
            let text = document.plain_text();
            fallback_rebuild.push(measure(|| {
                let content: text_editor::Content = text_editor::Content::with_text(&text);
                black_box(content);
            }));
            derived_layout_rebuild.push(measure(|| {
                black_box(rich_buffer::DocumentLayout::new(
                    &document,
                    document_text_width(&document),
                ));
            }));
        }
        let session = DocumentSession::new(1, document, DocumentLocation::Untitled, "Ready".into());
        let mut app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 2,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        eprintln!(
            "typing fixture: {} semantic paragraphs, {} fallback lines",
            app.document.paragraphs().len(),
            app.editor.line_count()
        );
        let paragraph_indexes = [
            0,
            app.document.paragraphs().len() / 2,
            app.document.paragraphs().len() - 1,
        ];
        let mut insert = Vec::new();
        let mut delete = Vec::new();
        let mut replace = Vec::new();
        let mut undo = Vec::new();
        let mut redo = Vec::new();

        for _ in 0..12 {
            for paragraph in paragraph_indexes {
                let source = &app.document.paragraphs()[paragraph].text;
                let mut byte = source.len() / 2;
                while !source.is_char_boundary(byte) {
                    byte -= 1;
                }
                let offset = rich_editor_prototype::offset_for_cursor(
                    &app.document,
                    amipro_editing::Cursor { paragraph, byte },
                )
                .unwrap();
                app.prototype_selection = PrototypeSelection::caret(offset);
                insert.push(measure(|| {
                    app.update_prototype(PrototypeAction::Insert("x".into()))
                }));
                undo.push(measure(|| app.update_prototype(PrototypeAction::Undo)));
                redo.push(measure(|| app.update_prototype(PrototypeAction::Redo)));
                undo.push(measure(|| app.update_prototype(PrototypeAction::Undo)));

                let source = &app.document.paragraphs()[paragraph].text;
                let byte = source
                    .char_indices()
                    .nth(2)
                    .map_or(source.len(), |(byte, _)| byte);
                let offset = rich_editor_prototype::offset_for_cursor(
                    &app.document,
                    amipro_editing::Cursor { paragraph, byte },
                )
                .unwrap();
                app.prototype_selection = PrototypeSelection::caret(offset);
                delete.push(measure(|| app.update_prototype(PrototypeAction::Backspace)));
                undo.push(measure(|| app.update_prototype(PrototypeAction::Undo)));

                let source = &app.document.paragraphs()[paragraph].text;
                let start = source.char_indices().nth(3).map_or(0, |(byte, _)| byte);
                let end = start + source[start..].chars().next().unwrap().len_utf8();
                let start = rich_editor_prototype::offset_for_cursor(
                    &app.document,
                    amipro_editing::Cursor {
                        paragraph,
                        byte: start,
                    },
                )
                .unwrap();
                let end = rich_editor_prototype::offset_for_cursor(
                    &app.document,
                    amipro_editing::Cursor {
                        paragraph,
                        byte: end,
                    },
                )
                .unwrap();
                app.prototype_selection = PrototypeSelection {
                    anchor: start,
                    focus: end,
                };
                replace.push(measure(|| {
                    app.update_prototype(PrototypeAction::Insert("z".into()))
                }));
                undo.push(measure(|| app.update_prototype(PrototypeAction::Undo)));
            }
        }

        let paragraph = paragraph_indexes[1];
        let byte = app.document.paragraphs()[paragraph].text.len();
        let offset = rich_editor_prototype::offset_for_cursor(
            &app.document,
            amipro_editing::Cursor { paragraph, byte },
        )
        .unwrap();
        app.prototype_selection = PrototypeSelection::caret(offset);
        let mut sustained = Vec::new();
        for _ in 0..40 {
            sustained.push(measure(|| {
                app.update_prototype(PrototypeAction::Insert("t".into()))
            }));
        }
        for _ in 0..40 {
            undo.push(measure(|| app.update_prototype(PrototypeAction::Undo)));
        }

        assert_eq!(app.document, original);
        assert_eq!(app.editor.text(), app.document.plain_text());

        let empty_original = Document::default();
        let empty_session = DocumentSession::new(
            2,
            empty_original.clone(),
            DocumentLocation::Untitled,
            "Ready".into(),
        );
        let mut empty_app = AmiProApp {
            core: cosmic::Core::default(),
            #[cfg(feature = "page-surface")]
            shift: false,
            #[cfg(feature = "page-surface")]
            control: false,
            #[cfg(feature = "page-surface")]
            last_find: None,
            #[cfg(feature = "page-surface")]
            spelling: spelling::Spelling::default(),
            #[cfg(feature = "page-surface")]
            field_variables: std::collections::BTreeMap::new(),
            sessions: vec![empty_session],
            tab_model: segmented_button::SingleSelectModel::default(),
            active_session: 0,
            next_session_id: 3,
            pending_close: None,
            import_dialog: None,
            close_after_save: None,
            styles_visible: false,
            rail: RailState::default(),
            workspace: WorkspaceState::default(),
            preferences: Preferences::default(),
            settings: None,
            #[cfg(feature = "page-surface")]
            updates: updates_panel::UpdatesState::default(),
            #[cfg(feature = "page-surface")]
            help: help_panel::HelpState::default(),
            #[cfg(feature = "page-surface")]
            update_backend: Arc::new(updates::OfflineBackend),
            #[cfg(feature = "page-surface")]
            recovery: recovery_panel::RecoveryState::default(),
            toolbar_editor: None,
            preferences_dialog: None,
            new_document_dialog: None,
            revert_pending: false,
            password_dialog: None,
            clean_screen: false,
            #[cfg(feature = "page-surface")]
            special_character: false,
            #[cfg(feature = "page-surface")]
            font_families: Vec::new(),
            #[cfg(feature = "page-surface")]
            fast_format_ask: None,
            about: About::default(),
            show_about: false,
            help_drawer: None,
            quit_requested: false,
            clipboard_has_text: None,
            #[cfg(feature = "page-surface")]
            clipboard_has_image: None,
            #[cfg(feature = "page-surface")]
            cell_clipboard: None,
            #[cfg(feature = "page-surface")]
            draw_memory: None,
            #[cfg(feature = "page-surface")]
            draw_clipboard: None,
            page_keyboard: 0,
            #[cfg(feature = "page-surface")]
            page_surface: false,
        };
        let mut empty_typing = Vec::new();
        for _ in 0..40 {
            empty_typing.push(measure(|| {
                empty_app.update_prototype(PrototypeAction::Insert("t".into()))
            }));
        }
        for _ in 0..40 {
            empty_app.update_prototype(PrototypeAction::Undo);
        }
        assert_eq!(empty_app.document, empty_original);
        black_box(&app);
        eprintln!(
            "rich editor controller baseline: {} paragraphs; document clone {}; plain-text rebuild {}; legacy fallback rebuild {}; legacy derived-layout rebuild {}; empty-document typing {}; insert {}; delete {}; replacement {}; sustained typing {}; undo {}; redo {}",
            app.document.paragraphs().len(),
            latency_summary(&mut document_clone),
            latency_summary(&mut plain_text),
            latency_summary(&mut fallback_rebuild),
            latency_summary(&mut derived_layout_rebuild),
            latency_summary(&mut empty_typing),
            latency_summary(&mut insert),
            latency_summary(&mut delete),
            latency_summary(&mut replace),
            latency_summary(&mut sustained),
            latency_summary(&mut undo),
            latency_summary(&mut redo),
        );
    }

    /// Private SAM named by `AMIPRO_PERF_SAM`, or a synthetic 350-paragraph
    /// document when the corpus is unavailable.
    fn performance_fixture() -> Document {
        if let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") {
            amipro_formats::import_sam_path(path).unwrap().document
        } else {
            Document::from_plain_text(
                &(0..350)
                    .map(|index| {
                        format!(
                            "paragraph {index:03} cafe 🙂 with enough representative text for editing"
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
    }

    /// Release-mode end-to-end baseline for the accepted default editor.
    #[cfg(not(feature = "rich-editor-prototype"))]
    mod default_editor_performance {
        use super::*;

        /// Headless frame pipeline for the real application view: rebuild the
        /// view, diff the retained widget tree, lay out, and record draw
        /// primitives with the tiny-skia renderer (no rasterization/compositor).
        struct FrameHarness {
            renderer: cosmic::Renderer,
            theme: cosmic::Theme,
            tree: Option<Tree>,
            window: cosmic::iced::Size,
        }

        #[derive(Default)]
        struct FrameTimings {
            view: Vec<Duration>,
            layout: Vec<Duration>,
            draw: Vec<Duration>,
        }

        impl FrameHarness {
            fn new(window: cosmic::iced::Size) -> Self {
                Self {
                    renderer: cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
                        cosmic::iced::Font::default(),
                        cosmic::iced::Pixels(14.0),
                    )),
                    theme: cosmic::Theme::default(),
                    tree: None,
                    window,
                }
            }

            fn frame(&mut self, app: &AmiProApp, timings: &mut FrameTimings) {
                use cosmic::iced::advanced::Renderer as _;

                let started = Instant::now();
                let mut root = cosmic::Application::view(app);
                match &mut self.tree {
                    Some(tree) => tree.diff(root.as_widget_mut()),
                    None => self.tree = Some(Tree::new(&root)),
                }
                let tree = self.tree.as_mut().unwrap();
                let viewed = Instant::now();
                let node = root.as_widget_mut().layout(
                    tree,
                    &self.renderer,
                    &layout::Limits::new(cosmic::iced::Size::ZERO, self.window),
                );
                let laid_out = Instant::now();
                let viewport = cosmic::iced::Rectangle::with_size(self.window);
                self.renderer.reset(viewport);
                root.as_widget().draw(
                    tree,
                    &mut self.renderer,
                    &self.theme,
                    &cosmic::iced::advanced::renderer::Style::default(),
                    cosmic::iced::advanced::Layout::new(&node),
                    cosmic::iced::mouse::Cursor::Unavailable,
                    &viewport,
                );
                let drawn = Instant::now();
                timings.view.push(viewed - started);
                timings.layout.push(laid_out - viewed);
                timings.draw.push(drawn - laid_out);
            }
        }

        #[derive(Default)]
        struct EditTimings {
            update: Vec<Duration>,
            total: Vec<Duration>,
            frame: FrameTimings,
        }

        impl EditTimings {
            fn report(&mut self, label: &str) {
                eprintln!(
                    "  {label}: total {}; update {}; view {}; layout {}; draw {}",
                    latency_summary(&mut self.total),
                    latency_summary(&mut self.update),
                    latency_summary(&mut self.frame.view),
                    latency_summary(&mut self.frame.layout),
                    latency_summary(&mut self.frame.draw),
                );
            }
        }

        /// One message followed by the frame the user would see.
        fn timed_message(
            app: &mut AmiProApp,
            harness: &mut FrameHarness,
            message: Message,
            timings: &mut EditTimings,
        ) {
            let started = Instant::now();
            drop(cosmic::Application::update(app, message));
            let updated = Instant::now();
            harness.frame(app, &mut timings.frame);
            timings.update.push(updated - started);
            timings.total.push(started.elapsed());
        }

        fn byte_column(text: &str, characters: usize) -> usize {
            text.char_indices()
                .nth(characters)
                .map_or(text.len(), |(byte, _)| byte)
        }

        #[test]
        #[ignore = "manual release-mode default-editor end-to-end performance baseline"]
        fn default_editor_typing_performance_baseline() {
            const SAMPLES: usize = 40;
            let window = cosmic::iced::Size::new(1280.0, 800.0);
            let fixture = performance_fixture();
            let last_line = fixture.paragraphs().len() - 1;
            eprintln!(
                "default editor baseline: {} paragraphs, {SAMPLES} samples per operation, {}x{} window",
                fixture.paragraphs().len(),
                window.width,
                window.height
            );
            let cases = [
                ("empty document", Document::default(), 0),
                ("fixture start", fixture.clone(), 0),
                ("fixture middle", fixture.clone(), last_line / 2),
                ("fixture end", fixture, last_line),
            ];
            for (name, document, line) in cases {
                let text = &document.paragraphs()[line].text;
                let column = byte_column(text, text.chars().count() / 2);
                let mut app = test_app(document, DocumentLocation::Untitled);
                let mut harness = FrameHarness::new(window);
                let mut warmup = FrameTimings::default();
                harness.frame(&app, &mut warmup);
                // Bring the edited line into view, as a user scrolling there would.
                drop(cosmic::Application::update(
                    &mut app,
                    Message::Edit(text_editor::Action::Scroll {
                        lines: i32::try_from(line.saturating_sub(5)).unwrap(),
                    }),
                ));
                app.editor.move_to(text_editor::Cursor {
                    position: text_editor::Position { line, column },
                    selection: None,
                });
                harness.frame(&app, &mut warmup);
                eprintln!("{name} (line {line}):");

                let mut typing = EditTimings::default();
                for _ in 0..SAMPLES {
                    timed_message(
                        &mut app,
                        &mut harness,
                        Message::Edit(text_editor::Action::Edit(text_editor::Edit::Insert('e'))),
                        &mut typing,
                    );
                }
                typing.report("typing");

                let mut deletion = EditTimings::default();
                for _ in 0..SAMPLES {
                    timed_message(
                        &mut app,
                        &mut harness,
                        Message::Edit(text_editor::Action::Edit(text_editor::Edit::Backspace)),
                        &mut deletion,
                    );
                }
                deletion.report("backspace");

                let mut replacement = EditTimings::default();
                for _ in 0..SAMPLES {
                    let caret = app.editor.cursor().position;
                    let text = &app.document.paragraphs()[caret.line].text;
                    let preceding = text[..caret.column].chars().count();
                    let anchor = byte_column(text, preceding.saturating_sub(3));
                    app.editor.move_to(text_editor::Cursor {
                        position: caret,
                        selection: Some(text_editor::Position {
                            line: caret.line,
                            column: anchor,
                        }),
                    });
                    timed_message(
                        &mut app,
                        &mut harness,
                        Message::Edit(text_editor::Action::Edit(text_editor::Edit::Insert('r'))),
                        &mut replacement,
                    );
                }
                replacement.report("selection replacement");

                let mut undo = EditTimings::default();
                for _ in 0..SAMPLES {
                    timed_message(
                        &mut app,
                        &mut harness,
                        Message::Command(CommandId::Undo),
                        &mut undo,
                    );
                }
                undo.report("undo");

                let mut redo = EditTimings::default();
                for _ in 0..SAMPLES {
                    timed_message(
                        &mut app,
                        &mut harness,
                        Message::Command(CommandId::Redo),
                        &mut redo,
                    );
                }
                redo.report("redo");
                black_box(&app.document);
            }
        }
    }

    #[cfg(feature = "rich-editor-prototype")]
    fn measure(operation: impl FnOnce()) -> Duration {
        let started = Instant::now();
        operation();
        started.elapsed()
    }

    fn latency_summary(samples: &mut [Duration]) -> String {
        samples.sort_unstable();
        let percentile = |numerator: usize| {
            samples[(samples.len() - 1) * numerator / 100].as_secs_f64() * 1_000.0
        };
        format!(
            "median {:.3} ms, p95 {:.3} ms, worst {:.3} ms",
            percentile(50),
            percentile(95),
            samples.last().unwrap().as_secs_f64() * 1_000.0,
        )
    }
}
