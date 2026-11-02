# Changes

## 1.12.0 — linked campaign production

- Added portable campaigns with separately composed Instagram post/story, YouTube thumbnail and A4 layouts, shared copy, named colour links and a project library of reusable editable groups.
- Added group-aware browser multi-selection, movement, live native-SVG feedback, proportional scaling, alignment/distribution, grouping, duplication and keyboard nudges. Desktop campaign/brand controls and MCP use the same command engine.
- Components retain shared fields/colours while owning their geometry and captured typography. Scaling materializes linked text styles locally so type scales correctly.
- Added non-mutating named-layout previews/exports and bounded campaign ZIP delivery with editable master, PDFs, PNGs and manifest.
- New-feature projects use a guarded version-2 portable envelope, including all inactive layout/component assets; older builds reject them. Existing static projects remain compatible.

## 1.11.0 — local coding-agent control

- Added a standard-library Python STDIO MCP adapter with 11 tools for live browser inspection, templates, blank composition, atomic/dry-run batches, image previews, preflight, workspace exports, portable opening, font/photo import and undo/redo.
- Guarded document handles/revisions prevent stale edits and exports. Local connection credentials remain private; scoped file access rejects traversal/outside symlinks and accidental overwrite.
- Browser shape type/fill/gradient/outline/corners are editable directly. Browser artwork follows external agent changes while preserving unapplied form/title drafts. Static social PNG output supports exact 96-DPI layout dimensions.
- Added a real STDIO integration workflow and CI coverage, plus a sourced studio review and staged plan for campaign layout, media timelines, HyperFrames and optional Three.js. These motion/3D features are not yet implemented.

## 1.10.0 — a visual design studio

- Replaced the starter catalog with 18 editable designs across Instagram, YouTube, print, presentation, identity and data formats (27 pages). Existing project and legacy template identifiers remain supported.
- Added native artwork thumbnails, catalog search/categories, full browser previews and inserting template pages into an existing design.
- Refreshed both interfaces with a quieter charcoal/teal palette, consistent vector tool icons, clearer hierarchy and responsive browser panels.
- New text starts with neutral paragraph styling instead of inheriting a poster headline’s tight tracking.
- 74 Rust checks and both isolated browser/native-window workflows pass, including multi-page carousel insertion and narrow browser layouts.

## 1.9.0 — local browser editing

- Added `--serve-browser` and `run-browser.sh`: a bundled browser interface backed by the native document/font/export engine, with no account, subscription or JavaScript runtime requirement.
- Open/save portable projects, personalise shared fields/photos, edit text/paragraphs/character emphasis, import fonts, add/replace photos, move/resize/rotate layers, manage pages and undo/redo. Shared-style definitions, book-master editing, puzzles and CSV remain desktop tools; their data is preserved.
- Added native SVG previews, print checks, PDF output and 150-DPI PNG proofs. Production downloads reject required-field failures, unsupported content, modified puzzle keys and text overflow.
- Added private per-port recovery, revision conflict handling, draft-preserving form refresh and protection against downloading unapplied edits.
- The loopback-only service requires a random session token and valid host/origin, gates uploads before reading bodies, and bounds uploads/workers. It is not a public multi-user hosting service.
- 73 Rust checks and isolated Chrome/Chromium/native-window workflows verify editable delivery, Unicode/imported fonts, photos, shared fields, typography, undo/redo, recovery, access checks and narrow layouts. GitHub CI now runs both UI workflows.

## 1.8.1 — native desktop usability fixes

- Scroll the tool rail so Book/Puzzles remain reachable at 680 px window height. Split the header into two rows so Save/Export stay accessible at 1024 px width.
- Keep page navigation, status and zoom controls bounded for long books; page duplication now uses the shared fresh-group identity path.
- Fit new/opened pages and retain Fit mode across resizing, while preserving deliberate manual zoom.
- Fix fast save/undo/redo shortcuts when modifier release arrives in the same frame. Add `--open PATH` for ordinary desktop launches.
- Keep generated headers/folios inside the configured trim safety margin.
- 66 automated checks and native X11 window interactions cover minimum size, resizing, title editing/save/reopen, undo/redo, tool scrolling, book navigation and export/preflight display.

## 1.8.0 — reproducible word-search products

- Added a Puzzles panel with vocabulary, 8–24 grids, three direction rules, seeds, print typography/colours and batches of up to 50 puzzles. Optional answer pages follow the puzzles.
- Exhaustively verify every target word in all eight directions, rejecting missing/ambiguous matches, duplicate/reversed-duplicate words and invalid input. Difficulty is a direction rule; reading age requires vocabulary review.
- Save grid data, placements and recipes in portable projects. Rebuilding styling keeps the grid/answers and added artwork. Template insertion remaps recipe identities.
- Generated-layer changes block checked exports until rebuilt or explicitly detached. Detaching retains ordinary editable pages and ends automated answer validation.
- 63 release checks include 180 seed/difficulty combinations, 8/24 grids, 50 puzzles plus 50 answer pages, fresh-process recipe editing, modified-output detection and independent PDF/solver proof checks.

## 1.7.0 — book production

