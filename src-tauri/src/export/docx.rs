//! Word (`.docx`) writer.
//!
//! Text stays editable: titles are real headings, lists are real lists and the
//! guide's Markdown becomes ordinary runs. Screenshots are the same flattened
//! PNGs every other format embeds, so annotations and crops come out identical.

use std::io::Cursor;

use docx_rs::*;
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use crate::error::{AppError, Result};
use crate::guide_markdown::events;
use crate::models::Project;

use super::{ExportOptions, RenderedStep};

// A4 in twentieths of a point, with 2 cm margins.
const PAGE_W: u32 = 11_906;
const PAGE_H: u32 = 16_838;
const MARGIN: i32 = 1_134;
/// 1 twip is 635 EMU; the usable width is what is left between the margins.
const CONTENT_EMU: u32 = (PAGE_W - 2 * MARGIN as u32) * 635;
/// One CSS pixel in EMU, the unit screenshots are measured in.
const PX_EMU: u32 = 9_525;

const ACCENT: &str = "3B5BDB";
const MUTED: &str = "6B7280";
const BULLETS: usize = 1;

const BODY: usize = 22; // half-points
const MONO: &str = "Consolas";

/// One piece of text with its formatting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    /// Only `http` and `https` links survive, as in the HTML export.
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Marker {
    Bullet,
    Number(u64),
}

/// A paragraph of the guide's prose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Block {
    pub spans: Vec<Span>,
    /// 1-based nesting depth for list items, 0 for plain paragraphs.
    pub depth: usize,
    pub marker: Option<Marker>,
    pub code_block: bool,
}

impl Block {
    fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| span.text.is_empty())
    }
}

/// Turns the guide's Markdown into paragraphs, lists and formatted runs.
pub(crate) fn blocks(markdown: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut current: Option<Block> = None;
    let (mut bold, mut italic, mut strike) = (0usize, 0usize, 0usize);
    let mut link: Option<String> = None;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut in_code_block = false;

    fn flush(out: &mut Vec<Block>, current: &mut Option<Block>) {
        if let Some(block) = current.take() {
            if !block.is_empty() {
                out.push(block);
            }
        }
    }

    for event in events(markdown) {
        match event {
            Event::Start(Tag::Paragraph) => {
                if current.is_none() {
                    current = Some(Block {
                        depth: lists.len(),
                        ..Default::default()
                    });
                } else if let Some(block) = current.as_mut() {
                    // A second paragraph in a list item stays in the item.
                    block.spans.push(Span {
                        text: "\n".into(),
                        ..Default::default()
                    });
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if lists.is_empty() {
                    flush(&mut out, &mut current);
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                flush(&mut out, &mut current);
                current = Some(Block::default());
                bold += 1;
            }
            Event::End(TagEnd::Heading(_)) => {
                bold = bold.saturating_sub(1);
                flush(&mut out, &mut current);
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut out, &mut current);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                flush(&mut out, &mut current);
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                flush(&mut out, &mut current);
                let marker = match lists.last_mut() {
                    Some(Some(number)) => {
                        let marker = Marker::Number(*number);
                        *number = number.saturating_add(1);
                        marker
                    }
                    _ => Marker::Bullet,
                };
                current = Some(Block {
                    depth: lists.len(),
                    marker: Some(marker),
                    ..Default::default()
                });
            }
            Event::End(TagEnd::Item) => flush(&mut out, &mut current),
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut out, &mut current);
                in_code_block = true;
                let _ = matches!(kind, CodeBlockKind::Fenced(_));
                current = Some(Block {
                    code_block: true,
                    depth: lists.len(),
                    ..Default::default()
                });
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some(block) = current.as_mut() {
                    // Drop the newline pulldown-cmark leaves after the last line.
                    if let Some(last) = block.spans.last_mut() {
                        last.text = last.text.trim_end_matches('\n').to_owned();
                    }
                }
                flush(&mut out, &mut current);
            }
            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => strike += 1,
            Event::End(TagEnd::Strikethrough) => strike = strike.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => {
                link = tauri::Url::parse(&dest_url)
                    .ok()
                    .filter(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
                    .map(|_| dest_url.to_string());
            }
            Event::End(TagEnd::Link) => link = None,
            // Images in prose are never loaded, as in the other formats.
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => {}
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                let block = current.get_or_insert_with(|| Block {
                    depth: lists.len(),
                    ..Default::default()
                });
                block.spans.push(Span {
                    text: text.to_string(),
                    bold: bold > 0,
                    italic: italic > 0,
                    strike: strike > 0,
                    code: in_code_block,
                    link: link.clone(),
                });
            }
            Event::Code(text) => {
                let block = current.get_or_insert_with(|| Block {
                    depth: lists.len(),
                    ..Default::default()
                });
                block.spans.push(Span {
                    text: text.to_string(),
                    bold: bold > 0,
                    italic: italic > 0,
                    strike: strike > 0,
                    code: true,
                    link: link.clone(),
                });
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(block) = current.as_mut() {
                    block.spans.push(Span {
                        text: "\n".into(),
                        ..Default::default()
                    });
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(block) = current.as_mut() {
                    block.spans.push(Span {
                        text: "  ".into(),
                        ..Default::default()
                    });
                }
            }
            Event::End(TagEnd::TableRow | TagEnd::TableHead) => flush(&mut out, &mut current),
            _ => {}
        }
    }
    flush(&mut out, &mut current);
    out
}

