//! File › Print… through the desktop's print dialog (`docs/Design/UI_SPEC.md`
//! §29, PO1; `docs/research/PRINT_NOTES.md`, "The desktop side"): the XDG
//! print portal's PreparePrint shows the dialog, set to the document's
//! paper; the app sends the pages it chose as a PDF through Print. The
//! dialog's page range is the app's to apply (GTK passes it to no one for a
//! file); copies, collate, reverse, odd and even pages, and pages per sheet
//! reach CUPS as job options.

use std::path::PathBuf;

use cosmic::dialog::ashpd::desktop::print::{Orientation, PageSetup, PrintProxy, Settings};

/// Millimetres in a twip.
const MM_PER_TWIP: f64 = 25.4 / 1_440.0;

/// What the dialog chose.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Prepared {
    /// The pages to send, by position in the document (0-based).
    pub pages: Vec<usize>,
    /// The paper, in twips, as the page lies on it (landscape turned).
    pub paper: Option<(f32, f32)>,
    pub token: u32,
}

/// The dialog's Pages choice, read back: `print-pages` (all, current,
/// ranges) and `page-ranges` ("0-2,4", from 0, as GTK stores them), over
/// `count` pages; `current` is the caret's page (0-based).
pub(crate) fn chosen_pages(
    print_pages: Option<&str>,
    page_ranges: Option<&str>,
    count: usize,
    current: usize,
) -> Vec<usize> {
    match print_pages {
        Some("current") => vec![current.min(count.saturating_sub(1))],
        Some("ranges") => {
            let mut pages = Vec::new();
            for range in page_ranges.unwrap_or_default().split(',') {
                let mut ends = range.trim().splitn(2, '-');
                let Some(Ok(start)) = ends.next().map(|start| start.trim().parse::<usize>()) else {
                    continue;
                };
                // "3-" runs to the end; a single number is one page.
                let end = match ends.next().map(str::trim) {
                    Some("") => count.saturating_sub(1),
                    Some(end) => end.parse::<usize>().unwrap_or(start),
                    None => start,
                };
                for page in start..=end.min(count.saturating_sub(1)) {
                    if !pages.contains(&page) {
                        pages.push(page);
                    }
                }
            }
            pages
        }
        _ => (0..count).collect(),
    }
}

/// The dialog's page set-up for a page `width` × `height` twips: a
/// standard paper by its PWG name where one is within 1/8 in (as Ami Pro
/// matches paper, `1238:280a`), else a custom size; turned for landscape.
pub(crate) fn page_setup(width: f32, height: f32) -> PageSetup {
    let landscape = width > height;
    let (short, long) = if landscape {
        (height, width)
    } else {
        (width, height)
    };
    const PAPERS: [(&str, &str, f32, f32); 5] = [
        ("na_letter", "US Letter", 12_240.0, 15_840.0),
        ("na_legal", "US Legal", 12_240.0, 20_160.0),
        ("iso_a4", "A4", 11_906.0, 16_838.0),
        ("iso_a5", "A5", 8_391.0, 11_906.0),
        ("na_executive", "Executive", 10_440.0, 15_120.0),
    ];
    let standard = PAPERS.iter().find(|(_, _, paper_short, paper_long)| {
        (paper_short - short).abs() <= 180.0 && (paper_long - long).abs() <= 180.0
    });
    let (name, display, short, long) = match standard {
        Some((name, display, paper_short, paper_long)) => (
            (*name).to_owned(),
            (*display).to_owned(),
            *paper_short,
            *paper_long,
        ),
        None => {
            let (width_mm, height_mm) = (
                (f64::from(short) * MM_PER_TWIP).round(),
                (f64::from(long) * MM_PER_TWIP).round(),
            );
            (
                format!("custom_{width_mm}x{height_mm}mm"),
                format!("{width_mm} × {height_mm} mm"),
                short,
                long,
            )
        }
    };
    PageSetup::default()
        .name(name.as_str())
        .display_name(display.as_str())
        .width(f64::from(short) * MM_PER_TWIP)
        .height(f64::from(long) * MM_PER_TWIP)
        .orientation(if landscape {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        })
}

