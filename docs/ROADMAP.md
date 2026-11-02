# Print and digital product roadmap

The detailed baseline assessment is [USEFULNESS-ASSESSMENT-2026-11-01.md](USEFULNESS-ASSESSMENT-2026-11-01.md). It describes version 1.1.0, not the corrected current behaviour. Acceptance scenarios are in [WORKFLOW-ACCEPTANCE.md](WORKFLOW-ACCEPTANCE.md).

| Stage | Status | Scope |
| --- | --- | --- |
| 1. Trustworthy starter editor | Implemented; automated and PDF proof checks added | Exportable template defaults, complete selection copy/transform, size-aware page insertion, checked SVG/HTML and accurate local-tool descriptions. A targeted native-window workflow passed in 1.8.1; broader platform/physical acceptance remains open. |
| 2. Useful personal print studio | Core implemented through 1.5.0 | Implemented personal masters with tags/previews/search and independent copies; point size/leading/tracking/overflow controls; numeric geometry and print-first defaults. Imported static TrueType faces added in 1.3.0. Paragraph spacing, insets and mixed character styles added in 1.4.0. Guides, single-file delivery and saved output presets added in 1.5.0. Foundations implemented; complete-product and manual acceptance remain open. |
| 3. Product production | Core implemented through 1.7.0 | Fields, shared styles, embedded field photos and background CSV production implemented in 1.6.0. Page organisation, book masters/folios/margins, background artwork placement and a 50-page acceptance fixture added in 1.7.0. Core production workflows implemented; native/physical acceptance remains open. |
| 4. Specialist product tools | Word-search workflow implemented in 1.8.0 | Seeded grids, direction/density controls, 50-puzzle batches, checked answer pages, portable recipes and styling regeneration. Mazes and other puzzle families remain future scope. |
| 5. Customer editing | Local browser workflow implemented in 1.9.0 | Portable-project personalisation, text/photo/layout controls, native previews/exports, recovery and revision checks. Browser/desktop round trips tested in isolated processes. Public multi-user hosting remains future scope. |

Stages are accepted through completed product workflows, not through the existence of toolbar controls. Keep changes incremental and preserve old document compatibility.

The native Linux application can now host local browser editing; see [BROWSER-EDITOR.md](BROWSER-EDITOR.md).
Hosted public customer links, additional puzzle families, PPTX and fillable-PDF editing remain future scope.
Broader desktop-platform/physical-print acceptance and a full-resolution production-book benchmark remain open.

## Professional studio, agent control and motion

The [2 November studio review](PROFESSIONAL-STUDIO-REVIEW-2026-11-02.md) is the
current direction beyond the print foundations. Versions 1.10–1.11 implement an
18-design / 27-page visual catalog, refreshed native/browser interfaces and a
working live-browser STDIO MCP adapter. See [agent control](AGENT-CONTROL.md).

Next: shared layout commands and campaign components/brand tokens; then a real
media timeline and audio/proxy pipeline; then a deterministic HyperFrames
adapter; then practical optional Three.js product mockups. Video, HyperFrames
rendering and 3D editing are planned work, not current app capabilities. Each
stage has an acceptance workflow in the review. Local storage is sufficient;
remote hosting remains a separate decision.
