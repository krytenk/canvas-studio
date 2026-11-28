# Starting workflows

Versions 1.20–1.21 add the same four starting choices to desktop and browser. The
task names and explanations come from `src/workflows.rs`; the browser reads
them from the guarded `/api/workflows` endpoint.

| Starting choice | Desktop | Browser |
| --- | --- | --- |
| Design a product | Opens the editable template library | Opens the editable template library |
| Create a campaign | Opens campaign setup, layouts and brand colours | Opens campaign setup, layouts and brand colours |
| Make a video | Opens Video studio | Opens Video studio |
| Bring in research | Opens personal Research | Opens personal Research |

The initial publishing profile keeps AI generation out of the starting workflow.

Choosing a task is navigation. It does not replace the current document, start
GPU work or discard edits. Starting a new campaign or applying a replacement
template remains an explicit action with the existing unsaved-work handling.
Home includes **Continue current project**, which resumes the previous workspace,
and **Open design**.

New desktop sessions open Home. Explicitly opened projects enter the editor.
The browser opens Home for a single blank document; existing non-empty projects
continue in the editor. Home and the browser session status show the current
engine version. Unapplied form values remain available after a visit to Home;
apply or discard them before exporting.

The remaining authoring differences are explained under **More tools and
formats**: book setup, puzzle creation and CSV production remain desktop tools.
Portable projects preserve these structures when opened and personalised in
the browser. This milestone aligns entry tasks and document preservation; it
does not claim every advanced native control is available in the browser.

## Validation

- Shared-engine checks cover task navigation with a dirty document and existing
  content, plus blank-document detection that preserves notes and video work.
- `scripts/workflow_smoke.py` exercises first use, all four tasks, unapplied
  text preservation, editable delivery, reopening from Home and a 700px viewport.
- Existing browser checks cover text/font/photo edits, undo, PDF output,
  template insertion, session guards and recovery.
- Native generation and personal Research navigation retain Home proofs at 1280×900 and 1024×680.
  Existing native smoke checks open and save real portable projects.
