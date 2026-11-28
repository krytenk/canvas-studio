# Canvas Studio usability review and next product milestones

Review date: 27 November 2026. Baseline: 1.19.0; bounded corrections: 1.19.1.

Canvas Studio is a capable personal production tool. It is not yet a beginner-ready hosted product. Its strongest direction is helping small publishers and creators turn a brief into a finished product and its marketing materials. The next development work should make those jobs easier to finish before increasing the number of tools.

## Evidence and limits

The review covers the current browser Design, Video, Mockups and Generate workspaces, template discovery, help, the native navigation and production implementation, existing native 1.19.0 window proofs, and the shared engine and delivery boundaries. Reference Desk's local source was reviewed for integration opportunities. No new GPU job was required for this review.

This is an expert workflow review, not a study with independent customers. Existing automated acceptance checks establish specific working paths; they do not establish that a new user can discover them. Physical print, a full screen-reader audit, Windows/macOS support and public multi-user hosting remain separate acceptance work.

## What works well

- The generated-media default is now model, prompt, optional references, Generate. Technical controls are disclosed separately.
- The 42-template catalog provides useful editable starting points, with search and categories.
- Designs, media and production use one document engine, with portable projects, undo, recovery and guarded revisions.
- Video and product mockups have dedicated workspaces rather than covering the canvas with unrelated controls.
- Print preflight, validated word searches, linked fields, batch production, campaign exports and book/card/box mockups provide a practical publishing foundation.
- The MCP adapter already allows agents to inspect, compose, preview, check and export designs; an agent integration does not need to start again from zero.

## Corrections in 1.19.1

1. **Video navigation:** clicking Designs, Pages or another rail tool could change a hidden panel while leaving Video on screen. Video now closes before the clicked tool opens. An explicit Back to design action is also available.
2. **Empty template search:** an empty grid gave no useful next step. The library now explains the result and offers Show all templates, clearing both text and category.
3. **Language:** the video library explains smaller preview files without requiring users to know what a proxy is.

Browser regression coverage checks all eight Video-to-tool routes, the explicit return action and empty-search recovery, alongside existing editing, delivery, revision-conflict, recovery and responsive checks.

## Remaining usability work in priority order

| Priority | Observed issue or implementation gap | Proposed change | Acceptance |
| --- | --- | --- | --- |
| P0 | Native Home, native tool docks and browser navigation organise the same capabilities differently. The browser does not expose the current native book, puzzle and CSV authoring controls. | Define shared job names and a capability matrix. Provide the supported browser workflow directly; explain unavailable capabilities before someone starts. | A new user can identify where to create a book, a campaign and a video without assistance. Existing projects still round trip. |
| P0 | An empty document opens with an empty Fields inspector and a blank canvas. Native Home leads with many format presets. | Start with four task cards: Design a product, Create a campaign, Make a video, Generate an image. Offer a working demo, recent projects and a blank custom size secondarily. | First template personalisation and a successful export within five minutes for at least four of five new testers; no key required. |
| P0 | Recovery on this device, downloaded editable copies and native Save are different storage actions. Old app servers can remain open on different ports. | Use explicit save location/status and unsaved-form notices; display engine version in both interfaces and identify older sessions. Offer recent portable files and clear recovery choices. | Users can explain where their project is saved, reopen it, and identify their installed version. No silent overwrite of another session. |
| P1 | Template names are stylish but do not always identify the job; browser preview shows only the first page. Print previews lead with pixel sizes. | Add practical subtitles, format/ratio and physical trim labels; all-page previews, template favourites and task-based search tags. | Someone looking for a wedding newspaper, Reel cover or US Letter worksheet finds the right format without guessing a collection name. |
| P1 | Export controls are distributed between global PDF, page PNG, video MP4 and mockup output. Global PDF stays visible during video work. | Add one context-aware Export action with format, destination, size, likely wait and a small proof preview. Retain editable-file saving as a separate clear action. | Correct PNG, PDF, MP4 or launch bundle can be chosen without reading technical documentation; preflight errors link to the affected page/layer. |
| P1 | Video timing inputs use frames, and useful next actions compete with advanced setup. | Use seconds/timecode by default, keep frame precision under Advanced; add aspect presets and a three-step empty-state guide. | A user can import a clip, trim it, add captions and export without converting seconds into frames. |
| P1 | Mockup lighting, proportions and scene settings require art-direction decisions. | Provide ready scenes with sensible proportions, artwork fit and optional Advanced adjustments. | A selected design becomes a listing image in three decisions: product type, scene, export. |
| P1 | A stored provider credential can look like a live healthy connection. A model listing does not establish runtime support or output quality. | Show Keys saved versus Connection checked; distinguish Tested here, Available through provider and Experimental. Recommend a small tested model shortlist. | No model is advertised as one-click supported solely because a metadata search found it. Errors explain the next safe action without exposing keys. |
| P1 | Dense small labels and long panel forms remain, especially on smaller screens. | Review contrast, keyboard focus, target size, semantic selection states and inspector disclosure at 1024×680 and 700px widths. | Complete keyboard-only core workflow, screen-reader-labelled controls, no hidden primary action or horizontal overflow. |
| P2 | Workflows span different libraries and output histories. | Add a project asset drawer with search, provenance, used-in indicators and storage cleanup; preserve private references and originals. | A user finds a previous creation and reuses it without searching job folders. |

