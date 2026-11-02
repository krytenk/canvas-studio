# Browser editing in Canvas Studio 1.9

Canvas Studio serves a local browser interface using the same Rust document,
font, layout, preview and export engine as the desktop application. It needs no
account, subscription, Node installation or paid API. The distributed host is
Linux x86-64; Chrome and Chromium are covered by the current browser checks.

## Start and open

Run `./run-browser.sh`, or:

```bash
./canvas-studio --serve-browser
./canvas-studio --serve-browser --open /path/to/design.canvas
```

Open the private session URL printed in the terminal. Keep the server running;
Ctrl+C stops it. The default address is 127.0.0.1:8787. Use `--port 8788` for a
separate instance, or `--port 0` for an automatically assigned port. Each port
has its own recovery document. A restarted server prints a new session link.

The server binds only to loopback. This is an editing tool on the host computer,
not a public customer-editing link. Public hosting would require a separate
multi-user deployment and access design. Do not expose this server through a
port-forward or tunnel as though it were that hosted service.

## What you can edit

- Open a portable `.canvas` file with its images and imported fonts.
- Personalise linked text/photo fields across the collection.
- Edit ordinary text, font faces, point sizes, colours, leading, tracking and
  paragraph spacing/insets/indentation. Character selections support emphasis.
- Import supported static TrueType faces using the same embedding checks as desktop.
- Add text/photos, replace photos, move/resize/rotate layers, duplicate/delete
  layers and lock/unlock them. Selected layers can be dragged on the page.
- Select, rename, add, reorder and delete pages. Inherited book masters and
  numbering continue to render through the native engine.
- Undo/redo applied edits, run print checks and download editable files, PDF
  output or a 150-DPI PNG proof of the current page.

Book master editing, shared-style definitions, puzzle generation/rebuilding and
CSV production remain desktop controls. Their data is preserved in portable
browser round trips. Linked text frames retain their shared style; change its
definition in desktop Production rather than overriding it accidentally.
Generated puzzle changes still block checked delivery until rebuilt or detached
in desktop Puzzles.

## Applying, saving and recovery

Use the Apply button for field, text/style and geometry forms. Unapplied form
values survive an in-app refresh. Downloads are blocked while there are unapplied
form edits; apply them or choose **Discard unapplied edits** first. Undo history
covers applied document edits; normal text-area editing can use the browser's
text undo. Downloading an editable copy is also available with Ctrl/Cmd+S.

Applied edits are saved atomically to a recovery `.canvas` document under the
application's private browser data directory. The session resumes that port's
recovery when started without `--open`. Recovery failure leaves the last healthy
state in place and reports an error. Up to 20 undo states are retained in memory.
Explicitly download a `.canvas` copy to keep, back up or share a deliverable.
Unapplied form drafts are held in the current page, not the recovery file.

Multiple tabs share one server document. Revision checks reject stale writes
instead of overwriting a newer edit. Refresh the stale tab to see the current
state; its unapplied form inputs remain available for review.

## Limits and checks

Uploads are limited to 128 MiB for a project, 25 MB for photos and 20 MB per
imported font, with the native image-dimension and font checks. Browser documents
must contain 1–1000 pages with positive dimensions. Image uploads are normalised
to PNG; animation is not an editing format. Font weight/style is the actual
imported face, not a simulated variant.

Required fields, unsupported content, modified puzzle keys and overflowing text
block production downloads. Print checks also report resolution/safety warnings.
The browser preview uses native SVG output with embedded font/image data; print
guides remain editing-only. Review a physical printer proof before production.

The local service uses a random session token, host/origin checks, bounded worker
count and upload limits. Unauthorized requests are rejected before upload bodies
are read. It does not offer arbitrary server filesystem or shell access.

## Repeating the UI checks

After the Rust tests have generated the generic QA fixtures:

```bash
python3 -m venv .ui-venv
.ui-venv/bin/pip install -r scripts/ui-requirements.txt
.ui-venv/bin/python -m playwright install --with-deps chromium
.ui-venv/bin/python scripts/browser_smoke.py --binary ./canvas-studio --chrome chromium
.ui-venv/bin/python scripts/native_smoke.py --binary ./canvas-studio
```

The native check additionally needs Xvfb, xwininfo, libX11 and libXtst. Both drivers
use isolated data/profiles and copied fixtures. They do not read a personal
browser profile or modify existing user projects. GitHub CI runs these workflows
and retains their screenshots/results as artifacts.