fn run_for(span: &Span, size: usize, color: Option<&str>) -> Run {
    let mut run = Run::new().size(size);
    // A newline in a run is a line break in Word, not a space.
    for (index, line) in span.text.split('\n').enumerate() {
        if index > 0 {
            run = run.add_break(BreakType::TextWrapping);
        }
        run = run.add_text(line);
    }
    if span.bold {
        run = run.bold();
    }
    if span.italic {
        run = run.italic();
    }
    if span.strike {
        run = run.strike();
    }
    if span.code {
        run = run.fonts(RunFonts::new().ascii(MONO).hi_ansi(MONO).cs(MONO));
    }
    if span.link.is_some() {
        run = run.color("0563C1").underline("single");
    } else if let Some(color) = color {
        run = run.color(color);
    }
    run
}

/// One of the guide's paragraphs, as a Word paragraph.
fn paragraph_for(block: &Block, size: usize, color: Option<&str>) -> Paragraph {
    let mut paragraph = Paragraph::new().line_spacing(LineSpacing::new().after(120));

    if block.depth > 0 {
        let left = 360 * (block.depth as i32 + 1);
        match &block.marker {
            Some(Marker::Bullet) => {
                paragraph = paragraph.numbering(
                    NumberingId::new(BULLETS),
                    IndentLevel::new((block.depth - 1).min(2)),
                );
            }
            Some(Marker::Number(number)) => {
                // Written out rather than auto-numbered: every list restarts
                // at its own first number without any numbering bookkeeping.
                paragraph = paragraph
                    .indent(
                        Some(left),
                        Some(SpecialIndentType::Hanging(360)),
                        None,
                        None,
                    )
                    // The text starts at the indent, even in viewers that
                    // ignore a hanging indent as a tab stop.
                    .add_tab(Tab::new().val(TabValueType::Left).pos(left as usize))
                    .add_run(
                        Run::new()
                            .size(size)
                            .add_text(format!("{number}."))
                            .add_tab(),
                    );
            }
            None => {
                paragraph = paragraph.indent(Some(left), None, None, None);
            }
        }
    }
    if block.code_block {
        paragraph = paragraph.indent(Some(360), None, None, None);
    }

    for span in &block.spans {
        let run = run_for(span, size, color);
        paragraph = match &span.link {
            Some(url) => paragraph
                .add_hyperlink(Hyperlink::new(url.clone(), HyperlinkType::External).add_run(run)),
            None => paragraph.add_run(run),
        };
    }
    paragraph
}

fn heading(style: &str, text: &str) -> Paragraph {
    Paragraph::new()
        .style(style)
        .keep_next(true)
        .add_run(Run::new().add_text(text))
}

