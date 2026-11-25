# Changelog

## 0.2.7 — 2026-11-25

- Every click now gets its own screenshot, taken after **Pause after each action**. Quick successive clicks, such as opening a menu and choosing an item, are no longer merged into one step. A double-click gives two steps.
- Typing now gives one screenshot, taken **Pause after typing** (new setting, default 1 s) after the last key press, instead of one per burst of keys. A scroll gives one screenshot once the wheel stops.
- The minimum gap between steps now applies only to visual change detection.
- The date under the title is now an editable field. Left empty, exports still use the day they are made.
- The Version and Author fields in the guide are wider.

## 0.2.6 — 2026-11-25

- Every click now gets its own screenshot, taken 20 ms after the click, and is no longer limited by the minimum gap between steps.

## 0.2.5 — 2026-11-25

- Export a document as a portable `.zip` archive (text, steps and screenshots) from the library menu, and import it back with **Import**. An import always creates a new document.
- Windows: fixed the app freezing when a recording starts, and recordings that could not be stopped. Starting and stopping no longer run on the main thread.

## 0.2.4 — 2026-11-23

- Export a document straight from the library: a download button on each card, and **Export…** in its menu. The document is opened behind the scenes and closed again with the dialog.
- Export as Word (.docx): real headings and lists, formatted text, screenshots with annotations and crop applied, translated fixed wording, details line and page numbers.
- Fixed step thumbnails in the left column disappearing after the guide was edited (renaming, changing the version, AI updates). A picture that cannot be loaded now shows a placeholder instead of an empty box.
- Changing the default engine or model in Settings now also changes what Write uses; before, Write kept calling the previous provider until the write menu was changed by hand.
- Rust code formatted with `cargo fmt`.
- HTTPS now uses the operating system's TLS instead of a bundled crypto library. The Rust code compiles for Windows (x64 and ARM64) from macOS, and Linux builds need `libssl-dev`.
- An optional test (`RUN_LIVE_TESTS=1`) checks HTTPS against the real provider APIs.

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
