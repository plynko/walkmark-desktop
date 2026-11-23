use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};

fn events(input: &str) -> Parser<'_> {
    Parser::new_ext(
        input,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES,
    )
}

/// Guide prose shares a renderer between the webview and HTML export.
/// Keep raw HTML as text and never load images embedded in prose.
pub fn render(input: &str) -> String {
    let mut out = String::new();
    let mut allow_link = false;
    html::push_html(
        &mut out,
        events(input).filter_map(|event| match &event {
            Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text.clone())),
            Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => None,
            Event::Start(Tag::Link { dest_url, .. }) => {
                allow_link = tauri::Url::parse(dest_url)
                    .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.has_host());
                allow_link.then_some(event)
            }
            Event::End(TagEnd::Link) => allow_link.then_some(event),
            Event::SoftBreak => Some(Event::HardBreak),
            _ => Some(event),
        }),
    );
    out
}

/// Extract the readable content for the PDF writer's plain text layout.
pub fn plain_text(input: &str) -> String {
    let mut out = String::new();
    let mut lists = Vec::new();
    for event in events(input) {
        match event {
            Event::Text(text) | Event::Code(text) | Event::Html(text) | Event::InlineHtml(text) => {
                out.push_str(&text);
            }
            Event::Start(Tag::List(start)) => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if let Some(Some(number)) = lists.last_mut() {
                    out.push_str(&format!("{number}. "));
                    *number = number.saturating_add(1);
                } else {
                    out.push_str("• ");
                }
            }
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::End(
                TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::Item
                | TagEnd::CodeBlock
                | TagEnd::TableRow
                | TagEnd::TableHead,
            ) => {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Event::End(TagEnd::TableCell) => out.push_str("  "),
            _ => {}
        }
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_markdown_without_loading_active_content() {
        let html = render(
            "**Save** [Help](https://example.com) [Unsafe](javascript:alert%281%29)\n\n+             <script>alert(1)</script>\n\n![Screenshot](https://example.com/tracker.png)",
        );
        assert!(html.contains("<strong>Save</strong>"));
        assert!(html.contains("<a href=\"https://example.com\">Help</a>"));
        assert!(html.contains("Unsafe"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("Screenshot"));
        assert!(!html.contains("href=\"javascript:"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("<img"));
    }

    #[test]
    fn pdf_text_preserves_labels_code_and_lists() {
        assert_eq!(
            plain_text("## Setup\n\n1. Click **Save**.\n2. Enter `api_key` and read [Help](https://example.com)."),
            "Setup\n1. Click Save.\n2. Enter api_key and read Help."
        );
    }
}
