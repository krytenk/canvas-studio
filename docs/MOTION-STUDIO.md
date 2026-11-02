# Motion studio · 1.14

Stage D connects the existing film timeline to **HyperFrames 0.8.206**, running
locally. Animate text/caption clips, stills, design pages and footage using saved
position, scale, rotation and opacity keys. Print layouts remain native and editable.
This stage does not add Three.js, automatic transcription or arbitrary imported HTML.

## Start

Static design and the existing cuts/audio workflow need no Node installation.
Motion requires Node 22+, FFmpeg/FFprobe and the optional pinned local renderer:

```bash
python3 scripts/setup_motion.py
```

The helper downloads npm dependencies and HyperFrames' pinned Chromium. It installs
under your application-data directory, not into Git, and creates no subscription
or cloud account. For a development checkout use `--local`, which installs into
`tools/hyperframes/node_modules`. The Linux release includes the setup script and
npm lockfile. Network access is needed for setup; composition assets and rendering
are local. Optional telemetry/update checks are disabled for app-owned jobs.

The browser Film library reports whether the motion tools are installed. The
`CANVAS_STUDIO_HYPERFRAMES` environment variable can point to a trusted installation's
`hyperframes/bin/hyperframes.mjs`; the version must match the pin. Missing tools
produce an explanation while leaving the edit intact. The app does not silently
install software when opening a project.

## Make something move

Select a clip in **Film**, then open **Motion** (desktop: **Motion & keyframes**).
Choose Rise, Fade, Slide, Punch or Slow zoom, or build custom keys in the browser.
Edit frame, X/Y offsets, scale, rotation, opacity and the ease to the next key.
Apply explicitly, then **Review this frame** or **Build preview**. Native controls
edit the same keys; desktop AV review still opens in ffplay.

Keys use frames in the original animation. Position offsets are movie pixels;
scale 1 is unchanged, rotation is clockwise degrees and opacity ranges from 0–1.
The transform pivots around the clip rectangle's center. Linear, cubic ease-in,
ease-out/ease-in-out and a hold are supported. Before the first/after the last key,
the endpoint holds. Clear removes motion without changing static placement.

Splitting retains the original animation clock and keys on both children. Moving
moves the clock with the clip. Trimming retains the animation's timing rather than
restarting it. **Keep timing across splits** edits that inherited clock. Uncheck it
or apply a preset to restart the selected clip's animation. Detaching sound keeps
motion on the picture and gives the audio its own track.

Any clip with motion routes frame/preview/export jobs through HyperFrames. Other
films continue to use the established FFmpeg pipeline. Renders snapshot the edit;
source revisions and stale-preview warnings retain their existing meaning. There
is no live, zero-latency NLE preview: build the composed movie to review picture and
sound together. Frame review prepares only visible clips and seeks video through
its original sampling anchor.

## Source delivery and reproducibility

**Export motion source** creates a ZIP. Completed motion preview/export jobs also
have a **Motion source ZIP** download (native: Save motion source ZIP). Frame-only
jobs produce a still, not an incomplete full-film source archive.

The ZIP contains:

- `editable-master.canvas`: the complete native project, original media and fonts.
- `index.html` and one readable `compositions/clip-*.html` per visual clip.
- Native SVG artwork/text where supported, otherwise native PNG rendering; local
  normalized video clips and the native sample-accurate WAV mix.
- Pinned `package.json` / `package-lock.json`; use `npm ci`, then `npm run preview`,
  `npm run check` or `npm run render` to inspect/edit/render outside Canvas Studio.
- `manifest.json`: original clip timing/keys, source revision, output dimensions/FPS,
  SHA-256 hashes, renderer version, Node/Chromium/FFmpeg versions and render settings.
- Noto font license, where the native SVG artwork embeds these bundled fonts.

Caption words/layout remain editable in the native master. SVG glyphs and HTML
scene transforms are editable source; this is not a second HTML text/layout editor
inside Canvas Studio. External HTML edits do not import back into the native model.
A source exported from a preview records that preview's reduced resolution; use
Export motion source or a final export job for full-resolution source.

The adapter pre-normalizes video at the movie FPS from the original sampling
anchor, including preserved split offsets. It reuses the existing audio gain,
fade and latency-compensated limiter graph to make one WAV mix. HyperFrames drives
media and the pure `hf-seek` pose sampler at explicit timestamps. No timers, random
motion, live fetches or CDN assets determine a frame. Software browser rendering
and one capture worker are explicit. Lint gates every bundle; preview/final movie
jobs also run HyperFrames' runtime/layout check before rendering.

## Boundaries

Up to 64 ordered unique keys per clip. Existing film, original-media and output
limits still apply. Motion source ZIPs and MP4s are capped at 128 MiB. Bundles can
be substantially larger than a movie because they retain the originals, normalized
media, fonts and WAV mix. Rendering is bounded to two app jobs and 15 minutes per
worker operation. Large/full-resolution work is not a certified performance case.
Use the existing storage panel to save deliverables and clear finished render jobs.

Version-4 portable envelopes declare `motion-keyframes-v1`; older builds reject
these projects rather than silently discarding animations. Versions 1–3 still
open, and saving without any motion keeps the earlier applicable envelope.

MCP remains **13 tools**: `studio_edit` gains `film_motion(id, preset)` or
`film_motion(id, keys, preserve_timing)`. `studio_media_render` gains mode `source`;
`studio_media_job(operation: source, path: '*.zip')` saves a completed motion job's
source. Existing expected-revision, atomic batch, dry-run, workspace and no-overwrite
rules apply. No arbitrary shell, JavaScript or HTML execution is exposed as a tool.

## Acceptance

`scripts/motion_smoke.py` generates local marker/audio fixtures and drives real MCP
imports, key edits, exact-frame review, repeated exports, splitting and source ZIP
validation. It checks identical decoded frames across repeats and splits, a known
synchronization cue, portable version-4 flags, asset hashes, invalid-key rollback
and cancellation. Its optional isolated CI browser edits a preset/custom-key form.
Existing static/campaign/film/native workflows remain required checks.

`scripts/make_motion_demo.py` animates the prior credited Sintel editing demo using
real footage, native SVG caption entrances, video transforms and the editable CTA.
It retains **© copyright Blender Foundation | durian.blender.org**,
[CC BY 3.0 attribution](https://durian.blender.org/sharing/), and implies no filmmaker
endorsement. The source footage was not generated by Canvas Studio.

Reference contracts: [HyperFrames CLI](https://hyperframes.app/docs/5-packages/cli)
and [runtime/core](https://hyperframes.app/docs/5-packages/core). The app deliberately
pins the tested release instead of following npm latest automatically. Practical
Three.js product/mockup scenes remain the next milestone.
