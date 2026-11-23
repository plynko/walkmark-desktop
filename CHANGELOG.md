# Changelog

## 0.2.3 — 2026-11-23

- Stop writing now interrupts requests that are already running, including the overview pass, and returns unfinished steps to how they were.
- The fixed wording in exports now follows the guide's language: the heading above the title, “Before you start”, “Exported”, “Version” and “Author”, the step list and contents headings, “Step”, the step count and the credit line. Italian, Spanish, French, German, Portuguese and Dutch are built in; for any other language, translate it with AI or type your own in Settings → Writing. HTML exports also declare the right page language.
- A line under the title shows the export date, an editable version number and the author. It can be turned off in the export options.
- Removed unused commands and dependencies.

## 0.2.2 — 2026-11-23

- Highlight an area with a semi-transparent Tint rectangle, or point at it with an Arrow. Both can be recoloured in the editor and are embedded in exported images.
- Projects now record the version of their file format, so future releases can open older guides safely.

## 0.2.1 — 2026-11-23

- Return to the library from an open guide with Home.
- Add an individual screenshot to an existing guide by capturing, importing, or pasting it. Choose another frame explains how to swap in a nearby recording frame.
- Crop screenshots, mark clicks with a cursor, and select, move, or undo image annotations.
- Delete steps without jumping to the top of the guide, and restore the most recent deletion with Undo.
- Polish the existing title and description of a step with AI. Guide summaries and step descriptions grow to fit and support Markdown previews.
- Show tooltips for icon buttons and bundle interface icons for offline use.
- Make the full title bar draggable around its controls and add an About tab in Settings.

## 0.1.1 — 2026-08-09

- Per-document language picker for AI writing, with expanded preset languages
- Library search and pagination on the documents screen
- Document language saved when starting a recording

## 0.1.0 — 2026-08-08

First public open source release.

- Screen and window recording with automatic step detection
- Manual step marks, pause/resume, recording preferences
- Document editor with step rail, blur, redact, highlight
- Contextual AI writing (Gemini, OpenAI, Anthropic, Mistral, OpenRouter, Ollama, compatible servers)
- Export to Markdown, HTML, and PDF
- macOS, Windows, and Linux installers via GitHub Releases