- Added Book controls for thumbnail page organisation, independent duplication, reordering, facing-page margins, headers, footers, folios and front-matter numbering.
- Added reusable master artwork with explicit edit/save/cancel and per-page inheritance, including true blank backs. Master images/fonts are included in portable saves and validation.
- Batch-import images as artwork pages in the background, with contain/cover placement and margin refitting. Concurrent edits preserve the completed import as a separate portable project.
- Image buffers and embedded field photos now share immutable storage; pixel editing detaches the edited copy. Existing document encodings remain compatible.
- 57 release checks passed, including a synthetic 50-page workbook, independent PDF rendering, range export, fresh-process master assets and copy-on-write pixel independence.

## 1.6.0 — linked collections and CSV production

- Added collection text/photo fields and named shared typography styles. Preview and PDF/PNG/SVG/HTML resolve them without changing the editable master.
- Personal masters may retain unfilled required fields; delivery exports identify missing data. Optional empty photo fields are omitted. Embedded field photos travel with portable projects.
- Added background CSV production with quoted Unicode values, local relative photo paths, per-record preflight and JSON completion reports. Successful records receive PDFs and raster proofs; editable working copies are optional.
- Validated a five-page coordinated collection, photo reuse/portability and a 52-record batch: 50 valid records completed, two invalid/overflow records reported.

## 1.5.0 — print guides, output presets and portable projects

- Added compressed, versioned `.canvas` projects containing the editable design, image assets and imported fonts. Open them through the ordinary project dialog; save a portable copy from Export. Checked loading rejects damaged archives, unsafe asset names and oversized contents.
- Added persistent safe-area and bleed overlays, editable millimetre guides and snapping to custom guides. Editing guides are excluded from exported artwork; preflight identifies text entering the safety margin.
- Added named output presets stored with the document/template, including DPI, bleed, crop marks and transparency. Export controls scroll on smaller screens.
- Added cold-process portable-file, moved-resource, failed-save preservation, damaged-package and print-guide coverage.

## 1.4.0 — paragraph layout and character emphasis

- Added before/after paragraph spacing, first-line indentation and four text-frame insets in points.
- Select characters in the canvas editor to apply bold, italic, underline or an actual font face in Typography; clear character styles to return to the frame defaults.
- Unicode character ranges follow ordinary text insertion/deletion. Uppercase expansion and wrapping preserve the original style mapping.
- Preview, raster, SVG and PDF use the same formatted text layout, including continuous underline spans. Old documents keep their existing layout when paragraph formatting is unset.
- Added mixed-style Unicode/editing and independently rendered paragraph proofs to the regression suite.

## 1.3.0 — portable imported TrueType fonts

- Import individual static TrueType faces, select their actual weight/style, and embed their bytes in editable projects and personal masters.
- Imported faces are shared by preview, PNG, SVG and embedded-font PDF output; PDF font metrics now come from the actual face.
- Opening a document validates imported-font identities and payloads. Missing fonts produce errors instead of silently changing production output. Inserting template pages retains their fonts.
- Static TrueType outlines with editable embedding permission are supported. Variable fonts, collections and CFF OpenType outlines remain unsupported.
- 41 release checks passed, including cold-process font loading, Unicode PDF text extraction, embedded-font inspection, missing/corrupt-font rejection and legacy compatibility.

## 1.2.0 — first print-studio milestone increment

- Added **My Templates**: save a production master with a name, tags, image assets and preview; search the library and open independent working copies. Invalid library entries are reported without hiding healthy templates. Failed saves do not leave partial template entries.
- Added selected-text controls for point size, line height and tracking, with an overflow warning.
- Unified layer-panel and keyboard group operations, including complete-selection nudge/delete and protection when a group member is locked.
- Added numeric layer position and size in millimetres, inches or layout pixels, aspect lock, rotation and flips. Invalid geometry is rejected before mutation.
- New documents start as A4. Standard heading/body presets use dark ink; newly inserted ordinary shapes and sample images no longer enable unsupported shadows by default.
- Decorative preview effects are labelled so their production-export restriction is visible before use.
- Added fresh-process library portability, protected-master, invalid-entry, physical-unit and non-mutating panel checks. The complete local suite now has 38 passing checks.

This starts milestone 2. Imported fonts, rich text, linked styles, advanced print guides and single-file project packages are still pending. Later book/personalization/puzzle milestones are not implemented by this increment.

## 1.1.1 — production workflow fixes

- All seven included templates use exportable defaults. Badge backgrounds remain editable shapes; unsupported demo charts become labelled editable bar graphics. General chart editing still has the documented export restrictions.
- Group and multiple-selection duplication/copy/paste preserve all members with fresh element and group identities. Group handles scale proportionally and rotate around the selection centre; locked members prevent group transforms.
- Layer ordering and lock controls act on the complete selection.
- Template insertion fits every source page proportionally into the destination document, rather than copying only the first page at incompatible dimensions.
- SVG and HTML exports now return explicit errors for unsupported content. Callers validate before writing the destination file.
- Image enlargement and patch-blending controls describe the local algorithms they actually run.
- Added end-to-end template save/reopen/export checks, independent Poppler PDF renders and group/selection regression coverage.

The original 1.1.0 release remains available as a fallback. Native desktop operation and physical printing still require manual validation.