## Useful additions after usability consolidation

1. **Complete product launch packs.** One project produces the printable product, listing photos, a short reveal, a thumbnail and social crops. Build on existing campaign and mockup commands; add a guided assembly workflow and meaningful presets.
2. **Transcript and caption workflow.** Import timestamped TXT/SRT and owned media from a local reference tool; select passages as storyboard notes or caption drafts. Existing SRT and media import cover the first integration step. Editing a transcript must not silently edit footage until a user accepts a proposed cut list.
3. **Production-focused templates.** Add coordinated listing-image sequences, accessibility-friendly worksheets, complete printable event suites and book marketing packs. Add families with distinct composition and editable sample content rather than many colour swaps.
4. **Better photo finishing.** Add mask/segmentation-based background removal, editable crop/focal point, consistent shadows and optional image repair after model/license and quality checks. Existing edge-colour cutout is not equivalent to robust subject segmentation.
5. **Caption and video shortcuts.** License-reviewed local speech recognition, editable timed captions, caption styles, audio ducking, simple transitions and safe-area overlays. Test intelligibility and timing before adding large effect libraries.
6. **Agent design assistance.** Extend the current MCP with asset search, template suggestions, previewed atomic revisions and cost-aware render proposals. Require explicit public publishing and spending controls. Show a before/after preview and one-step undo.
7. **Publishing quality checks.** Printer profiles, trim/bleed and cover-spine workflows, reading-size previews, contrast and font checks, and customer proof approval. Validate against printer requirements and physical samples rather than claiming universal print readiness.

Do not prioritise arbitrary model execution, full Canva/CapCut feature parity, a template marketplace, full collaborative editing or a general-purpose public downloader before these workflows work for new users.

## Hosted product boundary

The current server is a private loopback control surface backed by the local Rust engine. It has one active document per process, app-owned files, credentials and subprocess tools. It must not simply be put behind a public tunnel.

Public hosting needs accounts, tenant-scoped project/media storage, export workers, safe job queues, encrypted provider credentials, quotas, metering, rate limits, retention/deletion and backup recovery. The existing MCP's desktop authority should not become an unauthenticated public administration interface. HTML/motion rendering and media decoding need isolated workers and resource limits.

Build the hosted editor around finished design and export tasks first. Add one commercially permitted image-generation provider with visible cost and bounded credits, then benchmark video. Keep local/private operation available separately. Paid GPU work must use an authorised paid provider or a separate agreement; user-supplied credentials do not override provider terms.

## Release and customer acceptance gates

- Preserve portable-project compatibility and old documents on every milestone.
- Test native and browser flows against the same practical jobs; record actual capabilities rather than implying parity.
- Use isolated test profiles; never include personal keys, reference images or private media in public fixtures.
- Run appropriate shared-engine, browser and native checks for a shipped change; record limitations separately from passing checks.
- Recruit five target users for unprompted first-use tasks before calling the app easy to use. Track completion, time, errors and repeated requests for help. Set success criteria before sessions.
- Public launch needs two-account isolation tests, bounded spending/retry tests, restore/deletion tests and a current model/provider licence review. These are launch requirements, not features already delivered by this patch.
