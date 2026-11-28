# Changes

## 1.20.0 — shared starting workflows

- Start new desktop and browser sessions with the same four tasks: Design a product, Create a campaign, Make a video and Generate an image.
- Preserve current projects and unapplied browser forms when visiting Home or changing tasks; continue or open saved designs directly.
- Use shared task descriptions and explain the remaining desktop-only production tools. Put native campaign setup with campaign colours and show the running browser version.
- Check first use, draft preservation, all four task routes, editable delivery/opening and a narrow viewport; keep native Home proofs with generation navigation checks.

## 1.19.1 — clearer workspace navigation

- Return from Video to the clicked design tool immediately, matching Generate and Mockups. Add an explicit Back to design button.
- Explain empty template searches and let users clear search and category together.
- Use plain language for preview files in the video library.
- Exercise all eight Video-to-tool routes and empty-search recovery in the browser regression.

## 1.19.0 — model, prompt, Generate

- Replace the technical generation screens in desktop and browser with a model
  selector, prompt, optional photos and one Generate button. Advanced settings,
  notebook tools and technical history stay in collapsed panels.
- Submit, monitor and download Kaggle outputs automatically in background workers,
  with ready previews and ordinary Use/Save actions. Reuse existing connections.
- Select reviewed fast image defaults; load saved character photos immediately.
- Preserve run identity on disconnection; Continue checks the same job without
  repeating GPU submission. Keep remote waits separate from local render slots
  and reject duplicate generation for an active document.


## 1.18.0 — reusable multi-image subject references

- Add Reference video in native/browser studios with one to four distinct views,
  ordered previews, add/remove/reorder controls and aggregate reference influence.
- Add private named reference sets, reusable across documents with source integrity
  checks; expose the same workflow through the live-editor MCP.
- Add the pinned Wan VACE 1.3B subject-reference recipe, preserving separate images
  in one video batch rather than treating them as collages or temporal keyframes.
- Reuse a private, version-pinned Kaggle input pack across shots; verify every
  ordered reference hash, full-set fingerprint, model revision and output checksum.
- Append generated shots after existing visuals, extending the timeline without
  silently truncating footage; preserve explicit agent timing and atomic rollback.
- Keep existing first-frame image-to-video compatible and reject extra images on
  single-reference recipes. Install/preflight VACE’s required prompt-cleaning dependency.


## 1.17.0 — image-to-video and private first frames

- Add Image to video in native/browser studios, with PNG/JPEG/WebP selection,
  preview, rotation normalization, fit/pad or crop framing and document isolation.
- Add a pinned small LTX 2B first-frame recipe, model-specific frames/FPS/decode
  settings, reference fingerprints and guarded workspace uploads for MCP agents.
- Send lossless references as versioned private Kaggle input datasets, outside
  notebook source limits; verify remote privacy and reuse inputs across retries.
- Prevent failed or pending reference replacements from using a previous persona.
  Check source-reference, model/revision and output hashes before importing clips.


## 1.16.2 — Kaggle inference runtime and result integrity

- Match unique private notebook titles to requested slugs, accept only confirmed
  SDK submissions, and record the actual canonical URL/version returned by Kaggle.
  Status, logs and output stay pinned to that version; timed-out uploads can be
  checked without an automatic duplicate GPU submission.
- Remove an incompatible optional torchao package in the disposable Kaggle
  environment and pin PEFT/bitsandbytes. Local desktop packages are unaffected.
- Load Wan video decoders in FP32 independently of the denoiser, reject non-finite
  video pixels, and record decoder/package versions in inference receipts.
- Check downloaded model/revision as well as output checksum. Show model names,
  remote GPU states and readable redacted failure logs in the studio.
- Clear incompatible image-only adapters when selecting a video model.
- Check pinned weight sizes before GPU submission. Use recipe-specific Wan BF16 /
  32 GiB defaults, probe BF16 execution before downloading, and keep remote model
  caches outside persisted notebook outputs.
- Verify real private Tesla T4 image/video generation through the app, result
  download, editable page/timeline import, playback and portable project recovery.
  Record tested settings and actual timings in the GPU acceptance report.


## 1.16.1 — Browser connection and model search repair

- A browser opened with the bare local address now asks for its private session
  link before accepting work. Same-tab private-link reconnection preserves entered
  connection fields; loopback token and origin checks remain enforced.
- Live catalog results, progress, empty states, provider errors and retries appear
  beside the search form, with source/query labels and saved-provider status.
