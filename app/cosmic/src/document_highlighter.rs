use std::ops::Range;

use amipro_document::{Document, Paragraph, ParagraphStyle};
use cosmic::iced::advanced::text::{Highlighter, highlighter};
use cosmic::iced::{Font, font};

#[derive(Clone, Debug, PartialEq)]
pub struct DocumentHighlightSettings {
    lines: Vec<Vec<(Range<usize>, DocumentHighlight)>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocumentHighlight {
    font: Font,
}

pub struct DocumentHighlighter {
    settings: DocumentHighlightSettings,
    current_line: usize,
}

impl DocumentHighlightSettings {
    pub fn from_document(document: &Document) -> Self {
        let lines = document
            .paragraphs()
            .iter()
            .map(|paragraph| {
                let style = &document.styles()[&paragraph.style];
                paragraph_highlights(paragraph, style)
            })
            .collect();
        Self { lines }
    }
}

impl Highlighter for DocumentHighlighter {
    type Settings = DocumentHighlightSettings;
    type Highlight = DocumentHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, DocumentHighlight)>;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.settings = new_settings.clone();
        self.current_line = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let highlights = self
            .settings
            .lines
            .get(self.current_line)
            .into_iter()
            .flatten()
            .filter(|(range, _)| range.end <= line.len())
            .cloned()
            .collect::<Vec<_>>();
        self.current_line += 1;
        highlights.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

pub fn document_highlight_format(
    highlight: &DocumentHighlight,
    _theme: &cosmic::Theme,
) -> highlighter::Format<Font> {
    highlighter::Format {
        color: None,
        font: Some(highlight.font),
    }
}

pub fn default_document_font() -> Font {
    Font {
        family: font::Family::Serif,
        ..Font::default()
    }
}

fn paragraph_highlights(
    paragraph: &Paragraph,
    style: &ParagraphStyle,
) -> Vec<(Range<usize>, DocumentHighlight)> {
    if paragraph.text.is_empty() {
        return Vec::new();
    }
    let base = document_font(&style.font_family, style.bold, style.italic);
    let mut highlights = Vec::with_capacity(paragraph.runs.len() * 2 + 1);
    let mut cursor = 0;
    for run in &paragraph.runs {
        if cursor < run.start {
            highlights.push((cursor..run.start, DocumentHighlight { font: base }));
        }
        highlights.push((
            run.start..run.end,
            DocumentHighlight {
                font: document_font(
                    run.format
                        .font_family
                        .as_deref()
                        .unwrap_or(&style.font_family),
                    run.format.bold.unwrap_or(style.bold),
                    run.format.italic.unwrap_or(style.italic),
                ),
            },
        ));
        cursor = run.end;
    }
    if cursor < paragraph.text.len() {
        highlights.push((
            cursor..paragraph.text.len(),
            DocumentHighlight { font: base },
        ));
    }
    highlights
}

pub(crate) fn document_font(family: &str, bold: bool, italic: bool) -> Font {
    let normalized = family.to_ascii_lowercase();
    let family = if normalized.contains("courier") || normalized.contains("mono") {
        font::Family::Monospace
    } else if normalized.contains("arial")
        || normalized.contains("helvetica")
        || normalized.contains("sans")
    {
        font::Family::SansSerif
    } else {
        font::Family::Serif
    };
    Font {
        family,
        weight: if bold {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        },
        style: if italic {
            font::Style::Italic
        } else {
            font::Style::Normal
        },
        ..Font::default()
    }
}

#[cfg(test)]
mod tests {
    use amipro_document::{CharacterFormat, CharacterRun, StyleId};

    use super::*;

    #[test]
    fn document_ranges_combine_paragraph_and_character_formatting() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Heading detail".into();
        document.paragraph_mut(0).style = StyleId(2);
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 8,
            end: 14,
            format: CharacterFormat {
                bold: Some(false),
                italic: Some(true),
                ..CharacterFormat::default()
            },
        });

        let settings = DocumentHighlightSettings::from_document(&document);
        let highlights = &settings.lines[0];

        assert_eq!(highlights.len(), 2);
        assert_eq!(highlights[0].0, 0..8);
        assert_eq!(highlights[0].1.font.weight, font::Weight::Bold);
        assert_eq!(highlights[1].0, 8..14);
        assert_eq!(highlights[1].1.font.weight, font::Weight::Normal);
        assert_eq!(highlights[1].1.font.style, font::Style::Italic);
    }

    #[test]
    fn unicode_highlight_ranges_remain_utf8_byte_ranges() {
        let mut document = Document::default();
        document.paragraph_mut(0).text = "Ami Привет".into();
        let text_length = document.paragraphs()[0].text.len();
        document
            .paragraph_mut(0)
            .apply_character_format(
                4,
                text_length,
                &CharacterFormat {
                    italic: Some(true),
                    ..CharacterFormat::default()
                },
            )
            .unwrap();

        let settings = DocumentHighlightSettings::from_document(&document);

        assert_eq!(settings.lines[0][1].0, 4..text_length);
    }
}