fn styles() -> [Style; 3] {
    let heading = |id: &str, name: &str, size: usize, level: usize| {
        Style::new(id, StyleType::Paragraph)
            .name(name)
            .based_on("Normal")
            .next("Normal")
            .size(size)
            .bold()
            .outline_lvl(level)
            .line_spacing(LineSpacing::new().before(300).after(120))
    };
    [
        Style::new("Title", StyleType::Paragraph)
            .name("Title")
            .based_on("Normal")
            .next("Normal")
            .size(56)
            .bold()
            .line_spacing(LineSpacing::new().after(120)),
        heading("Heading1", "heading 1", 32, 0),
        heading("Heading2", "heading 2", 26, 1),
    ]
}

fn bullets() -> AbstractNumbering {
    let mut numbering = AbstractNumbering::new(BULLETS);
    for (level, symbol) in ["•", "–", "•"].into_iter().enumerate() {
        numbering = numbering.add_level(
            Level::new(
                level,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new(symbol),
                LevelJc::new("left"),
            )
            .indent(
                Some(720 + 360 * level as i32),
                Some(SpecialIndentType::Hanging(360)),
                None,
                None,
            ),
        );
    }
    numbering
}

/// A screenshot, scaled down to the page width when it is wider than that.
fn picture(image: &super::RenderedImage) -> Paragraph {
    let natural_w = image.width.saturating_mul(PX_EMU);
    let natural_h = image.height.saturating_mul(PX_EMU);
    let (w, h) = if natural_w > CONTENT_EMU {
        let scale = CONTENT_EMU as f64 / natural_w as f64;
        (CONTENT_EMU, (natural_h as f64 * scale).round() as u32)
    } else {
        (natural_w, natural_h)
    };
    Paragraph::new()
        .line_spacing(LineSpacing::new().before(120).after(240))
        .add_run(Run::new().add_image(
            Pic::new_with_dimensions(image.png.clone(), image.width, image.height).size(w, h),
        ))
}

