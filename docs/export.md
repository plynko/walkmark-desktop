# Export

Open **Export** from the document toolbar (**⌘E** / **Ctrl+E**) when the guide reads right. You can also export from the library without opening a guide: hover over a document and select the download button, or choose **Export…** in its **⋯** menu.

## Formats

### Markdown

A `.md` file plus an `images/` folder beside it. Image links are relative paths.

Best for: Git repos, wikis, static site generators, Notion import, docs-as-code workflows.

### Web page (HTML)

One **self-contained** `.html` file with images embedded as base64. No external files, no CDN — works offline and in email attachments.

Best for: sharing with someone who does not have Walkmark, internal wikis that accept HTML upload, quick review in a browser.

### Word (.docx)

An editable **A4** document. Titles are real headings, lists are real lists, bold, italics, code and links keep their formatting, and each screenshot is embedded with its annotations and crop already applied. The footer carries page numbers. It opens in Word, Pages, Google Docs and LibreOffice, and the headings show up in Word's navigation pane.

Best for: handing a guide to someone who will rework the wording, style it with a company template, or sign it off in Word.

### PDF

Print-ready **A4** document with page numbers. Steps avoid awkward page breaks where possible.

Best for: handbooks, compliance packets, printed runbooks, sign-off documents.

## Export options

| Option | Default | Applies to |
| --- | --- | --- |
| **Include images** | On | Markdown (always embedded in HTML/PDF) |
| **Image width** | Standard (1400 px) | Compact 900 / Standard 1400 / Large 1920 |
| **Table of contents** | On | Markdown, HTML and Word (when more than two steps) |
| **Summary** | On | All formats |
| **Prerequisites** | On | All formats |
| **Date, version and author** | On | All formats — a line under the title. The date is the one set in the guide, or the export day if it is empty |
| **Theme** | Auto | HTML only — light, dark, or follow system |

After export, Walkmark can:

- **Reveal in Finder / Explorer** — open the containing folder
- **Copy Markdown to clipboard** — paste into Slack, Notion, etc.

## What gets exported

Only **included** steps (eye icon open) appear. Excluded steps and their frames are omitted.

| Content | Exported as |
| --- | --- |
| Title, summary, prerequisites | Document header |
| Step titles and bodies | Numbered steps |
| Screenshots | With annotations burned in |
| Locked hand-edited text | Exactly as you wrote it |

### Unwritten steps

Steps without body text still export if included — placeholders may appear. The export dialog warns how many steps are incomplete (amber notice).

## Workflow tips

- Run **Write** before export for polished prose, or export skeleton + screenshots for manual editing elsewhere.
- Use **HTML** for one-file sharing; **Markdown** when you need editable source in Git; **Word** when others will edit the text.
- Use **Compact** image width for Slack or narrow wiki columns; **Large** for print-like HTML.
- Redact in the image editor first — exports reflect annotations, not raw frames.

See [Editing guides](editing.md) for exclude, annotations, and locked steps.

## Archives (.zip)

To back up a guide or move it to another computer, choose **Export archive (.zip)** in a document's **⋯** menu in the library. The zip holds the whole guide: its text, steps and screenshots. To restore it, select **Import** in the library and pick the zip. The guide is added as a new document, so importing never overwrites an existing one.
