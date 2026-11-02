# Practical print and digital product workflows

Use Canvas Studio 1.9.0 or later for these features. All design work runs locally;
no account, subscription or paid generation service is required.

## Fonts and paragraphs

Open **Text → Import TrueType Font (.ttf)**. Supported fonts are individual static
TrueType-outline faces permitting editable embedding. Import each actual weight
or italic face separately, then choose it in the font menu. Font bytes travel with
the document; fonts do not need to be installed on the recipient's computer.
Variable fonts, collections and CFF OpenType outlines are not supported.

Select a text frame for point size, leading, tracking and paragraph controls.
Paragraph options include before/after spacing, first-line indentation and frame
insets. Select characters in the canvas text editor to apply character emphasis
or a different face. Preflight identifies overflowing text; inspect long values.

## Reusable collections and personalization

In **Production**, add named fields such as `guest_name` and `event_date`.
Insert `{{guest_name}}` in text or bind a selected frame. Mark required fields;
photo fields can embed local images and bind to multiple image frames.

Save a shared text style from a selected text frame and link other frames to it.
Changing the definition updates those frames in preview/export. Save the complete
collection in **My Templates** to preserve an independent master. Unfilled required
fields are allowed in a master, but must be completed before delivery exports.

For CSV production, headers must exactly match field keys. Use quoted fields for
commas/newlines; photo paths may be relative to the CSV file. The background job
produces PDFs, first-page proofs and a JSON result report. Missing required values
and overflow are reported per record. Editable working copies are optional.

## Books

The **Book** panel provides page thumbnails, rename/reorder/copy/delete and blank
page insertion. Enable the book master for facing-page margins, headers, footers
and folios. Headers/footers accept `{{page}}`, `{{pages}}` and `{{page_title}}`.
Front-matter skipping changes the numbering start; physical blank backs still
count in page positions. Disable Master on a page for a genuinely blank back.

Move selected unlocked layers to the shared master. **Edit Master on This Page**
creates working layers; explicitly save or cancel that edit before organising
pages. Saving the document also commits an active master edit. Master artwork
is inherited behind the ordinary body layers.

Batch artwork import accepts PNG/JPEG/WebP files in the selected order. Contain
preserves the full artwork; Cover fills the content area and may crop it. Optional
blank backs follow each imported page. Refit imported artwork after margin changes.
If the document changes during production, the completed job is preserved separately;
save it as a portable project from Book before starting another production task.

Check final dimensions, image resolution and safety margins at the intended output
settings. The automated 50-page fixture uses modest images; it does not establish
performance for every full-resolution illustration book.

## Word searches

In **Puzzles**, enter one word per line, a grid size, direction difficulty and seed.
Spaces, hyphens and apostrophes are omitted in the grid. Vocabulary uses English
A–Z letters. Review vocabulary and filler for the intended audience; direction
rules are not a reading-age assessment or a general content filter.

The same seed/settings reproduce the grid. Each target must occur exactly once
when scanned in all eight directions. Duplicates, reversed duplicates and invalid
or impossible inputs are rejected. Create up to 50 puzzles in one job, with optional
answer pages collected at the back. Larger grids must remain at least 10 pt at the
chosen page size; titles and word lists must fit rather than silently clipping.

Recipes are saved with the document. **Load controls**, adjust Print styling, then
**Rebuild with current styling** to retain the grid/answers while changing their
appearance. Added custom artwork is preserved; generated layers are replaced.
Hand editing generated layers blocks checked export until rebuilt. **Detach recipe**
retains all editable layers but ends automated answer checking for that puzzle.

## Portable files and delivery

Use **Export → Save Editable Portable Copy (.canvas)** for a single file containing
the design, imported fonts and image assets. Existing JSON-plus-assets projects
remain supported. Print guides and named output presets travel with the project;
guides are editing overlays and never appear in the exported artwork.

A `.canvas` file is editable in the native Canvas Studio application or its local
browser editor. The distributed host binary is Linux x86-64; this does not provide
a public customer/Canva link, PPTX or fillable PDF. See [the browser guide](BROWSER-EDITOR.md).
PDF/PNG/SVG are production outputs, not evidence of unrestricted recipient editing.
Test the actual recipient platform before advertising an editable product.

Before physical production, independently inspect the PDF and run a real printer
proof. Automated/headless checks do not replace native desktop or printer acceptance.
