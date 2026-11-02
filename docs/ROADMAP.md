# Print and digital product roadmap

The detailed baseline assessment is [USEFULNESS-ASSESSMENT-2026-11-01.md](USEFULNESS-ASSESSMENT-2026-11-01.md). It describes version 1.1.0, not the corrected current behaviour. Acceptance scenarios are in [WORKFLOW-ACCEPTANCE.md](WORKFLOW-ACCEPTANCE.md).

| Stage | Status | Scope |
| --- | --- | --- |
| 1. Trustworthy starter editor | Implemented; automated and PDF proof checks added | Exportable template defaults, complete selection copy/transform, size-aware page insertion, checked SVG/HTML and accurate local-tool descriptions. Manual desktop acceptance remains open. |
| 2. Useful personal print studio | Core implemented through 1.5.0 | Implemented personal masters with tags/previews/search and independent copies; point size/leading/tracking/overflow controls; numeric geometry and print-first defaults. Imported static TrueType faces added in 1.3.0. Paragraph spacing, insets and mixed character styles added in 1.4.0. Guides, single-file delivery and saved output presets added in 1.5.0. Foundations implemented; complete-product and manual acceptance remain open. |
| 3. Product production | Core implemented through 1.7.0 | Fields, shared styles, embedded field photos and background CSV production implemented in 1.6.0. Page organisation, book masters/folios/margins, background artwork placement and a 50-page acceptance fixture added in 1.7.0. Core production workflows implemented; native/physical acceptance remains open. |
| 4. Specialist product tools | Word-search workflow implemented in 1.8.0 | Seeded grids, direction/density controls, 50-puzzle batches, checked answer pages, portable recipes and styling regeneration. Mazes and other puzzle families remain future scope. |
| 5. Customer editing | Optional later milestone | A chosen recipient editing format or browser editor, tested independently of the creator's computer. |

Stages are accepted through completed product workflows, not through the existence of toolbar controls. Keep changes incremental and preserve old document compatibility.

Next optional extension: recipient editing beyond the native Linux application.
The current editable delivery route is a `.canvas` file opened in a recent Canvas Studio build; browser editing, PPTX and fillable-PDF editing are not implemented.
Manual desktop/physical-print acceptance and a real full-resolution production-book benchmark remain open.
