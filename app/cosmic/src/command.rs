//! Authoritative UI command catalog.
//!
//! Menu construction, shortcut labels/matching, enablement, and semantic
//! dispatch all start from this catalog. Future commands intentionally remain
//! present so menus can name their eventual homes without making them active.

use cosmic::iced::keyboard::{self, Key};
use cosmic::widget::menu::KeyBind;
use cosmic::widget::menu::key_bind::Modifier;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MenuRoot {
    File,
    Edit,
    View,
    Insert,
    Format,
    /// Ami Pro's Table menu, always present (TD1).
    Table,
    /// Ami Pro's Draw menu, always present (DW4).
    Draw,
    Tools,
    Window,
    Help,
}

impl MenuRoot {
    #[cfg(test)]
    pub const ALL: [Self; 10] = [
        Self::File,
        Self::Edit,
        Self::View,
        Self::Insert,
        Self::Format,
        Self::Table,
        Self::Draw,
        Self::Tools,
        Self::Window,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::View => "View",
            Self::Insert => "Insert",
            Self::Format => "Format",
            Self::Table => "Table",
            Self::Draw => "Draw",
            Self::Tools => "Tools",
            Self::Window => "Window",
            Self::Help => "Help",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Implementation {
    Implemented,
    Future,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchRoute {
    File,
    Editor,
    View,
    Format,
    Window,
    Help,
    Future,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolbarRoute {
    ActionBar,
    TabStrip,
    StylesPanel,
    None,
    Future,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnableRule {
    Always,
    HasDocument,
    Save,
    Undo,
    Redo,
    Selection,
    /// Character formatting: on the page surface a caret or a selection in
    /// text (a caret formats the text typed next, as in Ami Pro); in the
    /// text editor a selection.
    TextFormat,
    Paste,
    HasText,
    MultipleTabs,
    /// One or more frames are selected on the page surface.
    FrameSelected,
    /// File › Revert to Saved…: the document has a saved file and changed
    /// since.
    RevertToSaved,
    /// The frame selection can become a group.
    FramesGroupable,
    /// The frame selection is one group.
    FrameGroup,
    /// The caret's page has a page before it (page surface).
    PreviousPage,
    /// The caret's page has a page after it (page surface).
    NextPage,
    /// Insert › Image… can put a picture where the caret or the frame
    /// selection is (`UI_SPEC.md` §25).
    PictureTarget,
    /// One frame holding a picture is selected.
    PictureSelected,
    /// One frame holding a bitmap a reader takes is selected (Tools ›
    /// Image Processing…).
    BitmapSelected,
    /// The caret is in a field (page surface).
    OnField,
    /// The caret or the cell selection is in a table; each Table command
    /// has its own rule beyond that (`UI_SPEC.md` §32).
    InTable,
    /// Insert › Table…: the caret in the main text outside a table, or one
    /// empty text frame selected.
    TableTarget,
    /// Convert Text to Table: paragraphs of the main text selected.
    TextToTable,
    /// Tools › Charting…: the page surface, outside Draw mode.
    Charting,
    /// Tools › Equations… and Paste LaTeX Equation: the same.
    Equations,
    /// Draw mode is open; each Draw command has its own rule beyond that
    /// (`UI_SPEC.md` §33).
    InDrawing,
    /// Insert › Drawing…: where Insert › Image… can put a picture.
    DrawingTarget,
    Future,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CommandId {
    NewDocument,
    /// File › New… (CM1): a new document from a style sheet.
    NewFromStyleSheet,
    Open,
    CloseDocument,
    Save,
    SaveAs,
    RevertToSaved,
    /// File › Password… (CM5): the document saved password protected.
    Password,
    ExportPdf,
    ExportDocx,
    ExportOdt,
    ExportRtf,
    ExportText,
    ExportMarkdown,
    ExportFolder,
    ImportFolder,
    Print,
    PrintCurrentPage,
    PrintOptions,
    /// File › Page Setup…: Ami Pro's File menu name for Modify Page Layout;
    /// it opens the same rail panel as Format › Page Layout….
    PageSetup,
    DocumentProperties,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    PastePlainText,
    Delete,
    SelectAll,
    Find,
    FindNext,
    FindPrevious,
    Replace,
    GoToPage,
    GoToHeader,
    GoToFooter,
    GoToRuler,
    PreviousPage,
    NextPage,
    ShowToolbar,
    StylesPanel,
    Ruler,
    StatusBar,
    ZoomIn,
    ZoomOut,
    /// View › Full Page, Enlarged, Custom (CM8).
    FullPage,
    Enlarged,
    /// View › Draft Mode (CM9).
    DraftMode,
    /// View › Outline Mode (OM1).
    OutlineMode,
    /// View › Outline ▸ (OB.4): Expand and Contract the caret's heading,
    /// Show levels 1–9 and All.
    OutlineExpand,
    OutlineContract,
    /// View › Outline ▸ Use Outline Styles (OB.5).
    UseOutlineStyles,
    /// View › Outline ▸ Outline Navigator (OB.6): the headings in the rail.
    OutlineNavigator,
    ShowLevel1,
    ShowLevel2,
    ShowLevel3,
    ShowLevel4,
    ShowLevel5,
    ShowLevel6,
    ShowLevel7,
    ShowLevel8,
    ShowLevel9,
    ShowAllLevels,
    CustomZoom,
    ActualSize,
    FitPageWidth,
    ContinuousPages,
    TwoPages,
    FullScreen,
    PageBreak,
    ColumnBreak,
    Frame,
    DrawFrame,
    Table,
    InsertRowAbove,
    InsertRowBelow,
    InsertColumnLeft,
    InsertColumnRight,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
    /// Table › Edit Formula… (TD.7): the Formula section of Table
    /// Properties, its box focused.
    EditFormula,
    /// Table › Quick Add ▸ Row and Column.
    QuickAddRow,
    QuickAddColumn,
    /// Table › Recalculate Table.
    RecalculateTable,
    SelectColumn,
    SelectRow,
    SelectTable,
    ConnectCells,
    TableHeadings,
    LeaderNone,
    LeaderDots,
    LeaderDashes,
    LeaderLine,
    ProtectCells,
    DistributeRows,
    DistributeColumns,
    SortAscending,
    SortDescending,
    TableStyleGrid,
    TableStyleOutline,
    TableStyleHeaderRow,
    TableStyleBandedRows,
    TableStylePlain,
    TextToTable,
    TableToText,
    TableProperties,
    Image,
    InsertFile,
    Drawing,
    // The Draw menu (`UI_SPEC.md` §33).
    DrawProperties,
    DrawExtract,
    DrawApply,
    DrawRotateLeft,
    DrawRotateRight,
    DrawRotate,
    DrawFlipHorizontal,
    DrawFlipVertical,
    DrawGroup,
    DrawUngroup,
    DrawFront,
    DrawBack,
    DrawForward,
    DrawBackward,
    DrawAlignLeft,
    DrawAlignCentre,
    DrawAlignRight,
    DrawAlignTop,
    DrawAlignMiddle,
    DrawAlignBottom,
    DrawDistributeAcross,
    DrawDistributeDown,
    DrawDuplicate,
    DrawSnap,
    DrawGrid,
    DrawSelectAll,
    ImportDrawing,
    SaveDrawingAs,
    DrawDone,
    HeaderFooter,
    PageNumber,
    Footnote,
    /// Insert › Note (`UI_SPEC.md` §34, NR1).
    InsertNote,
    /// Insert › Cross-Reference… (NR6).
    CrossReference,
    /// Edit › Notes…: the Notes panel (NR1).
    Notes,
    /// Edit › Paste Special… (DO.4, OD10): the rail lists the clipboard's
    /// formats and pastes the chosen one.
    PasteSpecial,
    /// Tools › Image Processing… (DO.5, OD8): Frame Properties › Picture ›
    /// Image for the selected bitmap.
    ImageProcessing,
    /// Tools › Charting… (DO.8, OD3 to OD6): the Chart panel for a chart,
    /// or a new chart in the selected empty frame, an empty table cell, or
    /// a new frame.
    Charting,
    /// Tools › Chart from Table (DO.8, OD4, OD11): the selected cells (or
    /// the table) become the data of a new chart after the table.
    ChartFromTable,
    /// Tools › Equations… (DO.9, OD2, OD6): an equation edits in place with
    /// the template palette in the rail; anywhere else a new one starts in
    /// the selected empty frame, an empty table cell, or the main text.
    Equations,
    /// Tools › Paste LaTeX Equation (DO.9, OD11): the clipboard's LaTeX
    /// (`$...$`, `\[...\]`, an equation environment) becomes an equation.
    PasteLatexEquation,
    /// Edit › Links…: the read-only list of the document's links and OLE
    /// objects (`UI_SPEC.md` §42.1, DO.3).
    Links,
    /// Help › Check for Updates…: the Updates panel (P6.2).
    CheckForUpdates,
    /// File › Recover Documents…: the Recover documents panel (P6.3).
    RecoverDocuments,
    /// Edit › Bookmarks…: the Bookmarks panel (NR4).
    Bookmarks,
    /// Tools › Footnote Options…: the Footnotes panel (NR10).
    FootnoteOptions,
    /// Tools › TOC, Index…: the Contents & Index panel (TX1).
    ContentsIndex,
    /// File › Master Document…: the Master Document panel (MS1).
    MasterDocument,
    /// File › Merge…: the Merge panel (`UI_SPEC.md` §38, MG1).
    Merge,
    /// File › Print Envelope…: the Envelope panel (`UI_SPEC.md` §38, MG8).
    PrintEnvelope,
    /// Insert › Merge Field… (Ami Pro's Edit › Insert › Merge Field…): the
    /// Merge panel at its fields (MG1).
    MergeField,
    /// Edit › Mark Text ▸ Index Entry…: its Entries page (TX1, TX4).
    MarkIndexEntry,
    /// Edit › Mark Text ▸ TOC Entry…: its Entries page's TOC entry.
    MarkTocEntry,
    /// Edit › Mark Text ▸ Glossary…: the Glossary panel (TX7).
    MarkGlossary,
    /// Insert › Glossary Record… (Ctrl+K): the word at the caret expanded,
    /// or the Glossary panel (TX7).
    InsertGlossary,
    /// View › Notes: note marks shown (NR1).
    ShowNotes,
    /// View › Bookmarks (NR5).
    ShowBookmarks,
    /// View › Show ▸ (CM10).
    ShowTabsReturns,
    ColumnGuides,
    MarginsInColour,
    ShowPictures,
    /// View › Show ▸ Table row/column headings (TD.7, TN9).
    TableAddresses,
    InsertBullet,
    Field,
    SpecialCharacter,
    CreateStyle,
    Styles,
    ModifyCurrentStyle,
    OutlineNumbering,
    UseAnotherStyleSheet,
    SaveAsStyleSheet,
    StyleManagement,
    Bold,
    Italic,
    Underline,
    WordUnderline,
    UpperCase,
    LowerCase,
    InitialCaps,
    SmallCaps,
    SpecialEffects,
    Font,
    ClearDirectFormatting,
    /// Edit › Mark Text ▸ Protected Text and No Hyphenation (`<+*>`, `<+.>`).
    ProtectedText,
    NoHyphenation,
    /// Format › Fast Format (CM4).
    FastFormat,
    ParagraphProperties,
    AlignLeft,
    Center,
    AlignRight,
    Justify,
    Bullets,
    Numbering,
    BulletNumberStyle,
    Promote,
    /// Outline › Move Up and Move Down (OM3): the caret's heading with its
    /// section.
    MoveUp,
    MoveDown,
    Demote,
    IncreaseIndent,
    DecreaseIndent,
    FrameProperties,
    GraphicsScaling,
    BringToFront,
    SendToBack,
    GroupFrames,
    UngroupFrames,
    PageLayout,
    /// Format › Ruler ▸ and Page Layout ▸ (CA.8).
    InsertRuler,
    RemoveRuler,
    InsertPageLayout,
    RemovePageLayout,
    RevertPageLayout,
    /// Format › Line Numbering… (Ami Pro's Page › Line Numbering, CM7):
    /// Page Layout Properties at its Line numbering section.
    LineNumbering,
    NextFrame,
    Spelling,
    SpellAsYouType,
    Thesaurus,
    GrammarCheck,
    ReadabilityStatistics,
    Language,
    /// Tools › Sort… (TD.8): the Sort panel in the rail.
    Sort,
    WordCount,
    UpdateFields,
    CustomizeToolbar,
    Preferences,
    NextDocument,
    PreviousDocument,
    /// Help › Help (F1): the topic for where the user is (P6.4).
    Help,
    /// Help › Help Contents: the topics by area.
    HelpContents,
    /// Help › Search Help…: the search box of the Help panel.
    HelpSearch,
    KeyboardShortcuts,
    Compatibility,
    About,
    // Fields (`UI_SPEC.md` §28).
    DateTime,
    UpdateAllFields,
    NextField,
    PreviousField,
    ShowPowerFields,
    FieldShading,
    EditField,
    LockField,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutKey {
    Character(&'static str),
    Named(keyboard::key::Named),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shortcut {
    pub modifiers: &'static [Modifier],
    pub key: ShortcutKey,
}

impl Shortcut {
    pub fn key_bind(self) -> KeyBind {
        KeyBind {
            modifiers: self.modifiers.to_vec(),
            key: match self.key {
                ShortcutKey::Character(value) => Key::Character(value.into()),
                ShortcutKey::Named(value) => Key::Named(value),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandMetadata {
    pub id: CommandId,
    pub label: &'static str,
    pub menu: MenuRoot,
    pub shortcut: Option<Shortcut>,
    pub enabled: EnableRule,
    pub dispatch: DispatchRoute,
    pub toolbar: ToolbarRoute,
    pub implementation: Implementation,
}

const CTRL: &[Modifier] = &[Modifier::Ctrl];
const CTRL_SHIFT: &[Modifier] = &[Modifier::Ctrl, Modifier::Shift];
const ALT: &[Modifier] = &[Modifier::Alt];
const ALT_CTRL: &[Modifier] = &[Modifier::Alt, Modifier::Ctrl];
const SHIFT: &[Modifier] = &[Modifier::Shift];
const NONE: &[Modifier] = &[];

const fn character(modifiers: &'static [Modifier], key: &'static str) -> Option<Shortcut> {
    Some(Shortcut {
        modifiers,
        key: ShortcutKey::Character(key),
    })
}

const fn named(modifiers: &'static [Modifier], key: keyboard::key::Named) -> Option<Shortcut> {
    Some(Shortcut {
        modifiers,
        key: ShortcutKey::Named(key),
    })
}

macro_rules! command {
    ($id:ident, $label:literal, $menu:ident, $shortcut:expr, $enabled:ident, $dispatch:ident, $toolbar:ident, $implementation:ident) => {
        CommandMetadata {
            id: CommandId::$id,
            label: $label,
            menu: MenuRoot::$menu,
            shortcut: $shortcut,
            enabled: EnableRule::$enabled,
            dispatch: DispatchRoute::$dispatch,
            toolbar: ToolbarRoute::$toolbar,
            implementation: Implementation::$implementation,
        }
    };
}

/// Find & Replace (`UI_SPEC.md` §27): the find bar over the pages,
/// implemented with the page surface, future in the text editor.
const fn page_surface_find(
    id: CommandId,
    label: &'static str,
    shortcut: Option<Shortcut>,
) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu: MenuRoot::Edit,
        shortcut,
        enabled: if page_surface {
            EnableRule::HasDocument
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::Editor
        } else {
            DispatchRoute::Future
        },
        toolbar: if page_surface {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

/// The zoom commands: implemented with the page surface (Phase 2.6), future
/// in the default build.
const fn page_surface_view(
    id: CommandId,
    label: &'static str,
    shortcut: Option<Shortcut>,
) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu: MenuRoot::View,
        shortcut,
        enabled: if page_surface {
            EnableRule::HasDocument
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::View
        } else {
            DispatchRoute::Future
        },
        toolbar: if page_surface {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

/// Underline and paragraph alignment: implemented with the page surface,
/// which draws them (gaps A1, A2), and future (without a shortcut) in the
/// default build, whose editor cannot show them.
const fn page_surface_format(
    id: CommandId,
    label: &'static str,
    shortcut: Option<Shortcut>,
    enabled: EnableRule,
) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu: MenuRoot::Format,
        shortcut: if page_surface { shortcut } else { None },
        enabled: if page_surface {
            enabled
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::Format
        } else {
            DispatchRoute::Future
        },
        toolbar: if page_surface {
            ToolbarRoute::ActionBar
        } else {
            ToolbarRoute::Future
        },
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

/// The Table menu's commands (`UI_SPEC.md` §32, TD1): implemented with the
/// page surface, future in the text editor.
const fn table_command(id: CommandId, label: &'static str, enabled: EnableRule) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu: MenuRoot::Table,
        shortcut: None,
        enabled: if page_surface {
            enabled
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::Format
        } else {
            DispatchRoute::Future
        },
        toolbar: ToolbarRoute::None,
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

/// The Draw menu's commands (`UI_SPEC.md` §33, DW4): implemented with the
/// page surface, future in the text editor.
const fn draw_command(id: CommandId, label: &'static str) -> CommandMetadata {
    CommandMetadata {
        menu: MenuRoot::Draw,
        enabled: if cfg!(feature = "page-surface") {
            EnableRule::InDrawing
        } else {
            EnableRule::Future
        },
        ..table_command(id, label, EnableRule::InDrawing)
    }
}

/// Ami Pro's Text menu commands beyond Bold, Italic, and Underline: menu
/// only, implemented with the page surface (which draws them), future in
/// the default build.
const fn text_menu(
    id: CommandId,
    label: &'static str,
    shortcut: Option<Shortcut>,
) -> CommandMetadata {
    let format = page_surface_format(id, label, shortcut, EnableRule::TextFormat);
    CommandMetadata {
        toolbar: if cfg!(feature = "page-surface") {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        ..format
    }
}

/// A command of the page surface (fields, `UI_SPEC.md` §28): implemented
/// there, menu only; future (and without a shortcut) in the default build.
const fn page_surface_command(
    id: CommandId,
    label: &'static str,
    menu: MenuRoot,
    shortcut: Option<Shortcut>,
    enabled: EnableRule,
) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu,
        shortcut: if page_surface { shortcut } else { None },
        enabled: if page_surface {
            enabled
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::Editor
        } else {
            DispatchRoute::Future
        },
        toolbar: if page_surface {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

/// Frame commands (`UI_SPEC.md` §11): implemented with the page surface,
/// future (and without a shortcut) in the default build.
const fn page_surface_frame(
    id: CommandId,
    label: &'static str,
    menu: MenuRoot,
    shortcut: Option<Shortcut>,
    enabled: EnableRule,
) -> CommandMetadata {
    let page_surface = cfg!(feature = "page-surface");
    CommandMetadata {
        id,
        label,
        menu,
        shortcut: if page_surface { shortcut } else { None },
        enabled: if page_surface {
            enabled
        } else {
            EnableRule::Future
        },
        dispatch: if page_surface {
            DispatchRoute::Format
        } else {
            DispatchRoute::Future
        },
        toolbar: if page_surface {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        implementation: if page_surface {
            Implementation::Implemented
        } else {
            Implementation::Future
        },
    }
}

pub const COMMANDS: &[CommandMetadata] = &[
    command!(
        NewDocument,
        "New Document",
        File,
        character(CTRL, "n"),
        Always,
        File,
        ActionBar,
        Implemented
    ),
    command!(
        NewFromStyleSheet,
        "New…",
        File,
        None,
        Always,
        File,
        ActionBar,
        Implemented
    ),
    command!(
        Open,
        "Open…",
        File,
        character(CTRL, "o"),
        Always,
        File,
        ActionBar,
        Implemented
    ),
    command!(
        CloseDocument,
        "Close Document",
        File,
        character(CTRL, "w"),
        HasDocument,
        File,
        TabStrip,
        Implemented
    ),
    command!(
        Save,
        "Save",
        File,
        character(CTRL, "s"),
        Save,
        File,
        ActionBar,
        Implemented
    ),
    command!(
        SaveAs,
        "Save As…",
        File,
        character(CTRL_SHIFT, "s"),
        HasDocument,
        File,
        None,
        Implemented
    ),
    command!(
        RevertToSaved,
        "Revert to Saved…",
        File,
        None,
        RevertToSaved,
        File,
        None,
        Implemented
    ),
    command!(
        Password,
        "Password…",
        File,
        None,
        HasDocument,
        File,
        None,
        Implemented
    ),
    // Print and PDF (`UI_SPEC.md` §29): the pages as they print.
    page_surface_command(
        CommandId::ExportPdf,
        "Export PDF…",
        MenuRoot::File,
        character(CTRL_SHIFT, "e"),
        EnableRule::HasDocument,
    ),
    // Export (`UI_SPEC.md` §30, EX2): a copy in another program's format,
    // through the exchange crate's filters; the tab keeps its own file.
    page_surface_command(
        CommandId::ExportDocx,
        "Export Word Document (.docx)…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ExportOdt,
        "Export OpenDocument Text (.odt)…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ExportRtf,
        "Export Rich Text (.rtf)…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ExportText,
        "Export Plain Text (.txt)…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ExportMarkdown,
        "Export Markdown (.md)…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    // A whole folder of Ami Pro documents (EX6): the Export Folder panel.
    page_surface_command(
        CommandId::ExportFolder,
        "Export Folder…",
        MenuRoot::File,
        None,
        EnableRule::Always,
    ),
    // Other programs' documents, a folder at once (`UI_SPEC.md` §31, IM11).
    page_surface_command(
        CommandId::ImportFolder,
        "Import Folder…",
        MenuRoot::File,
        None,
        EnableRule::Always,
    ),
    page_surface_command(
        CommandId::Print,
        "Print…",
        MenuRoot::File,
        character(CTRL, "p"),
        EnableRule::HasDocument,
    ),
    // Ami Pro's Print › Current page, which the desktop's dialog cannot
    // offer: the dialog with the caret's page as its range.
    page_surface_command(
        CommandId::PrintCurrentPage,
        "Print Current Page…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's File › Page Setup…: the Page Layout Properties panel, as
    // Format › Page Layout… opens it.
    page_surface_command(
        CommandId::PageSetup,
        "Page Setup…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's Print › Options…: the Print panel in the rail.
    page_surface_command(
        CommandId::PrintOptions,
        "Print Options…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's File › Doc Info…: Document Properties in the rail (FL3).
    page_surface_command(
        CommandId::DocumentProperties,
        "Document Properties…",
        MenuRoot::File,
        named(ALT, keyboard::key::Named::Enter),
        EnableRule::HasDocument,
    ),
    command!(
        Quit,
        "Quit",
        File,
        character(CTRL, "q"),
        Always,
        File,
        None,
        Implemented
    ),
    command!(
        Undo,
        "Undo",
        Edit,
        character(CTRL, "z"),
        Undo,
        Editor,
        ActionBar,
        Implemented
    ),
    command!(
        Redo,
        "Redo",
        Edit,
        character(CTRL_SHIFT, "z"),
        Redo,
        Editor,
        ActionBar,
        Implemented
    ),
    command!(
        Cut,
        "Cut",
        Edit,
        character(CTRL, "x"),
        Selection,
        Editor,
        ActionBar,
        Implemented
    ),
    command!(
        Copy,
        "Copy",
        Edit,
        character(CTRL, "c"),
        Selection,
        Editor,
        ActionBar,
        Implemented
    ),
    command!(
        Paste,
        "Paste",
        Edit,
        character(CTRL, "v"),
        Paste,
        Editor,
        ActionBar,
        Implemented
    ),
    command!(
        PastePlainText,
        "Paste as Plain Text",
        Edit,
        character(CTRL_SHIFT, "v"),
        Paste,
        Editor,
        None,
        Implemented
    ),
    command!(
        Delete,
        "Delete",
        Edit,
        named(NONE, keyboard::key::Named::Delete),
        Selection,
        Editor,
        None,
        Implemented
    ),
    command!(
        SelectAll,
        "Select All",
        Edit,
        character(CTRL, "a"),
        HasText,
        Editor,
        None,
        Implemented
    ),
    page_surface_find(CommandId::Find, "Find…", character(CTRL, "f")),
    page_surface_find(
        CommandId::FindNext,
        "Find Next",
        named(NONE, keyboard::key::Named::F3),
    ),
    page_surface_find(
        CommandId::FindPrevious,
        "Find Previous",
        named(SHIFT, keyboard::key::Named::F3),
    ),
    page_surface_find(CommandId::Replace, "Replace…", character(CTRL, "h")),
    // Ctrl+G is Ami Pro's Go To, whose dialog also offers Next frame; the
    // status bar's page number opens it, and its arrows move a page
    // (user's guide chapter 1, "Using the status bar"; user, 2026-10-12).
    page_surface_frame(
        CommandId::GoToPage,
        "Go To…",
        MenuRoot::Edit,
        character(CTRL, "g"),
        EnableRule::HasDocument,
    ),
    // Ami Pro's Page › Header/Footer… Edit Header or Footer: the caret in
    // the page's fixed header or footer (`UI_SPEC.md` §18).
    page_surface_frame(
        CommandId::GoToHeader,
        "Go To Header",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    // Edit › Go To › Ruler in Ami Pro: the keyboard's way into the ruler
    // (user's guide, ch. 8; `UI_SPEC.md` §19).
    page_surface_frame(
        CommandId::GoToRuler,
        "Go To Ruler",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::GoToFooter,
        "Go To Footer",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::PreviousPage,
        "Previous Page",
        MenuRoot::Edit,
        None,
        EnableRule::PreviousPage,
    ),
    page_surface_frame(
        CommandId::NextPage,
        "Next Page",
        MenuRoot::Edit,
        None,
        EnableRule::NextPage,
    ),
    page_surface_frame(
        CommandId::NextFrame,
        "Select Next Frame",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    command!(
        ShowToolbar,
        "Toolbar",
        View,
        None,
        Always,
        View,
        None,
        Implemented
    ),
    command!(
        StylesPanel,
        "Styles Panel",
        View,
        None,
        Always,
        View,
        StylesPanel,
        Implemented
    ),
    page_surface_view(CommandId::Ruler, "Ruler", None),
    page_surface_view(CommandId::StatusBar, "Status Bar", None),
    page_surface_view(CommandId::ZoomIn, "Zoom In", character(CTRL, "+")),
    page_surface_view(CommandId::ZoomOut, "Zoom Out", character(CTRL, "-")),
    page_surface_view(CommandId::ActualSize, "Actual Size", character(CTRL, "0")),
    page_surface_view(CommandId::FitPageWidth, "Fit Page Width", None),
    page_surface_view(CommandId::FullPage, "Full Page", character(CTRL, "d")),
    page_surface_view(CommandId::Enlarged, "Enlarged (200%)", None),
    page_surface_view(CommandId::CustomZoom, "Custom Zoom", None),
    page_surface_view(
        CommandId::DraftMode,
        "Draft Mode",
        character(CTRL_SHIFT, "d"),
    ),
    page_surface_view(CommandId::OutlineMode, "Outline Mode", None),
    page_surface_view(
        CommandId::OutlineExpand,
        "Expand",
        named(ALT, keyboard::key::Named::PageDown),
    ),
    page_surface_view(
        CommandId::OutlineContract,
        "Contract",
        named(ALT, keyboard::key::Named::PageUp),
    ),
    page_surface_view(CommandId::ShowLevel1, "Show 1 Level", None),
    page_surface_view(CommandId::ShowLevel2, "Show 2 Levels", None),
    page_surface_view(CommandId::ShowLevel3, "Show 3 Levels", None),
    page_surface_view(CommandId::ShowLevel4, "Show 4 Levels", None),
    page_surface_view(CommandId::ShowLevel5, "Show 5 Levels", None),
    page_surface_view(CommandId::ShowLevel6, "Show 6 Levels", None),
    page_surface_view(CommandId::ShowLevel7, "Show 7 Levels", None),
    page_surface_view(CommandId::ShowLevel8, "Show 8 Levels", None),
    page_surface_view(CommandId::ShowLevel9, "Show 9 Levels", None),
    page_surface_view(CommandId::ShowAllLevels, "Show All Levels", None),
    page_surface_view(CommandId::UseOutlineStyles, "Use Outline Styles", None),
    page_surface_view(CommandId::OutlineNavigator, "Outline Navigator", None),
    page_surface_view(CommandId::ContinuousPages, "Continuous Pages", None),
    page_surface_view(CommandId::TwoPages, "Facing Pages", None),
    command!(
        FullScreen,
        "Show Clean Screen",
        View,
        named(NONE, keyboard::key::Named::F11),
        Always,
        View,
        None,
        Implemented
    ),
    // Ami Pro's Page › Breaks: a page break at the caret, a column break
    // at the start of the caret's paragraph (VM fixtures PB1, CB1).
    page_surface_frame(
        CommandId::PageBreak,
        "Page Break",
        MenuRoot::Insert,
        named(CTRL, keyboard::key::Named::Enter),
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::ColumnBreak,
        "Column Break",
        MenuRoot::Insert,
        named(CTRL_SHIFT, keyboard::key::Named::Enter),
        EnableRule::HasDocument,
    ),
    // Ami Pro's Frame › Create Frame… (the Create Frame dialog) and its
    // frame SmartIcon (drawing on the page): `UI_SPEC.md` §20, decision F1.
    page_surface_frame(
        CommandId::Frame,
        "Frame…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::DrawFrame,
        "Draw Frame",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's Tools › Tables… (`UI_SPEC.md` §32).
    page_surface_frame(
        CommandId::Table,
        "Table…",
        MenuRoot::Insert,
        named(CTRL, keyboard::key::Named::F12),
        EnableRule::TableTarget,
    ),
    table_command(
        CommandId::InsertRowAbove,
        "Insert Row Above",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::InsertRowBelow,
        "Insert Row Below",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::InsertColumnLeft,
        "Insert Column Left",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::InsertColumnRight,
        "Insert Column Right",
        EnableRule::InTable,
    ),
    table_command(CommandId::DeleteRow, "Delete Row", EnableRule::InTable),
    table_command(
        CommandId::DeleteColumn,
        "Delete Column",
        EnableRule::InTable,
    ),
    table_command(CommandId::DeleteTable, "Delete Table", EnableRule::InTable),
    table_command(CommandId::SelectColumn, "Column", EnableRule::InTable),
    table_command(CommandId::SelectRow, "Row", EnableRule::InTable),
    table_command(CommandId::SelectTable, "Entire Table", EnableRule::InTable),
    table_command(
        CommandId::ConnectCells,
        "Connect Cells",
        EnableRule::InTable,
    ),
    table_command(CommandId::TableHeadings, "Headings", EnableRule::InTable),
    table_command(CommandId::LeaderNone, "None", EnableRule::InTable),
    table_command(CommandId::LeaderDots, "Dots  . . .", EnableRule::InTable),
    table_command(
        CommandId::LeaderDashes,
        "Dashes  - - -",
        EnableRule::InTable,
    ),
    table_command(CommandId::LeaderLine, "Line  ___", EnableRule::InTable),
    table_command(
        CommandId::ProtectCells,
        "Protect Cells",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::DistributeRows,
        "Distribute Rows Evenly",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::DistributeColumns,
        "Distribute Columns Evenly",
        EnableRule::InTable,
    ),
    // The rows below the headings by the caret's column (MR1, for merge
    // data files as tables).
    table_command(
        CommandId::SortAscending,
        "Sort Ascending",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::SortDescending,
        "Sort Descending",
        EnableRule::InTable,
    ),
    // Run C (`UI_SPEC.md` §41): formulas.
    table_command(CommandId::EditFormula, "Edit Formula…", EnableRule::InTable),
    table_command(CommandId::QuickAddRow, "Row", EnableRule::InTable),
    table_command(CommandId::QuickAddColumn, "Column", EnableRule::InTable),
    table_command(
        CommandId::RecalculateTable,
        "Recalculate Table",
        EnableRule::InTable,
    ),
    table_command(CommandId::TableStyleGrid, "Grid", EnableRule::InTable),
    table_command(CommandId::TableStyleOutline, "Outline", EnableRule::InTable),
    table_command(
        CommandId::TableStyleHeaderRow,
        "Header Row",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::TableStyleBandedRows,
        "Banded Rows",
        EnableRule::InTable,
    ),
    table_command(CommandId::TableStylePlain, "Plain", EnableRule::InTable),
    table_command(
        CommandId::TextToTable,
        "Convert Text to Table",
        EnableRule::TextToTable,
    ),
    table_command(
        CommandId::TableToText,
        "Convert Table to Text",
        EnableRule::InTable,
    ),
    table_command(
        CommandId::TableProperties,
        "Table Properties…",
        EnableRule::InTable,
    ),
    // Ami Pro's File › Import Picture… (`UI_SPEC.md` §25, decisions P1–P3).
    page_surface_frame(
        CommandId::Image,
        "Image…",
        MenuRoot::Insert,
        character(CTRL_SHIFT, "i"),
        EnableRule::PictureTarget,
    ),
    // Ami Pro's File › Open › Insert (`UI_SPEC.md` §31, IM1, IM3): a
    // file's text at the caret, after Import Options.
    page_surface_command(
        CommandId::InsertFile,
        "File…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's Tools › Drawing (`UI_SPEC.md` §33).
    page_surface_command(
        CommandId::Drawing,
        "Drawing…",
        MenuRoot::Insert,
        None,
        EnableRule::DrawingTarget,
    ),
    draw_command(CommandId::DrawProperties, "Line and Fill…"),
    draw_command(CommandId::DrawExtract, "Extract Line & Fill"),
    draw_command(CommandId::DrawApply, "Apply Line & Fill"),
    draw_command(CommandId::DrawRotateLeft, "Rotate Left 90°"),
    draw_command(CommandId::DrawRotateRight, "Rotate Right 90°"),
    draw_command(CommandId::DrawRotate, "Rotate by Step"),
    draw_command(CommandId::DrawFlipHorizontal, "Horizontal"),
    draw_command(CommandId::DrawFlipVertical, "Vertical"),
    draw_command(CommandId::DrawGroup, "Group"),
    draw_command(CommandId::DrawUngroup, "Ungroup"),
    draw_command(CommandId::DrawFront, "Bring to Front"),
    draw_command(CommandId::DrawBack, "Send to Back"),
    draw_command(CommandId::DrawForward, "Bring Forward"),
    draw_command(CommandId::DrawBackward, "Send Backward"),
    draw_command(CommandId::DrawAlignLeft, "Left"),
    draw_command(CommandId::DrawAlignCentre, "Centre"),
    draw_command(CommandId::DrawAlignRight, "Right"),
    draw_command(CommandId::DrawAlignTop, "Top"),
    draw_command(CommandId::DrawAlignMiddle, "Middle"),
    draw_command(CommandId::DrawAlignBottom, "Bottom"),
    draw_command(CommandId::DrawDistributeAcross, "Across"),
    draw_command(CommandId::DrawDistributeDown, "Down"),
    draw_command(CommandId::DrawDuplicate, "Duplicate"),
    draw_command(CommandId::DrawSnap, "Snap To"),
    draw_command(CommandId::DrawGrid, "Show Grid"),
    draw_command(CommandId::DrawSelectAll, "Select All Objects"),
    draw_command(CommandId::ImportDrawing, "Import Drawing…"),
    draw_command(CommandId::SaveDrawingAs, "Save Drawing As…"),
    draw_command(CommandId::DrawDone, "Done Drawing"),
    // Ami Pro's Page › Header/Footer › Floating Header/Footer and Page ›
    // Page Numbering, as small one-shot dialogs (decision L3).
    page_surface_frame(
        CommandId::HeaderFooter,
        "Floating Header/Footer…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::PageNumber,
        "Page Number…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    // The notes run (`UI_SPEC.md` §34): Ami Pro's Tools › Footnotes'
    // Insert footnote, Edit › Insert › Note, and the rest; Insert ›
    // Endnote left the menu (NR9: endnotes are Footnote Options' choice).
    page_surface_command(
        CommandId::Footnote,
        "Footnote",
        MenuRoot::Insert,
        character(ALT_CTRL, "f"),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::InsertNote,
        "Note",
        MenuRoot::Insert,
        character(ALT_CTRL, "m"),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::CrossReference,
        "Cross-Reference…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::Notes,
        "Notes…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's Edit › Paste Special… (0x24c): the formats the clipboard
    // offers (`UI_SPEC.md` §42.2). Paste Link and New Object are not offered.
    page_surface_command(
        CommandId::PasteSpecial,
        "Paste Special…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ImageProcessing,
        "Image Processing…",
        MenuRoot::Tools,
        None,
        EnableRule::BitmapSelected,
    ),
    page_surface_command(
        CommandId::Charting,
        "Charting…",
        MenuRoot::Tools,
        None,
        EnableRule::Charting,
    ),
    page_surface_command(
        CommandId::ChartFromTable,
        "Chart from Table",
        MenuRoot::Tools,
        None,
        EnableRule::InTable,
    ),
    page_surface_command(
        CommandId::Equations,
        "Equations…",
        MenuRoot::Tools,
        None,
        EnableRule::Equations,
    ),
    page_surface_command(
        CommandId::PasteLatexEquation,
        "Paste LaTeX Equation",
        MenuRoot::Tools,
        None,
        EnableRule::Equations,
    ),
    page_surface_command(
        CommandId::Links,
        "Links…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::RecoverDocuments,
        "Recover Documents…",
        MenuRoot::File,
        None,
        EnableRule::Always,
    ),
    page_surface_command(
        CommandId::CheckForUpdates,
        "Check for Updates…",
        MenuRoot::Help,
        None,
        EnableRule::Always,
    ),
    page_surface_command(
        CommandId::Bookmarks,
        "Bookmarks…",
        MenuRoot::Edit,
        named(CTRL_SHIFT, keyboard::key::Named::F5),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::FootnoteOptions,
        "Footnote Options…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::MasterDocument,
        "Master Document…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::Merge,
        "Merge…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::PrintEnvelope,
        "Print Envelope…",
        MenuRoot::File,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::MergeField,
        "Merge Field…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ContentsIndex,
        "TOC, Index…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::MarkIndexEntry,
        "Index Entry…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::MarkTocEntry,
        "TOC Entry…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::MarkGlossary,
        "Glossary…",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::InsertGlossary,
        "Glossary Record…",
        MenuRoot::Insert,
        character(CTRL, "k"),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ShowNotes,
        "Notes",
        MenuRoot::View,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ShowBookmarks,
        "Bookmarks",
        MenuRoot::View,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_view(CommandId::ShowTabsReturns, "Tabs & Returns", None),
    page_surface_view(CommandId::ColumnGuides, "Column Guides", None),
    page_surface_view(CommandId::MarginsInColour, "Margins in Colour", None),
    page_surface_view(CommandId::ShowPictures, "Pictures", None),
    page_surface_view(CommandId::TableAddresses, "Table Row/Column Headings", None),
    // Ami Pro's Edit › Insert › Bullet… (`UI_SPEC.md` §21, B4).
    page_surface_frame(
        CommandId::InsertBullet,
        "Bullet…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    // The Fields panel in the rail (FL2): Ami Pro's Edit › Power Fields ›
    // Insert… and Insert › Date/Time….
    page_surface_command(
        CommandId::Field,
        "Field…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::DateTime,
        "Date/Time…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::SpecialCharacter,
        "Special Character…",
        MenuRoot::Insert,
        None,
        EnableRule::HasDocument,
    ),
    // Format › Styles ▸, Ami Pro's Style menu (`UI_SPEC.md` §14). Ami
    // Pro's Ctrl+A (Modify Style) and Ctrl+Y (Select a Style) are Select
    // All and Redo here.
    command!(
        CreateStyle,
        "Create Style…",
        Format,
        None,
        HasDocument,
        Format,
        None,
        Implemented
    ),
    // Modify Style's sample is drawn by the page surface.
    CommandMetadata {
        toolbar: if cfg!(feature = "page-surface") {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        ..page_surface_format(
            CommandId::ModifyCurrentStyle,
            "Modify Style…",
            None,
            EnableRule::HasDocument,
        )
    },
    // Ami Pro's Outline Styles' Quick Numbering (`UI_SPEC.md` §22, O3).
    page_surface_frame(
        CommandId::OutlineNumbering,
        "Outline Numbering…",
        MenuRoot::Format,
        None,
        EnableRule::HasDocument,
    ),
    command!(
        UseAnotherStyleSheet,
        "Use Another Style Sheet…",
        Format,
        None,
        HasDocument,
        Format,
        None,
        Implemented
    ),
    command!(
        SaveAsStyleSheet,
        "Save as a Style Sheet…",
        Format,
        None,
        HasDocument,
        Format,
        None,
        Implemented
    ),
    command!(
        StyleManagement,
        "Style Management…",
        Format,
        None,
        HasDocument,
        Format,
        None,
        Implemented
    ),
    command!(
        Styles,
        "Select a Style",
        Format,
        character(CTRL, "y"),
        HasDocument,
        Format,
        StylesPanel,
        Implemented
    ),
    command!(
        Bold,
        "Bold",
        Format,
        character(CTRL, "b"),
        TextFormat,
        Format,
        ActionBar,
        Implemented
    ),
    command!(
        Italic,
        "Italic",
        Format,
        character(CTRL, "i"),
        TextFormat,
        Format,
        ActionBar,
        Implemented
    ),
    page_surface_format(
        CommandId::Underline,
        "Underline",
        character(CTRL, "u"),
        EnableRule::TextFormat,
    ),
    // Ami Pro's Text menu items (user decision, 2026-10-12): Ami Pro's
    // Ctrl+W (Word Underline) and Ctrl+N (Normal) are Close Document and
    // New here, so Word Underline has none and Normal keeps Ctrl+M.
    text_menu(CommandId::WordUnderline, "Word Underline", None),
    text_menu(CommandId::UpperCase, "Upper Case", None),
    text_menu(CommandId::LowerCase, "Lower Case", None),
    text_menu(CommandId::InitialCaps, "Initial Caps", None),
    text_menu(CommandId::SmallCaps, "Small Caps", None),
    text_menu(CommandId::SpecialEffects, "Special Effects…", None),
    CommandMetadata {
        menu: MenuRoot::Edit,
        ..text_menu(CommandId::ProtectedText, "Protected Text", None)
    },
    CommandMetadata {
        menu: MenuRoot::Edit,
        ..text_menu(CommandId::NoHyphenation, "No Hyphenation", None)
    },
    text_menu(CommandId::Font, "Font…", None),
    CommandMetadata {
        toolbar: if cfg!(feature = "page-surface") {
            ToolbarRoute::ActionBar
        } else {
            ToolbarRoute::Future
        },
        ..text_menu(
            CommandId::ClearDirectFormatting,
            "Normal",
            character(CTRL, "m"),
        )
    },
    page_surface_format(
        CommandId::InsertRuler,
        "Insert Ruler",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::RemoveRuler,
        "Remove Ruler",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::InsertPageLayout,
        "Insert Page Layout",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::RemovePageLayout,
        "Remove Page Layout",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::RevertPageLayout,
        "Revert to Standard Page Layout",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::LineNumbering,
        "Line Numbering…",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::FastFormat,
        "Fast Format",
        character(CTRL, "t"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::ParagraphProperties,
        "Paragraph…",
        None,
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::AlignLeft,
        "Align Left",
        character(CTRL, "l"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::Center,
        "Center",
        character(CTRL, "e"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::AlignRight,
        "Align Right",
        character(CTRL, "r"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::Justify,
        "Justify",
        character(CTRL, "j"),
        EnableRule::HasDocument,
    ),
    // Ami Pro has no list commands; these apply its list styles
    // (`UI_SPEC.md` §21, decisions B1 and B2).
    page_surface_format(
        CommandId::Bullets,
        "Bullets",
        character(CTRL_SHIFT, "l"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::Numbering,
        "Numbering",
        character(CTRL_SHIFT, "n"),
        EnableRule::HasDocument,
    ),
    // Style Properties at its Bullets and numbers section (B5).
    page_surface_frame(
        CommandId::BulletNumberStyle,
        "Bullet and Number Style…",
        MenuRoot::Format,
        None,
        EnableRule::HasDocument,
    ),
    // Ami Pro's Outline mode keys, on the page (`UI_SPEC.md` §22, O1, O4).
    page_surface_format(
        CommandId::Promote,
        "Promote",
        named(ALT, keyboard::key::Named::ArrowLeft),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::Demote,
        "Demote",
        named(ALT, keyboard::key::Named::ArrowRight),
        EnableRule::HasDocument,
    ),
    // In a table cell the page takes these keys for its rows (§32).
    page_surface_format(
        CommandId::MoveUp,
        "Move Up",
        named(ALT, keyboard::key::Named::ArrowUp),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::MoveDown,
        "Move Down",
        named(ALT, keyboard::key::Named::ArrowDown),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::IncreaseIndent,
        "Increase Indent",
        character(CTRL, "]"),
        EnableRule::HasDocument,
    ),
    page_surface_format(
        CommandId::DecreaseIndent,
        "Decrease Indent",
        character(CTRL, "["),
        EnableRule::HasDocument,
    ),
    page_surface_frame(
        CommandId::FrameProperties,
        "Frame Properties…",
        MenuRoot::Format,
        None,
        EnableRule::FrameSelected,
    ),
    // Ami Pro's Frame › Graphics Scaling…: Frame Properties › Picture in
    // the rail (`UI_SPEC.md` §25, P5).
    page_surface_frame(
        CommandId::GraphicsScaling,
        "Graphics Scaling…",
        MenuRoot::Format,
        None,
        EnableRule::PictureSelected,
    ),
    page_surface_frame(
        CommandId::BringToFront,
        "Bring to Front",
        MenuRoot::Format,
        None,
        EnableRule::FrameSelected,
    ),
    page_surface_frame(
        CommandId::SendToBack,
        "Send to Back",
        MenuRoot::Format,
        None,
        EnableRule::FrameSelected,
    ),
    page_surface_frame(
        CommandId::GroupFrames,
        "Group Frames",
        MenuRoot::Format,
        None,
        EnableRule::FramesGroupable,
    ),
    page_surface_frame(
        CommandId::UngroupFrames,
        "Ungroup Frames",
        MenuRoot::Format,
        None,
        EnableRule::FrameGroup,
    ),
    // Ami Pro's Page › Modify Page Layout, under Format (decision L2):
    // Page Layout Properties in the rail, which the page surface lays out.
    CommandMetadata {
        toolbar: if cfg!(feature = "page-surface") {
            ToolbarRoute::None
        } else {
            ToolbarRoute::Future
        },
        ..page_surface_format(
            CommandId::PageLayout,
            "Page Layout…",
            None,
            EnableRule::HasDocument,
        )
    },
    // Proofing (`UI_SPEC.md` §35): the Proofing panel in the rail.
    page_surface_command(
        CommandId::Spelling,
        "Spell Check…",
        MenuRoot::Tools,
        named(NONE, keyboard::key::Named::F7),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::SpellAsYouType,
        "Check Spelling as You Type",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::Thesaurus,
        "Thesaurus…",
        MenuRoot::Tools,
        named(SHIFT, keyboard::key::Named::F7),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::GrammarCheck,
        "Grammar Check…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ReadabilityStatistics,
        "Readability Statistics…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::Language,
        "Language…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::Sort,
        "Sort…",
        MenuRoot::Tools,
        None,
        EnableRule::HasDocument,
    ),
    // Document Properties at its statistics.
    page_surface_command(
        CommandId::WordCount,
        "Word Count",
        MenuRoot::Tools,
        character(CTRL_SHIFT, "c"),
        EnableRule::HasDocument,
    ),
    // Ami Pro's Edit › Power Fields ▸ Update and Update All (FL4).
    page_surface_command(
        CommandId::UpdateFields,
        "Update Fields",
        MenuRoot::Edit,
        named(NONE, keyboard::key::Named::F9),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::UpdateAllFields,
        "Update All Fields",
        MenuRoot::Edit,
        named(CTRL, keyboard::key::Named::F9),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::NextField,
        "Next Field",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::PreviousField,
        "Previous Field",
        MenuRoot::Edit,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::ShowPowerFields,
        "Show Power Fields",
        MenuRoot::View,
        named(ALT, keyboard::key::Named::F9),
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::FieldShading,
        "Field Shading",
        MenuRoot::View,
        None,
        EnableRule::HasDocument,
    ),
    page_surface_command(
        CommandId::EditField,
        "Edit Field…",
        MenuRoot::Edit,
        None,
        EnableRule::OnField,
    ),
    page_surface_command(
        CommandId::LockField,
        "Lock Field",
        MenuRoot::Edit,
        None,
        EnableRule::OnField,
    ),
    command!(
        CustomizeToolbar,
        "Customize Toolbar…",
        Tools,
        None,
        Always,
        View,
        None,
        Implemented
    ),
    command!(
        Preferences,
        "Preferences…",
        Tools,
        character(CTRL, ","),
        Always,
        Help,
        None,
        Implemented
    ),
    command!(
        NextDocument,
        "Next Document",
        Window,
        named(CTRL, keyboard::key::Named::PageDown),
        MultipleTabs,
        Window,
        TabStrip,
        Implemented
    ),
    command!(
        PreviousDocument,
        "Previous Document",
        Window,
        named(CTRL_SHIFT, keyboard::key::Named::Tab),
        MultipleTabs,
        Window,
        TabStrip,
        Implemented
    ),
    page_surface_command(
        CommandId::Help,
        "Help",
        MenuRoot::Help,
        named(NONE, keyboard::key::Named::F1),
        EnableRule::Always,
    ),
    page_surface_command(
        CommandId::HelpContents,
        "Help Contents",
        MenuRoot::Help,
        None,
        EnableRule::Always,
    ),
    page_surface_command(
        CommandId::HelpSearch,
        "Search Help…",
        MenuRoot::Help,
        None,
        EnableRule::Always,
    ),
    command!(
        KeyboardShortcuts,
        "Keyboard Shortcuts",
        Help,
        None,
        Always,
        Help,
        None,
        Implemented
    ),
    command!(
        Compatibility,
        "Compatibility and Fidelity",
        Help,
        None,
        HasDocument,
        Help,
        None,
        Implemented
    ),
    command!(
        About,
        "About AmiPro-Linux",
        Help,
        None,
        Always,
        Help,
        None,
        Implemented
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuSlot {
    Command(CommandId),
    /// A submenu.
    Folder(&'static str, &'static [MenuSlot]),
    Divider,
    Documents,
    /// File › 1 NAME…: the recent files (Preferences, CM6).
    Recent,
}

impl MenuSlot {
    /// The commands of `slots` in order, submenus included.
    pub fn commands(slots: &[MenuSlot]) -> Vec<CommandId> {
        slots
            .iter()
            .flat_map(|slot| match slot {
                MenuSlot::Command(command) => vec![*command],
                MenuSlot::Folder(_, children) => MenuSlot::commands(children),
                MenuSlot::Divider | MenuSlot::Documents | MenuSlot::Recent => Vec::new(),
            })
            .collect()
    }
}

use CommandId as C;
use MenuSlot::{Command as Item, Divider, Documents, Folder, Recent};

const FILE_MENU: &[MenuSlot] = &[
    Item(C::NewDocument),
    Item(C::NewFromStyleSheet),
    Item(C::Open),
    Item(C::ImportFolder),
    Divider,
    Item(C::CloseDocument),
    Divider,
    Item(C::Save),
    Item(C::SaveAs),
    Item(C::RevertToSaved),
    Item(C::RecoverDocuments),
    Item(C::Password),
    Divider,
    Folder("Export", EXPORT_MENU),
    Item(C::PageSetup),
    Item(C::Print),
    Item(C::PrintCurrentPage),
    Item(C::PrintOptions),
    Item(C::PrintEnvelope),
    Divider,
    Item(C::Merge),
    Item(C::MasterDocument),
    Item(C::DocumentProperties),
    Divider,
    Recent,
    Item(C::Quit),
];
/// File › Export ▸ (`UI_SPEC.md` §30, EX2).
const EXPORT_MENU: &[MenuSlot] = &[
    Item(C::ExportPdf),
    Item(C::ExportDocx),
    Item(C::ExportOdt),
    Item(C::ExportRtf),
    Item(C::ExportText),
    Item(C::ExportMarkdown),
    Divider,
    Item(C::ExportFolder),
];
const EDIT_MENU: &[MenuSlot] = &[
    Item(C::Undo),
    Item(C::Redo),
    Divider,
    Item(C::Cut),
    Item(C::Copy),
    Item(C::Paste),
    Item(C::PasteSpecial),
    Item(C::PastePlainText),
    Item(C::Delete),
    Divider,
    Item(C::SelectAll),
    Divider,
    Item(C::Find),
    Item(C::FindNext),
    Item(C::FindPrevious),
    Item(C::Replace),
    Item(C::GoToPage),
    Item(C::GoToHeader),
    Item(C::GoToFooter),
    Item(C::GoToRuler),
    Item(C::PreviousPage),
    Item(C::NextPage),
    Item(C::NextFrame),
    Divider,
    Item(C::Bookmarks),
    Item(C::Notes),
    Item(C::Links),
    Folder("Power Fields", FIELDS_MENU),
    Folder("Mark Text", MARK_TEXT_MENU),
];
/// Edit › Mark Text ▸, as Ami Pro's (`UI_SPEC.md` §36).
const MARK_TEXT_MENU: &[MenuSlot] = &[
    Item(C::ProtectedText),
    Item(C::NoHyphenation),
    Divider,
    Item(C::MarkIndexEntry),
    Item(C::MarkTocEntry),
    Item(C::MarkGlossary),
];
/// Edit › Power Fields ▸, as Ami Pro's (its Insert… is Insert › Field…;
/// `UI_SPEC.md` §28).
const FIELDS_MENU: &[MenuSlot] = &[
    Item(C::UpdateFields),
    Item(C::UpdateAllFields),
    Divider,
    Item(C::NextField),
    Item(C::PreviousField),
    Divider,
    Item(C::EditField),
    Item(C::LockField),
];
const VIEW_MENU: &[MenuSlot] = &[
    Item(C::ShowToolbar),
    Item(C::StylesPanel),
    Item(C::Ruler),
    Item(C::StatusBar),
    Divider,
    Folder("Show", SHOW_MENU),
    Divider,
    Item(C::ZoomIn),
    Item(C::ZoomOut),
    Item(C::FullPage),
    Item(C::ActualSize),
    Item(C::Enlarged),
    Item(C::CustomZoom),
    Item(C::FitPageWidth),
    Divider,
    Item(C::ContinuousPages),
    Item(C::TwoPages),
    Item(C::OutlineMode),
    Folder("Outline", OUTLINE_MENU),
    Item(C::DraftMode),
    Divider,
    Item(C::FullScreen),
];
/// View › Outline ▸ (OB.4): Ami Pro's Outline menu and bar, for Outline
/// mode (Promote, Demote, Move Up, and Move Down are in Bullets and
/// Numbering ▸, as they work in both modes).
const OUTLINE_MENU: &[MenuSlot] = &[
    Item(C::OutlineExpand),
    Item(C::OutlineContract),
    Divider,
    Item(C::ShowLevel1),
    Item(C::ShowLevel2),
    Item(C::ShowLevel3),
    Item(C::ShowLevel4),
    Item(C::ShowLevel5),
    Item(C::ShowLevel6),
    Item(C::ShowLevel7),
    Item(C::ShowLevel8),
    Item(C::ShowLevel9),
    Item(C::ShowAllLevels),
    Divider,
    Item(C::UseOutlineStyles),
    Item(C::OutlineNavigator),
];
/// View › Show ▸, as Ami Pro's View Preferences' marks (CM10).
const SHOW_MENU: &[MenuSlot] = &[
    Item(C::ShowTabsReturns),
    Item(C::ColumnGuides),
    Item(C::MarginsInColour),
    Item(C::ShowPictures),
    Item(C::TableAddresses),
    Divider,
    Item(C::ShowPowerFields),
    Item(C::FieldShading),
    Item(C::ShowNotes),
    Item(C::ShowBookmarks),
];
const INSERT_MENU: &[MenuSlot] = &[
    Item(C::PageBreak),
    Item(C::ColumnBreak),
    Divider,
    Item(C::Frame),
    Item(C::DrawFrame),
    Item(C::Table),
    Item(C::Image),
    Item(C::InsertFile),
    Item(C::Drawing),
    Divider,
    Item(C::HeaderFooter),
    Item(C::PageNumber),
    Item(C::Footnote),
    Item(C::InsertNote),
    Item(C::CrossReference),
    Divider,
    Item(C::InsertBullet),
    Item(C::DateTime),
    Item(C::Field),
    Item(C::MergeField),
    Item(C::InsertGlossary),
    Item(C::SpecialCharacter),
];
const FORMAT_MENU: &[MenuSlot] = &[
    Folder("Styles", STYLE_MENU),
    Divider,
    Item(C::Bold),
    Item(C::Italic),
    Item(C::Underline),
    Item(C::WordUnderline),
    Folder("Caps", CAPS_MENU),
    Item(C::SpecialEffects),
    Item(C::Font),
    Item(C::ClearDirectFormatting),
    Item(C::FastFormat),
    Divider,
    Item(C::ParagraphProperties),
    Folder("Alignment", ALIGNMENT_MENU),
    Folder("Bullets and Numbering", LISTS_MENU),
    Item(C::IncreaseIndent),
    Item(C::DecreaseIndent),
    Divider,
    Folder("Frame", FRAME_MENU),
    Divider,
    Item(C::PageLayout),
    Folder("Page Layout Changes", PAGE_LAYOUT_MENU),
    Folder("Ruler", RULER_MENU),
    Item(C::LineNumbering),
];
/// Format › Page Layout Changes ▸: Ami Pro's Page › Insert Page Layout ▸.
const PAGE_LAYOUT_MENU: &[MenuSlot] = &[
    Item(C::InsertPageLayout),
    Item(C::RemovePageLayout),
    Item(C::RevertPageLayout),
];
/// Format › Ruler ▸: Ami Pro's Page › Ruler ▸.
const RULER_MENU: &[MenuSlot] = &[Item(C::InsertRuler), Item(C::RemoveRuler)];
/// The Table menu (`UI_SPEC.md` §32, TD1): Ami Pro's Table menu with the
/// run's direct commands (TD2) and additions (TD7). Insert › Table… makes a
/// table.
const TABLE_MENU: &[MenuSlot] = &[
    Item(C::InsertRowAbove),
    Item(C::InsertRowBelow),
    Item(C::InsertColumnLeft),
    Item(C::InsertColumnRight),
    Divider,
    Item(C::DeleteRow),
    Item(C::DeleteColumn),
    Item(C::DeleteTable),
    Divider,
    Folder("Select", TABLE_SELECT_MENU),
    Divider,
    Item(C::ConnectCells),
    Item(C::TableHeadings),
    Folder("Leaders", LEADERS_MENU),
    Item(C::ProtectCells),
    Divider,
    Item(C::DistributeRows),
    Item(C::DistributeColumns),
    Folder("Table Style", TABLE_STYLE_MENU),
    Divider,
    Item(C::EditFormula),
    Folder("Quick Add", QUICK_ADD_MENU),
    Item(C::RecalculateTable),
    Divider,
    Item(C::SortAscending),
    Item(C::SortDescending),
    Divider,
    Item(C::TextToTable),
    Item(C::TableToText),
    Divider,
    Item(C::TableProperties),
];
/// The Draw menu (`UI_SPEC.md` §33, DW4): Ami Pro's Draw menu with the
/// run's additions (DW5, DW6); always present, enabled in Draw mode.
const DRAW_MENU: &[MenuSlot] = &[
    Item(C::DrawProperties),
    Item(C::DrawExtract),
    Item(C::DrawApply),
    Divider,
    Item(C::DrawRotateLeft),
    Item(C::DrawRotateRight),
    Item(C::DrawRotate),
    Folder("Flip", DRAW_FLIP_MENU),
    Divider,
    Item(C::DrawGroup),
    Item(C::DrawUngroup),
    Item(C::DrawFront),
    Item(C::DrawBack),
    Item(C::DrawForward),
    Item(C::DrawBackward),
    Folder("Align", DRAW_ALIGN_MENU),
    Folder("Distribute", DRAW_DISTRIBUTE_MENU),
    Item(C::DrawDuplicate),
    Divider,
    Item(C::DrawSnap),
    Item(C::DrawGrid),
    Item(C::DrawSelectAll),
    Divider,
    Item(C::ImportDrawing),
    Item(C::SaveDrawingAs),
    Divider,
    Item(C::DrawDone),
];
const DRAW_FLIP_MENU: &[MenuSlot] = &[Item(C::DrawFlipHorizontal), Item(C::DrawFlipVertical)];
const DRAW_ALIGN_MENU: &[MenuSlot] = &[
    Item(C::DrawAlignLeft),
    Item(C::DrawAlignCentre),
    Item(C::DrawAlignRight),
    Item(C::DrawAlignTop),
    Item(C::DrawAlignMiddle),
    Item(C::DrawAlignBottom),
];
const DRAW_DISTRIBUTE_MENU: &[MenuSlot] =
    &[Item(C::DrawDistributeAcross), Item(C::DrawDistributeDown)];
/// What the right-click menu holds in Draw mode (`UI_SPEC.md` §33, DW4).
#[cfg(feature = "page-surface")]
pub const DRAW_CONTEXT_MENU: &[MenuSlot] = &[
    Item(C::Cut),
    Item(C::Copy),
    Item(C::Paste),
    Item(C::DrawDuplicate),
    Divider,
    Item(C::DrawProperties),
    Item(C::DrawExtract),
    Item(C::DrawApply),
    Divider,
    Item(C::DrawRotateLeft),
    Item(C::DrawRotateRight),
    Folder("Flip", DRAW_FLIP_MENU),
    Item(C::DrawGroup),
    Item(C::DrawUngroup),
    Item(C::DrawFront),
    Item(C::DrawBack),
    Folder("Align", DRAW_ALIGN_MENU),
    Divider,
    Item(C::DrawDone),
];
const TABLE_SELECT_MENU: &[MenuSlot] = &[
    Item(C::SelectColumn),
    Item(C::SelectRow),
    Item(C::SelectTable),
];
/// Table › Leaders ▸, checked by the caret's cell (Ami Pro's has no check
/// marks; consistent conventions).
const LEADERS_MENU: &[MenuSlot] = &[
    Item(C::LeaderNone),
    Item(C::LeaderDots),
    Item(C::LeaderDashes),
    Item(C::LeaderLine),
];
/// Table › Quick Add ▸ (`UI_SPEC.md` §41).
const QUICK_ADD_MENU: &[MenuSlot] = &[Item(C::QuickAddRow), Item(C::QuickAddColumn)];
const TABLE_STYLE_MENU: &[MenuSlot] = &[
    Item(C::TableStyleGrid),
    Item(C::TableStyleOutline),
    Item(C::TableStyleHeaderRow),
    Item(C::TableStyleBandedRows),
    Item(C::TableStylePlain),
];
/// What the right-click menu adds in a table (`UI_SPEC.md` §32).
#[cfg(feature = "page-surface")]
pub const TABLE_CONTEXT_MENU: &[MenuSlot] = &[
    Item(C::TableProperties),
    Divider,
    Item(C::InsertRowAbove),
    Item(C::InsertRowBelow),
    Item(C::InsertColumnLeft),
    Item(C::InsertColumnRight),
    Item(C::DeleteRow),
    Item(C::DeleteColumn),
    Divider,
    Item(C::ConnectCells),
    Folder("Select", TABLE_SELECT_MENU),
    Divider,
];

/// Format › Alignment ▸, as Ami Pro's Text › Alignment ▸ (the user: "The
/// format menu is getting very tall", 2026-10-17; `UI_SPEC.md` §22).
const ALIGNMENT_MENU: &[MenuSlot] = &[
    Item(C::AlignLeft),
    Item(C::Center),
    Item(C::AlignRight),
    Item(C::Justify),
];
/// Format › Bullets and Numbering ▸ (`UI_SPEC.md` §21, §22), also in the
/// text right-click menu.
const LISTS_MENU: &[MenuSlot] = &[
    Item(C::Bullets),
    Item(C::Numbering),
    Divider,
    Item(C::Promote),
    Item(C::Demote),
    Item(C::MoveUp),
    Item(C::MoveDown),
    Divider,
    Item(C::BulletNumberStyle),
];
/// Format › Frame ▸ (Ami Pro's Frame menu).
const FRAME_MENU: &[MenuSlot] = &[
    Item(C::FrameProperties),
    Item(C::GraphicsScaling),
    Item(C::BringToFront),
    Item(C::SendToBack),
    Item(C::GroupFrames),
    Item(C::UngroupFrames),
];
/// The page's right-click menu on text (`UI_SPEC.md` §16; user,
/// 2026-10-13). Ami Pro 3.1's right button opens Modify Style in the main
/// text (user's guide, chapter 7), so it comes first; the app adds the
/// paragraph's style list after these.
#[cfg(feature = "page-surface")]
pub const TEXT_CONTEXT_MENU: &[MenuSlot] = &[
    Item(C::ModifyCurrentStyle),
    Divider,
    Item(C::Cut),
    Item(C::Copy),
    Item(C::Paste),
    Divider,
    Item(C::Font),
    Item(C::SpecialEffects),
    Divider,
    Folder("Bullets and Numbering", LISTS_MENU),
];
/// What the right-click menu adds on a field (`UI_SPEC.md` §28).
#[cfg(feature = "page-surface")]
pub const FIELD_CONTEXT_MENU: &[MenuSlot] = &[
    Item(C::EditField),
    Item(C::UpdateFields),
    Item(C::LockField),
    Divider,
];
/// What the right-click menu adds first in a page's margin, where Ami Pro
/// 3.1's right button opens Modify Page Layout (chapter 8; `UI_SPEC.md`
/// §16, §18).
#[cfg(feature = "page-surface")]
pub const MARGIN_CONTEXT_MENU: &[MenuSlot] = &[Item(C::PageLayout), Divider];
/// The right-click menu on a selected frame. Ami Pro 3.1's right button
/// opens Modify Frame Layout on a frame (chapter 15), so Frame Properties
/// comes first.
#[cfg(feature = "page-surface")]
pub const FRAME_CONTEXT_MENU: &[MenuSlot] = &[
    Item(C::FrameProperties),
    Item(C::GraphicsScaling),
    Divider,
    Item(C::BringToFront),
    Item(C::SendToBack),
    Divider,
    Item(C::GroupFrames),
    Item(C::UngroupFrames),
    Divider,
    Item(C::Delete),
];
/// Ami Pro's Style menu, in its order.
const STYLE_MENU: &[MenuSlot] = &[
    Item(C::CreateStyle),
    Item(C::ModifyCurrentStyle),
    Item(C::OutlineNumbering),
    Divider,
    Item(C::UseAnotherStyleSheet),
    Item(C::SaveAsStyleSheet),
    Item(C::StyleManagement),
    Divider,
    Item(C::Styles),
];
const CAPS_MENU: &[MenuSlot] = &[
    Item(C::UpperCase),
    Item(C::LowerCase),
    Item(C::InitialCaps),
    Item(C::SmallCaps),
];
const TOOLS_MENU: &[MenuSlot] = &[
    Item(C::Spelling),
    Item(C::SpellAsYouType),
    Item(C::Thesaurus),
    Item(C::GrammarCheck),
    Item(C::ReadabilityStatistics),
    Item(C::Language),
    Item(C::Sort),
    Item(C::ImageProcessing),
    Item(C::Charting),
    Item(C::ChartFromTable),
    Item(C::Equations),
    Item(C::PasteLatexEquation),
    Divider,
    Item(C::WordCount),
    Item(C::FootnoteOptions),
    Item(C::ContentsIndex),
    Divider,
    Item(C::CustomizeToolbar),
    Item(C::Preferences),
];
const WINDOW_MENU: &[MenuSlot] = &[
    Item(C::NextDocument),
    Item(C::PreviousDocument),
    Divider,
    Documents,
];
const HELP_MENU: &[MenuSlot] = &[
    Item(C::Help),
    Item(C::HelpContents),
    Item(C::HelpSearch),
    Divider,
    Item(C::KeyboardShortcuts),
    Item(C::Compatibility),
    Divider,
    Item(C::CheckForUpdates),
    Item(C::About),
];

pub const MENU_LAYOUTS: &[(MenuRoot, &[MenuSlot])] = &[
    (MenuRoot::File, FILE_MENU),
    (MenuRoot::Edit, EDIT_MENU),
    (MenuRoot::View, VIEW_MENU),
    (MenuRoot::Insert, INSERT_MENU),
    (MenuRoot::Format, FORMAT_MENU),
    (MenuRoot::Table, TABLE_MENU),
    (MenuRoot::Draw, DRAW_MENU),
    (MenuRoot::Tools, TOOLS_MENU),
    (MenuRoot::Window, WINDOW_MENU),
    (MenuRoot::Help, HELP_MENU),
];

pub fn metadata(id: CommandId) -> &'static CommandMetadata {
    COMMANDS
        .iter()
        .find(|command| command.id == id)
        .expect("every CommandId must have registry metadata")
}

pub fn command_for_shortcut(
    modifiers: keyboard::Modifiers,
    key: &Key,
    physical_key: Option<&keyboard::key::Physical>,
) -> Option<CommandId> {
    COMMANDS.iter().find_map(|command| {
        command
            .shortcut
            .is_some_and(|shortcut| shortcut.key_bind().matches(modifiers, key, physical_key))
            .then_some(command.id)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn registry_has_unique_commands_and_complete_menu_ownership() {
        let ids: HashSet<_> = COMMANDS.iter().map(|command| command.id).collect();
        assert_eq!(ids.len(), COMMANDS.len());

        let mut menu_counts = HashMap::new();
        for (root, slots) in MENU_LAYOUTS {
            for id in MenuSlot::commands(slots) {
                assert_eq!(metadata(id).menu, *root);
                *menu_counts.entry(id).or_insert(0usize) += 1;
            }
        }
        assert_eq!(menu_counts.len(), COMMANDS.len());
        assert!(menu_counts.values().all(|count| *count == 1));
    }

    #[test]
    fn approved_menu_roots_are_complete_and_ordered() {
        let roots: Vec<_> = MENU_LAYOUTS.iter().map(|(root, _)| *root).collect();
        assert_eq!(roots, MenuRoot::ALL);
        assert_eq!(
            roots.iter().map(|root| root.label()).collect::<Vec<_>>(),
            [
                "File", "Edit", "View", "Insert", "Format", "Table", "Draw", "Tools", "Window",
                "Help"
            ]
        );
    }

    fn command_order(root: MenuRoot) -> Vec<CommandId> {
        let (_, slots) = MENU_LAYOUTS
            .iter()
            .find(|(candidate, _)| *candidate == root)
            .unwrap();
        MenuSlot::commands(slots)
    }

    #[test]
    fn approved_commands_keep_their_documented_order_in_every_root() {
        assert_eq!(
            command_order(MenuRoot::File),
            [
                C::NewDocument,
                C::NewFromStyleSheet,
                C::Open,
                C::ImportFolder,
                C::CloseDocument,
                C::Save,
                C::SaveAs,
                C::RevertToSaved,
                C::RecoverDocuments,
                C::Password,
                C::ExportPdf,
                C::ExportDocx,
                C::ExportOdt,
                C::ExportRtf,
                C::ExportText,
                C::ExportMarkdown,
                C::ExportFolder,
                C::PageSetup,
                C::Print,
                C::PrintCurrentPage,
                C::PrintOptions,
                C::PrintEnvelope,
                C::Merge,
                C::MasterDocument,
                C::DocumentProperties,
                C::Quit,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::Edit),
            [
                C::Undo,
                C::Redo,
                C::Cut,
                C::Copy,
                C::Paste,
                C::PasteSpecial,
                C::PastePlainText,
                C::Delete,
                C::SelectAll,
                C::Find,
                C::FindNext,
                C::FindPrevious,
                C::Replace,
                C::GoToPage,
                C::GoToHeader,
                C::GoToFooter,
                C::GoToRuler,
                C::PreviousPage,
                C::NextPage,
                C::NextFrame,
                C::Bookmarks,
                C::Notes,
                C::Links,
                C::UpdateFields,
                C::UpdateAllFields,
                C::NextField,
                C::PreviousField,
                C::EditField,
                C::LockField,
                C::ProtectedText,
                C::NoHyphenation,
                C::MarkIndexEntry,
                C::MarkTocEntry,
                C::MarkGlossary,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::View),
            [
                C::ShowToolbar,
                C::StylesPanel,
                C::Ruler,
                C::StatusBar,
                C::ShowTabsReturns,
                C::ColumnGuides,
                C::MarginsInColour,
                C::ShowPictures,
                C::TableAddresses,
                C::ShowPowerFields,
                C::FieldShading,
                C::ShowNotes,
                C::ShowBookmarks,
                C::ZoomIn,
                C::ZoomOut,
                C::FullPage,
                C::ActualSize,
                C::Enlarged,
                C::CustomZoom,
                C::FitPageWidth,
                C::ContinuousPages,
                C::TwoPages,
                C::OutlineMode,
                C::OutlineExpand,
                C::OutlineContract,
                C::ShowLevel1,
                C::ShowLevel2,
                C::ShowLevel3,
                C::ShowLevel4,
                C::ShowLevel5,
                C::ShowLevel6,
                C::ShowLevel7,
                C::ShowLevel8,
                C::ShowLevel9,
                C::ShowAllLevels,
                C::UseOutlineStyles,
                C::OutlineNavigator,
                C::DraftMode,
                C::FullScreen,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::Insert),
            [
                C::PageBreak,
                C::ColumnBreak,
                C::Frame,
                C::DrawFrame,
                C::Table,
                C::Image,
                C::InsertFile,
                C::Drawing,
                C::HeaderFooter,
                C::PageNumber,
                C::Footnote,
                C::InsertNote,
                C::CrossReference,
                C::InsertBullet,
                C::DateTime,
                C::Field,
                C::MergeField,
                C::InsertGlossary,
                C::SpecialCharacter,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::Format),
            [
                C::CreateStyle,
                C::ModifyCurrentStyle,
                C::OutlineNumbering,
                C::UseAnotherStyleSheet,
                C::SaveAsStyleSheet,
                C::StyleManagement,
                C::Styles,
                C::Bold,
                C::Italic,
                C::Underline,
                C::WordUnderline,
                C::UpperCase,
                C::LowerCase,
                C::InitialCaps,
                C::SmallCaps,
                C::SpecialEffects,
                C::Font,
                C::ClearDirectFormatting,
                C::FastFormat,
                C::ParagraphProperties,
                C::AlignLeft,
                C::Center,
                C::AlignRight,
                C::Justify,
                C::Bullets,
                C::Numbering,
                C::Promote,
                C::Demote,
                C::MoveUp,
                C::MoveDown,
                C::BulletNumberStyle,
                C::IncreaseIndent,
                C::DecreaseIndent,
                C::FrameProperties,
                C::GraphicsScaling,
                C::BringToFront,
                C::SendToBack,
                C::GroupFrames,
                C::UngroupFrames,
                C::PageLayout,
                C::InsertPageLayout,
                C::RemovePageLayout,
                C::RevertPageLayout,
                C::InsertRuler,
                C::RemoveRuler,
                C::LineNumbering,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::Tools),
            [
                C::Spelling,
                C::SpellAsYouType,
                C::Thesaurus,
                C::GrammarCheck,
                C::ReadabilityStatistics,
                C::Language,
                C::Sort,
                C::ImageProcessing,
                C::Charting,
                C::ChartFromTable,
                C::Equations,
                C::PasteLatexEquation,
                C::WordCount,
                C::FootnoteOptions,
                C::ContentsIndex,
                C::CustomizeToolbar,
                C::Preferences,
            ]
        );
        assert_eq!(
            command_order(MenuRoot::Window),
            [C::NextDocument, C::PreviousDocument,]
        );
        assert_eq!(
            command_order(MenuRoot::Help),
            [
                C::Help,
                C::HelpContents,
                C::HelpSearch,
                C::KeyboardShortcuts,
                C::Compatibility,
                C::CheckForUpdates,
                C::About
            ]
        );
    }

    #[test]
    fn shortcuts_are_unique_and_match_non_latin_physical_fallback() {
        let mut bindings = HashMap::new();
        for command in COMMANDS {
            if let Some(shortcut) = command.shortcut {
                assert_eq!(bindings.insert(shortcut.key_bind(), command.id), None);
            }
        }

        let ctrl_n = metadata(CommandId::NewDocument)
            .shortcut
            .unwrap()
            .key_bind();
        assert_eq!(ctrl_n.to_string(), "Ctrl + N");
        assert!(ctrl_n.matches(
            keyboard::Modifiers::CTRL,
            &Key::Character("т".into()),
            Some(&keyboard::key::Physical::Code(keyboard::key::Code::KeyN)),
        ));
    }

    #[test]
    fn document_switching_uses_working_forward_fallback_and_retains_previous_shortcut() {
        assert_eq!(
            command_for_shortcut(
                keyboard::Modifiers::CTRL,
                &Key::Named(keyboard::key::Named::PageDown),
                None,
            ),
            Some(CommandId::NextDocument)
        );
        assert_eq!(
            command_for_shortcut(
                keyboard::Modifiers::CTRL,
                &Key::Named(keyboard::key::Named::Tab),
                None,
            ),
            None,
            "Ctrl+Tab is not advertised after COSMIC intercepted it physically"
        );
        assert_eq!(
            command_for_shortcut(
                keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
                &Key::Named(keyboard::key::Named::Tab),
                None,
            ),
            Some(CommandId::PreviousDocument)
        );
    }

    #[test]
    fn implemented_commands_never_use_future_routes_or_predicates() {
        for command in COMMANDS {
            if command.implementation == Implementation::Implemented {
                assert_ne!(command.dispatch, DispatchRoute::Future);
                assert_ne!(command.enabled, EnableRule::Future);
                assert_ne!(command.toolbar, ToolbarRoute::Future);
            }
        }
    }
}
