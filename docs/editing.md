# Editing guides

The editor has a **step rail** on the left and the document on the right. Click a step in the rail to scroll to it; click a step card in the document to select it in the rail.

Click **Home** in the top bar to return to the guide library.

## Document header

At the top of the document:

| Field | Purpose |
| --- | --- |
| **Title** | Guide name (library card and exports) |
| **Summary** | Short overview for readers |
| **Version, Author** | A line under the title, next to the export date. The date is filled in on the day you export; version (default 1.0) and author are yours to edit |
| **Before you start** | Prerequisites — only shown when filled |

The words around your text — the small heading above the title, “Before you start”, “Exported”, “Version”, “Author”, “Steps”, “On this page”, “Step”, the step count and the credit line — are the same for every guide in a language. They follow the guide's language and are set in **Settings → Writing → Wording in exports**.

AI fills these on write; edit them anytime.

## Step text

Each step card has:

- **Title** — short imperative line (e.g. "Open Billing settings")
- **Instructions** — body text for that step
- **Screenshot** — the captured frame with optional annotations

Click any title or instruction field to edit by hand. Changes save automatically.

The guide summary and step descriptions grow as you type. They accept Markdown such as `**bold**`, `*italics*`, lists, links and code. Use **Preview Markdown** to see the result and **Edit Markdown** to continue writing. HTML uses the formatted text, Markdown export keeps the source, and PDF shows the readable text and list markers.

### Add a screenshot to an existing guide

Click **Add step** in the step rail. Choose **Capture screenshot** to pick a screen or window and take one screenshot after a countdown. Use **Import image** for a PNG or JPEG, or **Paste from clipboard** for a copied image.

Walkmark saves a copy in the guide and inserts the step after your selection, or at the end if nothing is selected. Write the instructions by hand or use **Write** to draft them.

### Locked steps

Steps you edit manually are **locked** (lock icon). When you run **Rewrite everything**, locked steps are skipped so your fixes are not overwritten.

Use the **sparkles** button to polish the text on one step. The AI uses your title and instructions as its draft and the screenshot to clarify details. If the step is empty, it writes a first draft from the screenshot. Polishing a locked step keeps it locked.

### Write states

| State | Meaning |
| --- | --- |
| Empty | Not written yet |
| Writing | AI in progress (skeleton loader) |
| Ready | Text present |
| Error | Provider failed — message on card; retry from sparkles or full rewrite |

## Selection and bulk actions

Select steps in the rail or document:

| Input | Action |
| --- | --- |
| Click | Select one |
| **⌘** / **Ctrl + click** | Toggle in selection |
| **Shift + click** | Select range |

When multiple steps are selected, a floating bar offers:

| Action | Effect |
| --- | --- |
| **Merge** | Combine selected steps into one (one screenshot, merged context) |
| **Exclude** | Hide from document and export (frame kept on disk) |
| **Delete** | Remove the selected steps |

Use **Undo** in the confirmation or **Undo delete** in the step rail to restore the most recent deletion. This works after reopening the guide, too.

There is no **split** — if auto-detection merged actions you wanted separate, delete or re-record.

## Reorder

Drag steps in the rail. Step numbers update in the exported guide.

## Exclude steps

Click the **eye** icon to exclude a step from the document and export without deleting the frame. Useful for dead ends, mistakes, or false captures you might want later.

Excluded steps do not count toward **Write N steps** or export.

## Annotations

Open the **image editor** on a step screenshot (pencil icon or click the image):

| Tool | Shortcut | Effect |
| --- | --- | --- |
| **Select** | V | Click an annotation to select it, drag to move it, or press **Delete** to remove it |
| **Blur** | B | Pixelate sensitive details |
| **Cover** | R | Solid fill over a region |
| **Highlight** | H | Coloured box around a control |
| **Tint** | T | Drag a coloured rectangle that leaves the screenshot visible underneath |
| **Arrow** | A | Drag from the tail of the arrow to the point you want to mark |
| **Crop** | C | Drag over the area to keep, then click **Apply crop** |
| **Cursor** | M | Place the tip of an arrow cursor at the clicked point, with a translucent yellow halo |

Use **Crop → Reset crop** to restore the full screenshot. The original is kept.
Use **Undo** to reverse the last annotation change.

The crop and annotations are applied to exported images and to images sent to the model when you run **Write**. Blur or cover sensitive details before writing.

Highlight, Cover, Tint and Arrow support colour options in the toolbar. Highlight also supports outline weight.

## Pick a different moment

If a step fired too early or late, click **Choose another frame** on the screenshot when available. Pick one of the nearby frames from the same recording. The step text stays in place.

Alternates are captured automatically when Walkmark samples near the same timestamp — you do not manage them manually during recording.

## Toolbar actions

| Control | Action |
| --- | --- |
| **Model picker** | Provider + model for this document |
| **Write N steps** | Generate text for steps that are empty |
| **Rewrite** | Regenerate all unlocked steps |
| **Export** | Markdown, HTML, or PDF |

## Product vocabulary

In **Settings → Writing**, under **Products**, define product-specific terms:

- **Term** — word the writer should use (*Workspace*, *Billing portal*)
- **What it is** — short definition

Set a **Default product** for new recordings. The writer injects vocabulary for the product tagged on the guide.

See [Settings → Writing](settings.md).
