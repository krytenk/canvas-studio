# Canvas Studio: usefulness assessment and development direction

Assessment date: 1 November 2026. Baseline: Canvas Studio 1.1.0, commit `84d3e0b`.

This is the retained baseline assessment. See [ROADMAP.md](ROADMAP.md) and
[the changelog](../CHANGELOG.md) for fixes implemented after the assessment.

## Decision

Continue the project. The current code provides a useful base for a personal, offline print and digital design tool. It does not yet provide a dependable replacement for the relevant parts of Canva.

The chosen priority is **books, puzzles, stationery and editable templates**. The strongest direction is an editor that understands those products: reusable page systems, strong typography, checked print output and fast personalization. Reproducing the rest of Canva would add substantial work without establishing that this tool saves time on those jobs.

Keep the Rust application, document model, asset storage, recovery and rendering work. Fix the workflow defects before expanding the feature menu. There is no evidence from this review that a complete rewrite is needed.

This is a technical usefulness assessment, not a claim about customer demand or a full reverse engineering of Canva's service.

## Evidence and limits

The assessment combines source inspection, a fresh run of the existing release tests, an executable self-check, focused diagnostics on an isolated source copy, and current public Canva documentation. Application source was not changed by the investigation.

Verified during this assessment:

- `cargo test --release --offline --locked`: **28 passed, 0 failed**.
- Standalone executable: version 1.1.0; all **10 embedded font faces** available.
- Seven built-in templates inspected through the actual export preflight.
- Group duplication reproduced through the application's own grouping and duplication methods.

The additional diagnostic used an isolated copy so the application and its existing tests remained unchanged. The diagnostic's passing assertions confirmed the problematic current behaviour; they do not mean those workflows are correct.

Not established here: a complete manually operated desktop session, file-dialog behaviour, physical printer output, Windows/macOS operation, usability with a new customer, or memory/latency limits for large illustrated books. Existing tests include headless UI and independently rendered PDF checks, but those do not replace these missing checks.

Source references below identify the baseline implementation. Line numbers will move as it evolves.

## What is already worth keeping

| Capability | Evidence | Practical usefulness today |
| --- | --- | --- |
| Editable documents and multiple pages | `src/model.rs:679`, `src/model.rs:818`, `src/model.rs:944` | Text, shapes, photographs, drawing paths, charts and tables have document representations. Several complex types have export limitations. |
| Basic canvas interaction | `src/canvas_view.rs:113` | Selection, movement, resizing, rotation, snapping, pan/zoom, drawing and inline text entry exist. Group behaviour is incomplete. |
| Physical dimensions | `src/model.rs:42`, `src/model.rs:1084` | A4, Letter, 5×7 inches and custom dimensions are real features, including fractional physical sizes. |
| Local project storage | `src/exporter.rs:1507`, `src/exporter.rs:1572` | Checked saves, a previous-save backup and strictly loaded companion image assets. The JSON and assets must travel together. |
| Undo and recovery | `src/main.rs:486`, `src/main.rs:871`, `src/model.rs:1327` | Revision-aware undo/redo and per-document recovery are covered by regression tests. |
| Shared export foundations | `src/render_scene.rs:415`, `src/text_layout.rs:58` | Supported text, shapes, images and expanded table/path primitives share significant layout/rendering logic. Preview and export still have separate special cases. |
| Print output | `src/pdf_export.rs:335`, `src/pdf_export.rs:1266` | Embedded-font PDF, page selection, physical trim dimensions, bleed, crop marks and PDF page boxes. This is not a certified PDF/X workflow. |
| Screen output | `src/exporter.rs:24`, `src/exporter.rs:638` | SVG, PNG and JPEG; SVG embeds used bundled fonts. SVG is not a lossless editable project round trip. |
| Layout import | `src/import_layout.rs` | Structured JSON layouts and relative image paths can bring prepared work into the editor. This is useful for automation. |
| Basic photo adjustments | `src/magic_studio.rs:9` | Local brightness, contrast, colour adjustments, crop/fit and simple cutout/filter operations. |