pub fn render(
    project: &Project,
    steps: &[RenderedStep],
    options: &ExportOptions,
) -> Result<Vec<u8>> {
    let labels = &options.labels;
    let mut doc = Docx::new()
        .page_size(PAGE_W, PAGE_H)
        .page_margin(
            PageMargin::new()
                .top(MARGIN)
                .bottom(MARGIN)
                .left(MARGIN)
                .right(MARGIN)
                .header(708)
                .footer(708),
        )
        .default_size(BODY)
        .default_fonts(
            RunFonts::new()
                .ascii("Calibri")
                .hi_ansi("Calibri")
                .east_asia("Calibri")
                .cs("Calibri"),
        )
        .add_abstract_numbering(bullets())
        .add_numbering(Numbering::new(BULLETS, BULLETS));
    for style in styles() {
        doc = doc.add_style(style);
    }

    // --- Title block -------------------------------------------------------
    doc = doc.add_paragraph(
        Paragraph::new()
            .line_spacing(LineSpacing::new().after(60))
            .add_run(
                Run::new()
                    .size(18)
                    .bold()
                    .color(ACCENT)
                    .add_text(labels.guide().to_uppercase()),
            ),
    );
    doc = doc.add_paragraph(heading("Title", project.title.trim()));

    let details = super::detail_parts(project, options);
    if !details.is_empty() {
        doc = doc.add_paragraph(
            Paragraph::new()
                .line_spacing(LineSpacing::new().after(160))
                .add_run(
                    Run::new()
                        .size(20)
                        .color(MUTED)
                        .add_text(details.join(super::DETAIL_SEPARATOR)),
                ),
        );
    }

    if options.include_summary && !project.summary.trim().is_empty() {
        for block in blocks(&project.summary) {
            doc = doc.add_paragraph(paragraph_for(&block, 24, Some("374151")));
        }
    }

    if options.include_prerequisites && !project.prerequisites.is_empty() {
        doc = doc.add_paragraph(heading("Heading1", labels.prerequisites()));
        for item in &project.prerequisites {
            let block = Block {
                spans: vec![Span {
                    text: item.trim().to_owned(),
                    ..Default::default()
                }],
                depth: 1,
                marker: Some(Marker::Bullet),
                code_block: false,
            };
            doc = doc.add_paragraph(paragraph_for(&block, BODY, None));
        }
    }

    if options.include_toc && steps.len() > 2 {
        doc = doc.add_paragraph(heading("Heading1", labels.steps()));
        for step in steps {
            let block = Block {
                spans: vec![Span {
                    text: step.title.clone(),
                    ..Default::default()
                }],
                depth: 1,
                marker: Some(Marker::Number(step.number as u64)),
                code_block: false,
            };
            doc = doc.add_paragraph(paragraph_for(&block, BODY, None));
        }
    }

    // --- Steps -------------------------------------------------------------
    for step in steps {
        doc = doc.add_paragraph(heading(
            "Heading2",
            &format!("{}. {}", step.number, step.title),
        ));
        for block in blocks(&step.body) {
            doc = doc.add_paragraph(paragraph_for(&block, BODY, None));
        }
        if let Some(image) = &step.image {
            doc = doc.add_paragraph(picture(image));
        }
    }

    // --- Footer ------------------------------------------------------------
    doc = doc.footer(
        Footer::new().add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(
                    Run::new()
                        .size(17)
                        .color(MUTED)
                        .add_text(format!("{} · ", labels.generated_with())),
                )
                .add_page_num(PageNum::new()),
        ),
    );

    let mut buffer = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut buffer)
        .map_err(|e| AppError::Other(format!("The Word document could not be written: {e}")))?;
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    use crate::export::RenderedImage;
    use crate::models::DocumentLabels;

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn inline_formatting_becomes_runs() {
        let found = blocks("Click **Save** and *wait*, then `exit` ~~now~~.");
        assert_eq!(found.len(), 1);
        let spans = &found[0].spans;
        assert_eq!(text(spans), "Click Save and wait, then exit now.");
        assert!(spans
            .iter()
            .any(|s| s.text == "Save" && s.bold && !s.italic));
        assert!(spans.iter().any(|s| s.text == "wait" && s.italic));
        assert!(spans.iter().any(|s| s.text == "exit" && s.code));
        assert!(spans.iter().any(|s| s.text == "now" && s.strike));
    }

    #[test]
    fn lists_keep_their_markers_and_nesting() {
        let found = blocks("Intro\n\n- one\n- two\n  - inner\n\n3. three\n4. four");
        let shape: Vec<(String, usize, Option<Marker>)> = found
            .iter()
            .map(|b| (text(&b.spans), b.depth, b.marker.clone()))
            .collect();
        assert_eq!(
            shape,
            [
                ("Intro".to_owned(), 0, None),
                ("one".to_owned(), 1, Some(Marker::Bullet)),
                ("two".to_owned(), 1, Some(Marker::Bullet)),
                ("inner".to_owned(), 2, Some(Marker::Bullet)),
                ("three".to_owned(), 1, Some(Marker::Number(3))),
                ("four".to_owned(), 1, Some(Marker::Number(4))),
            ]
        );
    }

    #[test]
    fn only_web_links_survive_and_images_are_dropped() {
        let found = blocks(
            "[Help](https://example.com) [Bad](javascript:alert%281%29) ![x](https://e.com/a.png)",
        );
        let spans = &found[0].spans;
        assert_eq!(
            spans
                .iter()
                .find(|s| s.text == "Help")
                .unwrap()
                .link
                .as_deref(),
            Some("https://example.com")
        );
        assert!(spans
            .iter()
            .find(|s| s.text == "Bad")
            .unwrap()
            .link
            .is_none());
        assert!(!text(spans).contains("a.png"));
    }

    #[test]
    fn code_blocks_are_kept_as_one_monospace_paragraph() {
        let found = blocks("```\nlet a = 1;\nlet b = 2;\n```");
        assert_eq!(found.len(), 1);
        assert!(found[0].code_block);
        assert_eq!(text(&found[0].spans), "let a = 1;\nlet b = 2;");
    }

    fn png(width: u32, height: u32, shade: u8) -> RenderedImage {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([shade, 40, 40]));
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        RenderedImage {
            png: png.into_inner(),
            width,
            height,
        }
    }

    fn entries(bytes: &[u8]) -> (Vec<String>, String) {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip archive");
        let names = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .expect("document.xml")
            .read_to_string(&mut xml)
            .unwrap();
        (names, xml)
    }

    fn sample_steps() -> Vec<RenderedStep> {
        (1..=3)
            .map(|number| RenderedStep {
                number,
                title: format!("Apri il menu {number}"),
                body: "Fai clic su **File**.\n\n- prima\n- poi".into(),
                // Different pictures, as in a real guide: identical ones are stored once.
                image: Some(png(1600, 900, 40 * number as u8)),
            })
            .collect()
    }

    #[test]
    fn writes_a_word_file_with_text_images_and_translated_wording() {
        let mut project = Project::new("Guida all'installazione", "");
        project.summary = "Come installare **Walkmark**.".into();
        project.prerequisites = vec!["Un account".into()];
        project.author = "Erik".into();
        let options = ExportOptions {
            labels: DocumentLabels::built_in("Italian").unwrap(),
            exported_on: "2026-11-23".into(),
            ..Default::default()
        };

        let bytes = render(&project, &sample_steps(), &options).unwrap();
        assert!(bytes.starts_with(b"PK"), "a .docx is a zip archive");
        let (names, xml) = entries(&bytes);

        assert!(names.iter().any(|n| n == "[Content_Types].xml"));
        assert_eq!(
            names
                .iter()
                .filter(|n| n.starts_with("word/media/") && !n.ends_with('/'))
                .count(),
            3,
            "one picture per step: {names:?}"
        );
        for expected in [
            "GUIDA PASSO PASSO",
            "Guida all&apos;installazione",
            "2026-11-23 · Versione 1.0 · Autore Erik",
            "Prima di iniziare",
            "Un account",
            "Passaggi",
            "1. Apri il menu 1",
            "3. Apri il menu 3",
            "File",
        ] {
            assert!(
                xml.contains(expected),
                "missing {expected:?} in the document"
            );
        }
        for english in ["Step-by-step guide", "Before you start"] {
            assert!(!xml.contains(english), "{english} should be translated");
        }
    }

    #[test]
    fn wide_screenshots_are_scaled_to_the_page() {
        let bytes = render(
            &Project::new("Guide", ""),
            &sample_steps()[..1],
            &ExportOptions::default(),
        )
        .unwrap();
        let (_, xml) = entries(&bytes);
        // 1600 px is far wider than the page, so it is cut to the content width.
        assert!(xml.contains(&format!("cx=\"{CONTENT_EMU}\"")), "{xml}");
        assert!(!xml.contains(&format!("cx=\"{}\"", 1600 * PX_EMU)));
    }

    #[test]
    fn options_leave_out_what_was_turned_off() {
        let mut project = Project::new("Guide", "");
        project.summary = "A summary.".into();
        project.prerequisites = vec!["An account".into()];
        let options = ExportOptions {
            include_summary: false,
            include_prerequisites: false,
            include_toc: false,
            include_details: false,
            include_images: false,
            ..Default::default()
        };
        let steps: Vec<RenderedStep> = sample_steps()
            .into_iter()
            .map(|mut step| {
                step.image = None;
                step
            })
            .collect();
        let (names, xml) = entries(&render(&project, &steps, &options).unwrap());
        assert!(!names
            .iter()
            .any(|n| n.starts_with("word/media/") && !n.ends_with('/')));
        for left_out in [
            "A summary.",
            "An account",
            "Before you start",
            "Steps",
            "Exported",
        ] {
            assert!(!xml.contains(left_out), "{left_out} should be left out");
        }
        assert!(xml.contains("Apri il menu 1"));
    }

    #[test]
    fn a_broken_image_does_not_stop_the_export() {
        let steps = vec![RenderedStep {
            number: 1,
            title: "One".into(),
            body: String::new(),
            image: Some(RenderedImage {
                png: vec![1, 2, 3],
                width: 10,
                height: 10,
            }),
        }];
        assert!(render(
            &Project::new("Guide", ""),
            &steps,
            &ExportOptions::default()
        )
        .is_ok());
    }
}