- Hugging Face search and gated registration use the saved HF token. Metadata
  redirects strip bearer credentials on host changes and reject HTTP downgrades.
- Regression coverage includes a fresh unauthenticated browser, expired-session
  reconnection, entered-key preservation, private credential save and search states.


## 1.16.0 — Generation studio and expanded template systems

- Separate Design, Generate and Video studio workspaces in desktop/browser; 24 new
  editable template systems bring the library to 42 templates across 95 pages.
- Fourteen pinned image/video model entries, live Hugging Face/Civitai metadata
  discovery and supported safetensors registration, with licence and hardware
  distinctions rather than an unverified quality ranking.
- Shared finite Diffusers runner, explicit page references, SDXL Lightning adapter,
  FP16/BF16/FP32, transformer NF4 and CPU offload controls; PNG/MP4 result import.
- Private bounded Kaggle notebooks, optional official CLI submission/status/output,
  checksum receipts and secret-free source ZIPs; credentials stored outside Git.
- Fifteenth live-editor MCP tool and standalone six-tool Kaggle adapter.
- Rights/content-mode/document guards, notebook/worker/browser integration checks
  and actual tiny CPU inference acceptance. Production GPU runs require a connected
  eligible Kaggle account and remain a separate acceptance step.


## 1.15.0 — Product studio and desktop repair

- Three.js book, product-box and card-stack scenes with native editable front/back
  cover textures, camera/proportion/material/light controls and on-demand preview.
- Cancellable local listing PNGs and deterministic six-second MP4 reveals, with
  hashed editable source ZIPs, pinned renderer and bundled Three.js MIT license.
- Native art direction/poster review, checked static poster placement and Film
  import. Stale scene/cover output cannot be placed; existing snapshots stay explicit.
- Portable v5 scene metadata, a 14th MCP tool, and regression workflows for cold
  restore, repeated pixels, out-of-order seeks, cancellation and WebGL fallback.
- Repair obsolete Camva Linux shortcuts, preserve old launchers, launch from any
  working directory and log startup failures privately.


## 1.14.0 — editable motion and HyperFrames source delivery

- Added typed per-clip position, scale, rotation and opacity keyframes, cubic/linear/hold easing, presets and browser/native key controls. Splits and trims retain the animation clock; movement moves it with the clip.
- Added a local HyperFrames 0.8.206 adapter with deterministic frame seeking, native SVG/PNG artwork, frame-normalized video, the existing audio mix, runtime checks and cancellable snapshot jobs. Frame review prepares visible content only.
- Added source ZIP delivery containing readable HTML scenes, editable native master/originals, local media, dependency lockfile, font license and a renderer/input/version/hash manifest. External HTML edits do not round-trip into the native model.
- Motion projects use a guarded version-4 envelope; existing static, campaign and film files remain supported. MCP retains 13 tools with typed motion edits and source-job delivery.
- Added a pinned optional setup helper (Node 22+, FFmpeg/Chromium), repeated-render/split-frame equivalence, source-hash, audio-sync, cancellation and UI checks. Three.js, automatic transcription and public hosting remain future work.

## 1.13.0 — film, sound and captions

- Added a persisted frame-based multi-track timeline with verified local originals, disposable proxies/posters/waveforms, trim/split, source audio detachment, gain/fades, manual captions/SRT and native editable design overlays.
- Added a dedicated browser Film workspace with direct clip movement/edge trimming, framing/sound/caption inspectors, synchronized rendered MP4 review and stale-preview feedback. Native controls share the command engine; native AV review opens in ffplay.
- Added asynchronous, document-owned FFmpeg snapshot jobs with progress, cancellation, full-size MP4 delivery, frame review and storage reporting/finished-job clearing. Original media and edits survive render failure/cancellation.
- Split clips preserve both audio envelopes and source FPS sampling phase, including 24 fps footage on a 30 fps output timeline.
- Portable film projects use a guarded version-3 envelope containing SHA-256-verified originals. Local recovery references the private cache. Existing static/campaign formats remain supported.
- Expanded MCP to 13 tools, media import and typed film edits, asynchronous render/proxy requests, scoped job status/cancel/save. Added decoded-frame/audio, cold-cache restore, corruption, cancel and UI regression workflows plus a reproducible licensed 30-second editing demo.
- HyperFrames animation, keyframed effects, automatic transcription and 3D editing remain future work.

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
