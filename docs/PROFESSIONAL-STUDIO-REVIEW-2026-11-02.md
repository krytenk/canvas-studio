# Professional studio review and development direction

Assessment: 2 November 2026. Implemented work: versions 1.10–1.11.

## Judgment

The foundations are worth continuing. A common native document/export engine,
portable embedded assets/fonts, linked production fields, book masters, checked
puzzles and local browser editing are useful differentiators for a print-and-
digital production studio. The presentation had not caught up with those
capabilities. The next advantage should be completing a coordinated campaign
across print, social artwork and motion, with an agent helping inside the same
editable document.

The old interface used inconsistent neon accents, emoji tool icons with uneven
font support, dense undifferentiated controls and a catalog that described
examples without showing their artwork. Starter templates were too generic to
communicate an actual marketing use. A new colour palette alone would not solve
that: navigation, visible starting points, hierarchy and usable defaults matter.

This assessment is based on the source, export proofs and real native/browser
workflows. It is not a usability study with external customers or evidence that
these designs will increase sales. No conversion claims are made.

## What changed now

- **A coherent visual system.** Charcoal chrome, a restrained teal accent,
  neutral workspace, consistent vector tool rail, quieter panels and better
  visual hierarchy. Browser inspectors become drawers on narrower screens;
  panels remain usable at 700 px. The native rail remains scrollable at 680 px
  height. Dense controls still need further accessibility/usability work.
- **A visual design library.** Real native-rendered previews, category/search,
  multi-page metadata and full browser previews. Designs can start a document
  or have their pages fitted into the current one. Desktop saved masters and
  production tools remain available.
- **18 editable starting designs / 27 pages.** Seven Instagram designs,
  including a five-page educational carousel and three-part story sequence;
  six YouTube designs including three thumbnail approaches, a comparison,
  channel artwork and an end-screen artwork; two print designs plus a deck,
  infographic and identity starter. Text, colour blocks and illustration
  primitives remain editable. They are static design layouts, not rendered
  videos, interactive YouTube elements or licensed stock photographs.
- **A real local MCP foundation.** Eleven structured tools control the live
  browser studio. Batches are atomic, undoable and can be rehearsed without
  committing. Preview images let an agent inspect its work, while production
  checks guard delivery. Portable files bridge browser and native windows.
  See [setup and exact limits](AGENT-CONTROL.md).

