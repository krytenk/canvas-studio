# Film studio · 1.13

This guide describes the Stage C baseline introduced in 1.13. Stage C adds a local, persisted video/audio editing foundation to the same design
project. It is a useful cuts, sound, caption and artwork workflow. HyperFrames,
transitions, speed ramps, automatic transcription and Three.js
remain subsequent work. Version 1.14 adds [keyframes and a HyperFrames adapter](MOTION-STUDIO.md). This is not feature parity with CapCut.

## Use it

Install Python 3, FFmpeg and FFprobe; on Ubuntu: `sudo apt install python3 ffmpeg`.
Static print/design editing still runs without these tools. FFmpeg is an external
system dependency, not a bundled codec build. The tested environment is Linux,
FFmpeg 6.1.1. Native playback uses `ffplay` from that package; browser playback
uses its own MP4 player.

Open **Film** on the tool rail. Choose movie width, height, FPS and duration in
frames, and apply the settings before placing media. 900 frames at 30 FPS is 30
seconds. Canvas/FPS cannot change after clips exist; create a new document for a
new format. Duration can change if all existing clips still fit.

1. Import local footage, stills, music or dialogue. Add an asset at the playhead.
   Build a proxy for lighter review. Reimporting the identical original relinks
   missing media. Delete its clips before removing an asset from the project;
   this does not delete the cached original from disk.
2. Select a clip. Change start, source-in and duration, then **Apply timing**.
   Browser clips also move by dragging and trim by their edges. Invalid overlaps
   or out-of-source edits are rejected. Use separate tracks to overlap visuals
   or mix audio. Later visual/caption tracks composite over earlier tracks.
3. Split at the playhead. Both children retain their original sampling anchor
   and fade envelope. This matters when source FPS differs from output FPS.
   Moving preserves the envelope. Trimming rebases/clamps fades; explicitly
   setting fades starts a new envelope on that selected clip.
4. Set gain, source-audio on/off and fades. Detach source audio to edit it on a
   separate track. Tracks can be disabled; the browser can reorder them.
5. Add manual captions, position/style them, or import UTF-8 SRT. Times round to
   the nearest movie frame. SRT is the plain-text caption exchange; it does not
   carry typography. There is no speech recognizer. Text overflow and missing
   glyphs fail a render with an explanation.
6. Add the current design as an overlay. It references that page's identity and
   optional campaign layout, and resolves current fields/fonts/artwork at render
   time. It stays editable in the design workspace. A missing page fails clearly.
7. **Review this frame** produces an accurate still at the playhead. **Build
   preview** renders a maximum-640-pixel MP4, with picture, captions and mixed
   sound together. Load the completed preview in the browser and play/scrub it.
   Edits mark it stale. Desktop frame review is in the workspace; picture/sound
   playback opens in the system `ffplay` window, not a native real-time AV widget.
8. **Render MP4** snapshots the current edit and encodes full output size. Later
   edits do not alter that job. Jobs show their source revision; cancel stops
   work without changing the timeline. Save the completed movie separately.
9. **Save editable copy** embeds original media and fonts/artwork in one `.canvas`
   file. Reopening on another local installation restores the originals; rebuild
   proxies there. Local recovery references the private cache so every small
   edit does not repack a movie. Recovery alone is not a portable backup.

Review previews and final MP4s before publishing. A preview may use a compressed
proxy; final export verifies and reads originals. This is rendered playback, not
a zero-latency NLE: frame/preview requests require encoding. Multiple video
streams, arbitrary playlists/URLs and arbitrary scripts are not import formats.

## Media and resource boundaries

- Import: MP4/M4V/MOV/WebM/MKV; WAV/MP3/FLAC/OGG/M4A; PNG/JPEG/WebP.
- Up to 128 MiB per original, 256 MiB aggregate originals, 128 assets, 16 tracks
  and 200 clips. Browser/MCP file transfers are capped at 128 MiB, including a
  portable file; use native delivery for a larger supported package.
- Up to ten minutes, 24/25/30/50/60 integer FPS; even 64–3840 dimensions with at
  most 8,294,400 pixels. Variable-rate source footage is normalized to movie FPS.
  No 29.97/59.94 output preset yet. Inputs: at most two hours / 8K dimensions.
