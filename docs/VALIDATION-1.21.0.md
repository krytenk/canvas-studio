# Canvas Studio 1.21.0 acceptance

## Scope

A shared task-oriented Home starts desktop and browser work: product design,
campaigns, video editing and personal Research. Continue returns to the previous
workspace and preserves the current project. The publishing profile is the default
for both launchers and direct binary use; image/video generation is hidden and
rejected before worker submission unless the owner explicitly enables the optional
personal profile. No paid inference service is configured by this release.

Research bridges saved outputs from the installed personal Reference Desk app.
It supports explicit transcript snapshots, cited page notes, SRT caption insertion,
audio/video import and portable project delivery. It does not add remote downloading
or transcription to Canvas Studio.

## Evidence

Code revision: `044cafe317a9a9d2273663dd91132f0fc7099f71`.
[Final Linux acceptance run](https://github.com/krytenk/canvas-studio/actions/runs/46110689870).
The main integration also passed [the preceding full suite](https://github.com/krytenk/canvas-studio/actions/runs/46110192924).

| Check | Result |
| --- | --- |
| Release engine suite | 108 passed |
| Provider-independent Python checks | 36 passed; no GPU submission |
| Final focused Research suite | 9 passed |
| Embedded resources | All 10 fonts available outside a separate asset folder |
| Browser Home | Four tasks; draft/project preservation; Home/Continue resumes Video, Campaign and Research; editable delivery/reopening; 700 px layout; no page errors |
| Browser Research | Changed-preview rejection, source citations, undo/redo, script-like text rendered literally, caption timing and atomic overlap rejection, audio/video import, oversized-file guidance |
| Portable delivery | Version-6 package with five sources and two media files reopened using a fresh native data directory; previous 1.19.1 reader explicitly refuses the new format |
| Native interaction | Research source selected, passage added through UI, Ctrl+S saved, portable notes inspected; screenshots reviewed at 1280×900 and 1024×680 |
| Real rendered output | Caption frame rendered through Film and checked for visible content |
| Regression checks | Browser editing, native save/undo/pages/export, media timing/audio/cancellation/cold restore |
| Local live connection | Installed Reference Desk launched from the browser and its health endpoint returned the expected app identity; configured saved-output library listed |
| Privacy checks | Generated QA originals unchanged, no external browser requests during Research acceptance, generation API returns 403 in publishing profile |

UI acceptance uses isolated generated fixtures and separate application data.
Personal filenames and media are excluded from repository proofs and release assets.
Local proof images and reports live under ignored `qa/workflows`, `qa/research` and
`qa/native-research`. CI publishes generated fixture proofs as workflow artifacts.

The final Research browser run caught a test assertion that read the notes before
the refreshed DOM finished rendering. It now waits for the visible citation, rather
than only waiting for the underlying state. Both final browser workflows pass.

## Boundaries

- Native and browser share task names, document engine and Research operations;
  they retain different interfaces and their existing advanced-tool differences.
- Research import limits: 1 MB per TXT/SRT, 128 MiB per media file, 256 MiB total
  portable film media, 100 sources and the existing 200-clip/ten-minute timeline.
  Oversized saved files show guidance before import. Use a shorter working copy.
- Research envelope 6 requires a compatible reader. Original Reference Desk
  files, notes already copied and timeline clips remain when a source card is removed.
- No VPS deployment, public multi-user authentication, advertising integration,
  subscription billing, RunPod endpoint or funded generation allowance is included.
- The VPS capacity discussion is an initial assessment of the supplied hardware,
  not a host benchmark. Inspect available capacity, operating-system compatibility,
  dependencies and backup arrangements before moving personal work.

See [Research](PERSONAL-RESEARCH.md), [starting workflows](STARTING-WORKFLOWS.md)
and [hosting options](HOSTING-OPTIONS.md).
