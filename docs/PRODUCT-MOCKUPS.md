# Product studio — Three.js mockups (1.15)

Use editable page artwork to present a hardback book, product box or stationery
card stack. These are local geometric product scenes, without stock-image
subscriptions or an external generation API. The Three.js 0.187.0 source and MIT
license ship inside the application. The preview has no continuous animation
loop: it draws only when a scene loads or the reveal slider moves.

## Work from a real cover

In the browser, choose **Mockups**, then **+ Book**, **+ Box** or **+ Cards**.
The current page becomes the front artwork. Choose an optional back page, adjust
the proportions, camera turn/elevation/zoom, roll, lighting, roughness, side
colour and backdrop, then **Apply scene**. The width initially follows the
cover page aspect ratio. Dimensions are relative proportions, not printer trim
measurements. Changing them can stretch the complete cover texture; crop/layout
the editable page before rendering if necessary. Book spines are a plain colour;
separate spine artwork, curved pages, imported meshes and foil are future work.

The reveal slider seeks a fixed six-second eased turn. It never changes the
editable document. The Three.js renderer is shared by preview and output, with
fixed lighting and no random simulation. The live preview requires browser
WebGL2. Without it, the editor still offers local renders and completed PNG
previews; the main editor and existing poster images remain usable.

**Render listing image** queues a PNG at time zero. **Render 6s reveal** queues a
180-frame, 30fps H.264 MP4. Output width/height are even values from 256–2400.
Worker resources are isolated by their embedded content, so an older open build
cannot overwrite a newer build’s renderer during a job.
Rendering uses the pinned local HyperFrames/Chromium installation described in
[MOTION-STUDIO.md](MOTION-STUDIO.md); install it with
`python3 scripts/setup_motion.py`. Software WebGL is supported, so no physical GPU
or cloud account is required. Both operations are cancellable snapshot jobs,
share the existing two-job limit and preserve the source document/revision.

Download PNG/MP4 or an **Editable source ZIP**, containing the Three.js scene,
local renderer/library/license, cover PNGs, native `.canvas` master, pinned npm
lock file and a manifest with SHA-256 hashes. No session token or absolute source
path is written into that ZIP. For a standalone preview, extract it, run
`npm ci --ignore-scripts`, then `npm run preview`. `npm run render` reproduces the
six-second reveal, including for a source ZIP originally made for a still.

## Desktop, print and Film

In the native editor, open **Tools → Product studio · 3D**, or **Book → 3D product /
book mockups**. Art direction, render/cancel, poster review, PNG/MP4/source saving
and placement use the same engine and commands. Native live WebGL embedding is
not implemented: native review uses the generated poster image.

To place a listing image, select a different destination page and choose
**Place on current page**. It becomes an ordinary portable image layer, usable
in checked PDF/PNG production, with the original scene remaining editable.
Placement on its own cover/back page is refused to avoid recursive artwork.
A scene/cover change blocks placement of an older render. Unrelated changes,
such as the document title, do not. Already placed posters are intentionally
independent snapshots; delete/replace them explicitly after changing artwork.

**Add reveal to Film** imports the completed MP4 into the current timeline at
frame zero; native and agent commands can supply another start frame. Set a
suitable movie size/duration first (default Film settings remain valid).
The imported clip is a rendered video snapshot: trim, split, captions, sound and
existing motion controls work as usual. Subsequent scene changes require a new
reveal, rather than altering previously rendered Film clips silently. Three.js
geometry is not yet edited inside individual Film clips.

Portable projects with scenes require envelope v5 / `product-mockups-v1`.
Older app versions reject these documents instead of silently losing scenes.
Scenes and placed PNGs survive save/open; portable Film media embeds as before.
There is a maximum of 24 scenes per project. Render history shares the existing
1 GiB limit and can be cleared in Film after saving desired outputs.

## Agent control

The STDIO adapter now exposes 14 tools. Inspect `studio_document.mockups`, then
use shared `studio_edit` actions:

- `mockup_create(page, kind)` with `book`, `box` or `cards`.
- `mockup_update(id, ...)`: name, kind, page, back_page (stable page ID or null),
  width/height/depth, yaw/tilt/elevation/zoom/sweep, background, cover_color,
  light (`warm`/`neutral`), roughness, output_width/output_height.
- `mockup_delete(id)` removes a scene, preserving placed poster snapshots.
- `mockup_place(id, job, page)` places a matching completed poster on another page.
- `mockup_film(id, job, start)` imports a matching completed reveal into Film.

`studio_mockup_render(document_id, expected_revision, id, movie=false)` queues a
job. `studio_media_job` handles status/cancel/save/source, with `.png`, `.mp4` or
`.zip` output paths restricted to the configured workspace. Document guards,
atomic batch rollback, dry-run and undo apply to shared editing actions. Covers
are resolved through native production/typography and must pass error preflight.
No arbitrary URLs, scripts, meshes or shader code are accepted from agents.

## Acceptance

`cargo test --release --locked` includes typed validation, limits, missing cover
and portable round-trip cases. `scripts/mockup_smoke.py` uses the actual pinned
renderer through STDIO MCP: books/boxes/cards, pixel-identical repeated PNGs,
poster vs decoded MP4 frame, 180-frame reveal, source/license hashes, stale
artwork/camera and self-cover rejection, cancellation, PDF poster placement,
Film import and a cold portable load with independent storage. The CI browser
stage checks out-of-order seeks, idle draw count, art-direction editing and the
WebGL-disabled fallback. Native window regressions and the full earlier
browser/campaign/media/motion suites remain required.