The channel-art starter uses a 2560×1440 canvas and central safe guides. These
follow [YouTube's official channel-art dimensions](https://support.google.com/youtube/answer/10456525),
with safe-area values scaled from its minimum dimensions. Review the actual
channel preview on different devices before publishing; device cropping varies.
Other format sizes are chosen design presets, not a claim that every platform
placement or advertisement accepts them unchanged.

Existing projects remain compatible. Old template identifiers still build;
neutral newly added text no longer inherits a poster headline's tight tracking.
All catalog defaults pass the app's production checks, including text overflow.
A successful preflight establishes supported output, not artistic quality.

## Where the product is still weak

1. **Layout interaction.** Browser direct selection/drag and numeric geometry
   exist, but multi-select, resize handles, grouping, snapping/alignment and
   keyboard operation need a consistent interaction layer. Desktop capability
   should be reused through commands, rather than independently rebuilt.
2. **Brand and campaign reuse.** Shared text styles and fields exist, but named
   colour tokens, reusable components, per-format constraints and connected
   campaign variants are incomplete. Proportionally fitting a portrait page
   into a landscape page is not a good responsive design system.
3. **Asset management.** Embedded assets are portable, but a searchable library,
   licensing/provenance fields, deduplication visibility, replacement history
   and media proxies are missing.
4. **Accessibility and large-workflow proof.** Targeted contrast improvements
   are not a complete WCAG audit. Screen-reader navigation, keyboard-only
   editing, high-DPI/narrow screens, large libraries and representative long
   books need broader acceptance. Native Windows/macOS are not delivered.
5. **Motion and media.** There is no usable multi-track video/audio editor,
   synchronized preview timeline, media decode/proxy pipeline, caption editor
   or 3D scene system yet. Do not describe existing slides/transitions as a
   CapCut replacement.
6. **Hosted customer editing.** The current service is private loopback editing,
   not a public tenant-aware web service. Remote hosting needs a separate
   deployment/authentication/access/storage design.

## How the requested directions connect

| Direction | Useful capability to borrow | Shared foundation / next implementation | Acceptance before calling it done |
| --- | --- | --- | --- |
| Canva-style visual suite | Brand-aware, reusable layouts across formats | Versioned components/tokens, layout constraints and connected variants; one asset library and export engine | Build a real product campaign in print, portrait social, story and landscape thumbnail; edit a shared field/colour without breaking format-specific composition |
| CapCut-style editing | Trim/split, tracks, captions, keyframes and useful audio control | Asset records, rational timebase, timeline tracks, clip in/out, text/image/video nodes, deterministic keyframe evaluation, audio gain/fades, FFmpeg encoding | A 30-second story with real video, music/dialogue, captions and a CTA saves/reopens; exports with stable frames and synchronized sound |
| HyperFrames | Reproducible programmatic motion and agent-directed render jobs | A renderer adapter from the same document/timeline into seekable HTML scenes, native SVG/PNG assets and explicit placed audio | Seek arbitrary timestamps, inspect frames, render twice, compare key frames/duration/audio; retain the editable source and render manifest |
| Three.js | Useful 3D product/book mockups and dimensional titles | Optional scene node with camera/lights/materials/GLB assets, deterministic `renderAt(t)` and a baked poster frame for static exports | A book mockup uses a real cover texture; print poster and motion render agree; static editing idles without continuous GPU rendering |
| Coding-agent MCP | Inspect → edit → preview → critique → check → export | The implemented guarded command adapter, then the same command bus exposed to native editing and future timeline/scene nodes | Complete a representative design in the selected coding client, recover from stale state and visually review before delivery |

[Canva's visual suite](https://www.canva.com/visual-suite/) demonstrates the value
of connected formats, brand and editing workflows. The recommendation is our
own production-oriented implementation, not a pixel copy of its interface.
[CapCut's editing guide](https://www.capcut.com/resource/how-to-use-capcut)
shows why basic media editing and audio control belong before a large effects
catalog. The feature order above is an engineering judgment based on our app's
existing strengths; it is not a market-share ranking.

## Architecture to avoid three separate apps

Retain the current Rust model and production engine. Introduce a versioned
extension in small migrations, rather than replacing working print functions:

```text
DesignDocument
  assets + provenance + embedded/linked policy
  brand tokens + components + data fields
  pages/scenes + stable node IDs + campaign variants
  timeline + tracks + clip source ranges + keyframes
  optional scene3d + poster-frame fallback
            |
        validated command bus + history + revision checks
            |
  native UI | browser UI | MCP adapter
            |
  static SVG/PDF/PNG | HyperFrames scene | FFmpeg media | Three.js scene
```

Persist source timebase, duration, clip ranges, animation curves and renderer
version. Frame selection must use timestamps, not wall-clock elapsed time. Keep
static print text/vector content native; do not flatten an entire stationery
suite just to add motion. Put potentially expensive media decoding/rendering in
bounded workers. A future document feature manifest must identify required
motion/3D features so older builds fail clearly instead of saving away data they
do not understand. Existing static projects should migrate without visual change.

HyperFrames scenes should expose deterministic seek, with paused timelines and
`hf-seek` driving any Three.js scene. No uncontrolled animation loop, live
network request or random value should determine exported frames. Local
HyperFrames runtime, scene assets, CLI version and fonts must be recorded in a
render manifest. The installed HyperFrames skills describe this contract;
no HyperFrames runtime download, film render or paid generation was performed
for this assessment. Its integration remains a planned adapter.

Three.js should be an optional render surface, not a replacement for the whole
2D editor. Its [on-demand rendering guidance](https://threejs.org/manual/pages/rendering-on-demand.html)
suits a mostly idle design workspace. [OffscreenCanvas](https://threejs.org/manual/pages/offscreencanvas.html)
can move work into a worker where supported; keep a fallback and test target
browsers. [Renderer documentation](https://threejs.org/docs/pages/Renderer)
provides the basis for choosing optional WebGPU with a supported fallback.
Do not promise broad device support before measuring it.

## Ordered milestones

| Stage | Deliverable | Exit test |
| --- | --- | --- |
| A — implemented now | Visual catalog, modern static templates, shared visual styling, initial live-browser MCP | Catalog defaults export; native/browser workflows and real STDIO round trips pass |
| B — next | Shared command vocabulary; browser multi-select/resize/alignment; named brand colours and reusable components | One multi-format campaign is completed with keyboard/drag/numeric/MCP edits, then reopened with identical output |
| C — motion foundation | Persisted timeline; video/audio import, proxies, trim/split; gain/fades and basic captions | A real 30-second product story exports with verified timing and audio sync; save/reopen and cancelled render do not lose work |
| D — HyperFrames adapter | Editable animated type/layout, deterministic scene export, render queue and frame review | Repeatable key-frame/duration checks, source manifest, failure recovery and a rendered campaign preview |
| E — practical 3D | Book/product mockup scene, cover texture, lights/camera and poster export | Print and motion poster consistency, bounded memory, working fallback and idle-GPU check |
| F — hosting only when useful | Private asset storage and optional shared/render service | Access isolation, restore test, storage limits, costs and customer-data handling verified before public access |

MCP should evolve alongside B–E. Add timeline and scene tools only when their
underlying model and rendering work. Exposing arbitrary HTML/JavaScript or a
shell as an “agent design tool” would bypass the document/history guarantees.
Maintain protocol tests and test the chosen client separately. Current MCP
[requires per-request protocol context](https://modelcontextprotocol.io/specification/2026-07-28/basic);
the adapter supports that model and an older initialization flow.

## Storage decision

Measured at the start of this work:

| Location | Approximate usage / capacity |
| --- | --- |
| Repository folder, including ignored outputs | 934 MB |
| QA outputs | 593 MB |
| Release packages | 145 MB |
| Git history | 6.3 MB |
| Source / browser files / bundled assets | 972 KB / 44 KB / 5.4 MB before this change |
| Separate Rust build cache in the old project | 6.9 GB |
| Documents filesystem | 106 GB available, 53% used |
| Root filesystem | 124 GB available, 86% used |

There is no immediate storage reason to move the app. The build cache and
repeatable QA outputs dominate its source size. Nothing was deleted or migrated.
GitHub should hold source, documentation, small licensed examples and release
binaries; do not put customer projects, credentials or large render caches into
the public source repository. Use a managed media workspace with originals,
regenerable proxies and a bounded render cache. Offer cache inspection/cleanup
before deleting anything; preserve original media and finished customer files.

Video changes the numbers: uncompressed 1080×1920 RGBA at 30 fps is about
249 MB/second, or 7.46 GB for 30 seconds. Stream frames into an encoder rather
than retaining raw sequences by default. Record per-job space estimates, clean
only known disposable intermediates, and bound concurrent jobs. A sensible
first review trigger is fewer than 25 GB free or a media workspace above 50 GB,
not an automatic migration rule.

An external drive is a straightforward next home for large originals/exports
with backups. A VPS becomes useful for remote access or queued rendering, not
because source code is large. Before choosing one, benchmark CPU/GPU, upload
bandwidth, render duration, storage/egress costs and restoration. No VPS cost,
cloud deployment or external-drive change has been incurred here.

## Evidence and remaining acceptance

The repeatable checks are documented in [VALIDATION.md](VALIDATION.md): native
window interactions, browser editing and draft preservation, all-template
preflight/exports, and real STDIO MCP operations producing image/PDF/editable
proofs. These establish the current implemented scope. Personal visual approval,
a chosen coding-client integration, physical proofs, broader accessibility and
motion/3D performance remain separate acceptance work.