These are substantial foundations. Their existence is different from proving that a whole book or stationery collection can be completed comfortably.

## Current suitability for the chosen work

| Job | Assessment | Main constraint |
| --- | --- | --- |
| Make a simple printable page with supported elements | Usable with proofing | Avoid unsupported effects; inspect the export. |
| Produce a coordinated stationery collection | Partly usable, labour intensive | No linked customer fields, shared collection styles or saved user-template library. |
| Design a premium newspaper or editorial product | Partly usable | Limited fonts and text structure; no column-flow or rich paragraph system. |
| Assemble an illustrated book | Possible through manual pages/import | Missing master pages, folios, facing-page margins, page organisation and batch image placement. Large-book performance is unmeasured. |
| Generate a puzzle book with verified solutions | Not implemented as a workflow | Tables and drawings are not puzzle generation or validation. Externally prepared puzzles could be placed as artwork. |
| Keep editable masters for personal reuse | Supported, with limitations | Native files are local and must include image assets; opening a file is not a protected template-copy workflow. |
| Deliver a ready-to-print download | Supported for compatible layouts | Needs complete output proofing and a convenient packaging workflow. |
| Deliver a fully editable product to a typical customer | Not established | Native app installation and platform support, customer-facing editing controls and delivery formats are unresolved. |
| Make an advertising film | Outside the useful first scope | Presentation animations exist, but there is no video/audio document model or rendered MP4 workflow. |

## Confirmed problems to fix first

### 1. Every built-in template currently fails print/raster export preflight

The templates can load and look like demos, but their default contents include features the production exporters reject. The high-level PNG/JPEG path uses the same preflight as PDF, so this affects those outputs as well.

| Template ID | Pages | Fatal preflight issues at 300 DPI | Categories |
| --- | ---: | ---: | --- |
| `pitch_deck` | 2 | 10 | Unsupported effects and chart content |
| `social_launch` | 1 | 4 | Unsupported effects |
| `yt_thumbnail` | 1 | 4 | Unsupported effects |
| `festival_poster` | 1 | 2 | Unsupported effects |
| `story_reel` | 1 | 3 | Unsupported effects |
| `infographic_report` | 1 | 1 | Unsupported chart content |
| `brand_logo` | 1 | 2 | Unsupported effects |

Evidence: `src/templates.rs:16`, `src/templates.rs:86`, `src/pdf_export.rs:81`, `src/exporter.rs:638`. The existing seven-template test (`src/main.rs:3940`) checks construction and dimensions, not successful production export.

**Fix:** give every production template an exportable default. In the short term, remove unsupported decorations from those defaults and replace unsupported chart elements with supported primitives where appropriate. Keep preflight protection. Then implement desired effects through shared rendering, with a bounded raster fallback for effects that cannot be represented faithfully as vectors. Preserve ordinary text as text.

**Acceptance:** load each production template, edit it, save/reopen it, and export PDF and PNG successfully. Independently render each PDF and compare composition with the preview. A screen-only template, if retained, must be clearly labelled before selection.

### 2. Groups are not complete reusable objects

Selecting and moving a group includes its members. Duplicating it calls a method that clones only the primary element. In the diagnostic, a selection of two grouped elements appended **one** element, and that copy retained the original group ID. Copy/paste likewise holds only one element.

Evidence: `src/main.rs:624`, `src/main.rs:681`, `src/main.rs:889`, `src/canvas_view.rs:397`. Resize and rotation handle mutations also operate on the primary selected element (`src/canvas_view.rs:528`), rather than applying a transform to a complete selected group.

**Fix:** make selection-wide operations explicit. Duplicate/copy/paste a selected set, allocate fresh element IDs, remap each original group to a fresh group ID, and select the complete result. Apply group scaling/rotation around the selection bounds; preserve relative geometry. Define lock and aspect-ratio behaviour consistently.