/// The paper the dialog chose, in twips, as the page lies on it.
fn paper_of(setup: &PageSetup) -> Option<(f32, f32)> {
    let (width, height) = (setup.width?, setup.height?);
    let (width, height) = ((width / MM_PER_TWIP) as f32, (height / MM_PER_TWIP) as f32);
    Some(match setup.orientation {
        Some(Orientation::Landscape | Orientation::ReverseLandscape) => (height, width),
        _ => (width, height),
    })
}

/// Shows the desktop's print dialog for `title`, its paper set for the
/// first page (`width` × `height` twips) and, for Print Current Page…, the
/// range set to the caret's page.
pub(crate) async fn prepare(
    title: String,
    (width, height): (f32, f32),
    count: usize,
    current: usize,
    only_current: bool,
) -> Result<Option<Prepared>, String> {
    let proxy = PrintProxy::new()
        .await
        .map_err(|error| format!("the desktop's print service: {error}"))?;
    let range = current.to_string();
    let settings = if only_current {
        Settings::default()
            .print_pages("ranges")
            .page_ranges(format!("{range}-{range}").as_str())
    } else {
        Settings::default()
    };
    let request = proxy
        .prepare_print(
            None,
            &title,
            settings,
            page_setup(width, height),
            "Print",
            true,
        )
        .await
        .map_err(|error| error.to_string())?;
    let response = match request.response() {
        Ok(response) => response,
        // Cancel in the dialog.
        Err(cosmic::dialog::ashpd::Error::Response(_)) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    Ok(Some(Prepared {
        pages: chosen_pages(
            response.settings.print_pages.as_deref(),
            response.settings.page_ranges.as_deref(),
            count,
            current,
        ),
        paper: paper_of(&response.page_setup),
        token: response.token,
    }))
}

/// Sends `pdf` to the printer the dialog chose (its `token`), through a
/// file the portal reads before this returns; the file is removed after.
pub(crate) async fn send(title: String, pdf: Vec<u8>, token: u32) -> Result<(), String> {
    let path: PathBuf =
        std::env::temp_dir().join(format!("amipro-print-{}-{}.pdf", std::process::id(), token));
    std::fs::write(&path, &pdf).map_err(|error| error.to_string())?;
    let result = async {
        let file = std::fs::File::open(&path).map_err(|error| error.to_string())?;
        let proxy = PrintProxy::new()
            .await
            .map_err(|error| format!("the desktop's print service: {error}"))?;
        let request = proxy
            .print(None, &title, &file, Some(token), true)
            .await
            .map_err(|error| error.to_string())?;
        request.response().map_err(|error| error.to_string())
    }
    .await;
    let _ = std::fs::remove_file(&path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dialogs_pages_are_read_from_its_ranges() {
        // GTK stores the user's "1-3, 5" as "0-2,4".
        assert_eq!(
            chosen_pages(Some("ranges"), Some("0-2,4"), 10, 0),
            [0, 1, 2, 4]
        );
        assert_eq!(chosen_pages(Some("ranges"), Some("8-"), 10, 0), [8, 9]);
        assert_eq!(chosen_pages(Some("ranges"), Some("6-20"), 8, 0), [6, 7]);
        assert_eq!(chosen_pages(Some("ranges"), Some("2,2,1"), 5, 0), [2, 1]);
        assert_eq!(chosen_pages(Some("current"), None, 5, 3), [3]);
        assert_eq!(chosen_pages(Some("all"), None, 3, 1), [0, 1, 2]);
        assert_eq!(chosen_pages(None, None, 2, 0), [0, 1]);
    }

    #[test]
    fn the_paper_is_named_where_it_is_standard_and_turned_for_landscape() {
        let letter = page_setup(12_240.0, 15_840.0);
        assert_eq!(letter.name.as_deref(), Some("na_letter"));
        assert_eq!(letter.orientation, Some(Orientation::Portrait));
        assert!((letter.width.unwrap() - 215.9).abs() < 0.01);
        let landscape = page_setup(16_838.0, 11_906.0);
        assert_eq!(landscape.name.as_deref(), Some("iso_a4"));
        assert_eq!(landscape.orientation, Some(Orientation::Landscape));
        assert_eq!(
            paper_of(&landscape).map(|(w, h)| (w.round(), h.round())),
            Some((16_838.0, 11_906.0))
        );
        let custom = page_setup(8_640.0, 12_960.0);
        assert_eq!(custom.name.as_deref(), Some("custom_152x229mm"));
    }
}