- Two jobs per process; 15-minute worker deadline; 128 MiB rendered output cap.
  Export uses H.264/yuv420p, stereo 48 kHz AAC in fast-start MP4. Caption/design
  graphics use the existing native renderer. Audio gain/fades use per-sample
  envelopes; the final limiter compensates its look-ahead delay.
- Originals are SHA-256 addressed and verified, separate from disposable proxies.
  Private data lives under `$XDG_DATA_HOME/canvas-studio/media` (normally
  `~/.local/share/canvas-studio/media`); its working directories are mode 0700.
  Media/jobs do not go into Git. No paid API or cloud service is used.
- The storage panel reports original, proxy and render-history usage. A soft
  1 GiB history guard blocks new jobs until space is cleared. Save movies first,
  then explicitly clear finished jobs for this document. Originals/edits are
  retained. There is no automatic deletion of source media or silent migration
  to a VPS/external drive. The Rust build cache and release archives remain
  separate disk usage.

## Coding-agent use

The MCP adapter now exposes 13 tools. `studio_document` includes `timeline`.
Use `studio_import(kind: "media")`, then `studio_edit` with `film_setup`,
`film_add`, `film_track`, `film_track_update`, `film_clip`, `film_split`,
`film_detach_audio`, `film_caption`, `film_design`, `film_delete` and
`film_forget_asset`. Inspect new asset/track/clip IDs after creation.

All timeline start/duration/split/fade values are integer **frames**; source-in
is **microseconds**; gain is dB; placement/caption size uses movie pixels.
Caption font IDs follow the existing document font list. Each atomic batch is
one undo step; dry runs and stale-document/revision guards still apply.

`studio_media_render` queues `preview`, `export`, `frame` (requires frame), or
`proxy` (requires asset). `studio_media_job` inspects progress, requests cancel,
or saves completed output to the scoped workspace without overwriting by
default. Use a fresh current revision when invoking these tools; a completed
job retains its independent source revision. No arbitrary FFmpeg command, shell,
URL, HTML or JavaScript execution is exposed as an agent tool.

## Acceptance and reproducible demo

`scripts/media_smoke.py` generates its own offline fixtures, drives the real
STDIO MCP/HTTP engine and checks decoded frames around clip boundaries, source
in, a split within an audio fade, an audio synchronization impulse, exact movie
frame count, sample rate/duration, captions, frame review, proxy creation,
wrong-document/workspace/no-overwrite guards, cancelled output and a portable
cold-cache restore. It deliberately corrupts a test original and verifies repair
from the portable project. A 24 fps moving luminance fixture must produce the
same decoded frame sequence after splitting on a 30 fps timeline. Optional
`--chrome chromium` exercises an isolated CI Film interface.

`scripts/make_film_demo.py` downloads the official Sintel trailer and uses the
MCP to assemble a **30-second, 1280×720, 30 FPS** review film with source dialogue,
music, manual narrative captions and a native editable CTA. This is an editing
example, not a new original film or an advertisement for the filmmaker. Outputs
and the portable master are in ignored `qa/film-demo/`; they are kept out of Git.
The script does not publish anything.

Footage and original sound: **© copyright Blender Foundation | durian.blender.org**.
Sintel is [CC BY 3.0](https://durian.blender.org/sharing/);
[source trailer](https://download.blender.org/durian/trailer/sintel_trailer-720p.mp4),
[license text](https://creativecommons.org/licenses/by/3.0/).
The demonstration trims/reorders clips, mixes fades, adds captions and CTA art;
it implies no endorsement. Logos/trademarks are not independently relicensed.
Do not replace this attribution with a claim that Canvas Studio generated the
source film.

The implementation follows FFmpeg's documented
[FPS, trim, timestamp and audio filter behavior](https://ffmpeg.org/ffmpeg-filters.html).
Tests establish these supported workflows, not every codec, platform or broadcast
conformance requirement. Final render review and broader accessibility/performance
acceptance remain necessary as the editor grows.