**Acceptance:** duplicate a three-part invitation motif, move and resize the copy, undo/redo and reopen the file. The original must remain unchanged, and the new motif must retain all its parts and their relationships.

### 3. Adding a template page does not account for a different document size

The “Add as New Page” path takes the first template page and appends it to the current project without fitting its elements to the destination dimensions. For example, presentation coordinates can be inserted unchanged into an A4 document. This is a source-confirmed mismatch risk; it was not manually exercised in the native window during this assessment.

Evidence: `src/main.rs:2308`.

**Fix:** check source and destination geometry. Offer an explicit fit-to-page conversion with a preview, or reject incompatible sizes until that conversion exists. Import all requested pages, not implicitly just the first. Keep “replace project” separate from “insert pages.”

**Acceptance:** inserting a landscape template into a portrait page produces a predictable fitted result or a clear size-choice dialog. Nothing silently disappears outside the page.

### 4. Export support is inconsistent between formats

PDF and high-level raster export refuse known unsupported elements. SVG returns a string without the same preflight contract, and its shared scene does not carry every preview effect. HTML slides are built from that SVG output. There is no basis for assuming every feature visible in the editor survives every output format.

Evidence: `src/exporter.rs:24`, `src/exporter.rs:638`, `src/exporter.rs:1336`, `src/render_scene.rs:415`, `src/pdf_export.rs:81`.

**Fix:** define a capability matrix per feature and export format. Add target-specific preflight and show exactly which layer needs attention. Choose faithful output, an explicit flattening option, or a clear refusal. Do not silently lose a decoration or describe unsupported content as fully editable.

**Acceptance:** a small reference sheet covering supported effects, masks, borders, gradients, tables and text has verified preview/PDF/PNG/SVG behaviour. Export warnings lead to the relevant layer.

### 5. Some tool descriptions promise more than their implementations

| Tool | Actual current operation | Appropriate next step |
| --- | --- | --- |
| Background removal | Samples perimeter colours and reduces alpha for similar pixels | Keep as a simple background-colour cutout. A portrait-quality remover would need segmentation and edge refinement. |
| 2× enlargement | Bilinear interpolation followed by sharpening | Correct the “Bicubic/Catmull-Rom” UI claim; retain as ordinary resampling. It does not reconstruct missing detail. |
| Patch inpaint | Interpolates neighbouring pixels across a rectangle; UI targets the centre patch | Call it patch blending and add region selection if useful. Treat scene-aware object removal as a separate later capability. |
| Palette composition | Samples four image positions and adds decorative cards, swatches and text | Keep as a composition helper. It does not recover the original picture's objects or editable layers. |
| Writing helper | Inserts fixed strings based on a topic and tone | Present as copy presets. An optional language model is a separate integration. |
| Automatic resize | Uniformly scales and centres most elements, with a full-bleed heuristic | Keep the basic fit operation; add anchors and layout rules for reliable format variants. |

Evidence: `src/magic_studio.rs:9`, `:212`, `:262`, `:306`, `:446`, `:527`; tool labels in `src/main.rs:2110`.

For the selected print-first purpose, most sophisticated AI replacements can wait. Correct labels make it easier to judge and trust the tools that are already useful.

## Missing capabilities that would materially improve personal production

### A. Typography and precise layout — essential

The text model has one content string and one overall style per element (`src/model.rs:679`). The font selector has five roles backed by Noto Sans, Noto Serif and Noto Sans Mono; several roles are styles of the same family (`src/model.rs:367`, `src/font_registry.rs`). It is not an extensible user font library.

Add:

- Imported local fonts, searchable previews and explicit missing-font substitution. Preserve the font identity in the project and check whether embedding is permitted.
- Styled text spans for emphasis within a paragraph, reusable paragraph/character styles, paragraph spacing, visible leading/tracking controls and text-box inset controls.
- Clear overflow markers, optional bounded shrink-to-fit and a minimum readable size. Longer text must not silently damage a layout.
- A position/size inspector with units, aspect lock, alignment/distribution, reusable guides, margins and safe zones. Existing snapping and rulers remain useful.
- For newspapers or longer reading products: linked text frames and columns after the simpler type controls are dependable. Justification/hyphenation need deliberate typography work, not extra spaces.

**Technical approach:** replace the fixed font-role-only choice with stable font references and a registry; migrate existing roles without losing old projects. Introduce styled runs while preserving old single-style text. Have preview and export consume the same laid-out glyph data. Add layout commands rather than implementing each new operation separately in toolbar callbacks.

**Proof of usefulness:** an invitation with mixed emphasis and an editorial page with long text can be edited and exported without rebuilding separate text fragments by hand.

### B. A library of reusable templates, assets and styles — essential

The current template list is seven compiled-in examples. Brand settings are stored within a document; the UI offers a name, fixed palette swatches and colour shuffling. Uploads are a file chooser and four generated sample assets. These are not a personal reusable production library.

Evidence: `src/templates.rs:16`, `src/templates.rs:1034`, `src/main.rs:2308`, `:2684`, `:2767`, `src/model.rs:847`.

Add:

- “Save as template,” preview thumbnails, tags/search, favourites and “new copy from template.”
- Personal asset folders and reusable motifs/element groups, with original file references and attribution/licence notes where relevant.
- Editable brand colours, fonts and logos saved independently of one document.
- Named style roles such as heading, body, accent and background, so changing a collection's style is controlled rather than a random recolour.
- Linked collection styles with an explicit “apply to this page/all pages” choice.
- Recent files, document thumbnails and a clear separation between a master and a customer's working copy.

**Technical approach:** use an application-level library index and portable template packages. Keep libraries separate from `Project` records. A template copy gets its own document/recovery identity and does not overwrite the master. Store style IDs with local overrides rather than only copying colours.

**Proof of usefulness:** create a coordinated five-piece stationery collection, change the accent colour and heading font once, and generate a fresh copy without touching the master.

### C. Named fields and batch personalization — very high value

There is no named-field binding or CSV merge pipeline in the reviewed document model/import/UI. Structured layout JSON import is a useful starting point but does not provide this workflow itself.

Add text and image fields, a form to enter values once, field validation, live proofing, and CSV import/export. A field such as `event_date` should update every linked occurrence. Longer values need an explicit overflow policy. Batch runs need a preview, output naming and per-record error reports.

**Technical approach:** keep field definitions separate from design elements; elements reference field keys and formatting rules. Instantiate a design from a master plus a data record without destructively replacing the master. Make CSV a thin input adapter, not the document format. Start with local images; do not add remote fetching to the first implementation.

**Proof of usefulness:** personalize an entire event collection from one form, then produce 50 place cards from a guest list. A deliberately long name must be caught before delivery.

Canva's documented Bulk Create also separates imported data from connected design fields. That is a useful workflow reference, rather than a requirement to reproduce its infrastructure. [Canva: Create designs in bulk](https://www.canva.com/help/bulk-create/)

### D. Books and page production — essential for books

Multiple pages and add/duplicate/delete controls already exist. A book needs more than a list of pages.

Add:

- A page organiser with real thumbnails, reorder, insert, duplicate and selected-page operations.
- Master pages, automatic page numbers and consistent headers/footers.
- Facing pages, inner/outer margins, gutters and explicit blank-page handling.
- Batch artwork import with fit/crop options, per-image resolution checks and a chosen placement template.
- Per-product trim/bleed/safe-area profiles and output presets. Use the actual print provider's specification; do not hardcode a universal “book-ready” rule.
- A job summary with page count, dimensions, missing art, overflow, expected export files and proof status.

**Technical approach:** layer book settings and master-page references over the existing page model. Expand inherited elements into the render scene while retaining editable overrides. Page numbers are generated fields, not manually typed text.

**Proof of usefulness:** assemble a 50-page illustrated workbook, reorder a section, update the master footer, save/reopen and export a correctly ordered PDF. Measure responsiveness and peak memory on a representative image-heavy version before claiming long-book readiness.

### E. Print proofing and product packaging — essential

The current PDF implementation already writes trim/bleed/media boxes and supports crop marks. Do not rebuild these features under new names.

Add visible trim/bleed/safe overlays; checks for important content near the trim; reliable image extension into bleed; page-aware warnings; saved output presets; batch raster export; and a delivery package containing the intended files and concise instructions. Gradient/image backgrounds and content crossing a trim boundary need specific bleed proofing. Their correctness was not established merely by the existence of PDF boxes.

A portable single-file project container would remove the “remember both JSON and assets” burden. A first version can package the existing format rather than replacing it: versioned manifest, relative assets, integrity validation and atomic save/recovery behaviour.

Colour-managed conversion, ICC profiles and PDF/X are conditional requirements for particular print workflows. They are not prerequisites for every home-printable product. The existing PDF uses RGB content; obtain the destination requirements before selecting a professional colour pipeline.

**Proof of usefulness:** export A4 and Letter customer files plus preview images, open the package on a fresh process, render the PDF independently and check print dimensions. No missing assets, unexpected clipping or unannounced substitutions.

Canva documents margin, safe-zone, bleed and crop-mark workflows; its current help page describes its margins/bleed sizes as fixed. Canvas Studio already has adjustable bleed, providing a useful starting point for provider-specific controls. [Canva: Margins, bleed, rulers and crop marks](https://www.canva.com/help/margins-bleed-crop-marks/)

### F. Puzzle and activity modules — a useful way to make this app distinctive

These are product-specific additions, not features that must exist before a reliable general editor is useful.

Start with a word-search module: theme/word list, difficulty controls, seeded generation, grid size, printable typography and answer-key pages. Validate that every required word is present in the allowed directions and that the solution matches the puzzle. Define how duplicate words and accidental matches are handled. Keep puzzle data in the project, so a design edit does not destroy reproducibility.

Later add maze or other puzzle types only when each has a solver/validator and useful production controls. A good-looking grid with a wrong answer key is a failed product.

For colouring/activity books, start with batch artwork placement, line-art-friendly proofing, single-sided/blank-back options and consistent age-oriented text sizing. Artwork generation is a separate optional tool; assembling a good book does not require generating its art inside the editor.

**Proof of usefulness:** generate a small puzzle collection and its answer section, verify it automatically and by inspection, change the visual style, then regenerate identical puzzle content from the saved seed.

## What “editable templates” must mean

There are three different deliverables:

| Deliverable | What it requires | Recommended position |
| --- | --- | --- |
| Editable personal master | Native project, portable assets, library and safe template-copy workflow | First target. Much of the underlying storage exists. |
| File editable in another application | A specifically supported format with font/element mapping and recipient-side tests | Add only the format actually needed. Existing SVG can help with vector interchange; it is not an all-purpose template experience. PPTX is currently absent. |
| Customer opens a link and edits online | Browser editing surface, isolated customer copies, asset handling, persistence, font rendering and download/proof workflow | Separate later milestone. It is not created by exporting an HTML presentation. |

A fillable PDF is another limited option for changing predefined fields, but it is not a fully editable layout and the current PDF writer does not produce form fields.

For now, prioritize editable internal masters and personalized finished files. This creates practical value without requiring customers to install a Linux application. If customer self-editing becomes necessary, choose the required delivery route explicitly and test it with a recipient before advertising it.

Canva's template links let recipients use a design as a template; that is a separate product experience from downloading a finished PDF. [Canva: Share a design as a template link](https://www.canva.com/help/share-template-link/)

## A development sequence with clear stopping points

| Stage | Work | Exit condition |
| --- | --- | --- |
| 1. Trustworthy starter editor | Exportable default templates; correct group/copy operations; size-aware template insertion; format-specific export checks; accurate tool labels | A user can complete, save and export the included examples without surprises. |
| 2. Useful personal print studio | Font import, practical type controls, precise layout inspector, guides/safe zones, user-template library, portable packages and output presets | A real stationery collection and small illustrated workbook are completed entirely through normal app controls. |
| 3. Product production | Linked styles/fields, CSV personalization, page organiser/masters/folios, batch image placement, batch export and package generation | One customer form produces a consistent collection; a representative 50-page book passes proofing and measured performance checks. |
| 4. Distinctive product tools | Verified puzzle generator, answer keys, activity-book controls, collection variants and reusable recipes | The app reliably performs a product job faster than the prior manual method. |
| 5. Customer editing, if needed | Selected editable interchange format or a browser customer editor | A recipient can open, personalize and export without developer help. |

These stages describe dependencies, not calendar estimates. Stop adding scope once the app performs the jobs actually needed. It can be valuable before stages 4 or 5.

Keep the current implementation and make changes incrementally. As features are touched, move document operations out of the large UI module into reusable commands, and keep storage, text layout and render/export responsibilities separate. Do not spend an entire release reorganising code before demonstrating a completed product workflow.

## What can wait

For the selected purpose, defer real-time collaboration, enterprise roles/SSO, a stock marketplace, website publishing, spreadsheet replacement, complex whiteboards, social scheduling, mobile editing and a general video timeline.

Also defer built-in model training and broad generative AI integration. Optional image segmentation or writing models can later be adapters. The core editor should continue opening, editing, saving and printing without them. Local models still need hardware and maintenance; they are not automatically a free substitute for hosted tools.

Canva's video editor includes a layered timeline, captions and animation features. Rebuilding that is a separate substantial workstream, and does not resolve the print-product gaps. [Canva: Video editor](https://www.canva.com/video-editor/)

## How this becomes uniquely useful

The strongest proposed features are connected production workflows:

1. **One event brief, a coordinated collection.** Names, date, venue and colour choices populate an invitation, programme, welcome sign, table cards and newspaper; each retains its own layout rules.
2. **One book recipe, a checked interior.** Trim size, artwork folder, page structure, folios and answer sections generate a consistent book with a preflight summary.
3. **One puzzle specification, matching puzzle and solution.** Design styling changes without breaking puzzle data or reproducibility.
4. **One master, deliberate output variants.** Generate A4, Letter and screen previews with proofed layout rules, rather than treating proportional shrinking as completed design adaptation.
5. **A transparent production record.** Record sources, version, template, output settings and proof status so a repeat order can be reproduced.

Those are proposed differentiators, not claims of novelty or existing implementation. They make the project easier to evaluate: each has an end-to-end result that can be demonstrated.

## Recommended next implementation

Begin with stage 1 and a small print-first example collection. Preserve the current release as a fallback. The next development brief should require exported evidence and the acceptance scenarios below, rather than accept new toolbar buttons as completion.

Use [WORKFLOW-ACCEPTANCE.md](WORKFLOW-ACCEPTANCE.md) as the practical checklist. The app earns its place when it makes repeated real jobs easier, produces dependable files and preserves editable masters.

## Reference scope

The installed REA skill instructions were reviewed. They direct ordinary source analysis for a complete source repository. No decompilation, live Canva session inspection or private Canva API investigation was performed here. Future focused behavioural comparisons can examine a selected workflow when documentation leaves a concrete question unanswered.

Canva documentation was checked on the assessment date. Feature availability and account-specific behaviour can change. The comparisons here cover only the documented workflows cited above; they are not a comprehensive feature-parity score.
