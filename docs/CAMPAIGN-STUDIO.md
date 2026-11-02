# Campaign studio — 1.12

A campaign is one portable project with independently composed layouts and a
shared library of copy fields, named colours, fonts and reusable groups.
The starter includes a 1080×1350 Instagram post, 1080×1920 story, 1280×720
YouTube thumbnail and A4 print design. These are editable static layouts.

## Use it

In the browser, choose **Campaign** in the left rail, then **New four-format
campaign**. Save an independent copy of your current design before replacing it;
Undo remains available. On desktop, expand **Campaign layouts** in Designs.

Choose a layout in Campaign to switch compositions. It keeps its own dimensions,
pages, guides, book settings and artwork. **Fields** changes shared names,
headlines, descriptions and calls to action across the campaign. Apply each form
explicitly. Long replacement copy can overflow; production exports report it.

**Linked brand colours** change named colours across all layouts and saved
reusable groups. Select layers and use **Link colour** in Edit to attach their
primary colour. **Unlink colour** freezes the current appearance for a local
choice. Text/shape colour controls are disabled while their corresponding colour
is linked. MCP rejects a local colour override until that link is removed.
Background colour links are also available through the shared command API.
On desktop, ordinary colour pickers/palettes create a local override and detach
the affected link; named-colour controls keep shared links intact.

Use **Save current as another layout** to create an independent copy, including
fresh page/layer/group identities and retained copy/colour links. Additional
artboard sizing is available through the desktop format/custom-size controls.
The four starter formats are separately composed, not proportionally stretched
versions of one page. Layout changes are captured when switching and saving.

## Select and arrange

- Click a layer, or Shift-click to add/remove layers or complete groups.
  Layers provides the same selection when artwork overlaps.
- Drag selected artwork to move it; the native SVG artwork follows the pointer.
- Drag any corner handle to scale the selection proportionally, including type,
  paragraph measurements and outlines. Numeric selection width does the same.
  A single layer's ordinary geometry form resizes its frame independently.
- Use alignment or even-spacing controls. Groups act as single units. One unit
  aligns to the page; multiple units align to their combined bounds. Distribution
  requires three independent units. Locked members reject the entire operation.
- Arrow keys nudge one layout pixel; Shift-arrow moves ten. Ctrl/Cmd+A selects
  visible layers and their groups, Ctrl/Cmd+D duplicates, Ctrl/Cmd+G groups,
  Ctrl/Cmd+Shift+G ungroups, Delete removes and Escape clears selection. Text
  fields retain their ordinary typing shortcuts.

Proportional scaling takes ownership of selected text's type measurements:
linked typography styles are materialized locally before scaling. Shared copy
and named-colour links remain. Undo restores the prior links and geometry.
Unapplied forms block layout edits and switching; apply or discard them first.

## Reusable groups

Select artwork, open Campaign/Brand, choose a simple library key and save the
selection. Insert it into any campaign layout. Images and fonts remain portable;
colour/field links stay shared and each inserted copy is grouped with fresh IDs.
Typography is materialized when captured so instances can scale reliably.

The library belongs to this project, with up to 100 saved groups. Inserted copies
have independent geometry; editing one copy's shapes or wording does not rewrite
the saved definition or other copies. Shared fields/colours still update all
bound copies. Live component-definition overrides and a cross-project component
manager remain future work.

## Deliver and reopen

**Save editable copy** preserves every layout and its shared library in one
`.canvas`. Individual PDF/PNG buttons export the current layout. Browser
**Download campaign bundle** packages the editable project, each layout's PDF,
96-DPI PNG pages and `manifest.json`. The manifest records dimensions, page
counts, output folders and the source revision. PDF is the print deliverable;
96-DPI PNGs retain intended social pixel dimensions and serve as print proofs.

The bundle supports up to 100 pages and 128 MiB of source output, and rejects
changes made during export instead of mixing revisions. It reads inactive
layouts without changing the live selection/layout. MCP offers the same bundle
with `studio_export(format:"zip")`, and can preview/export a named `layout`
without switching the active document.

Projects using campaign, brand-link or reusable-group features use a version-2
portable envelope with required-feature metadata. Older builds reject it. Existing
ordinary projects remain readable and keep their format. If such a project is
saved under an old `.canvas.json` name, it still uses the protected portable
encoding; loading detects the header, not the suffix. Prefer `.canvas` for clear
file naming. Earlier app builds cannot edit these new-feature projects safely.

The supported limits are 16 layouts, 100 named colours, 100 saved groups and
500 layers per group/selection command. No video timeline or 3D editing is
implemented by this milestone. See the [roadmap](ROADMAP.md) for what follows.

## Acceptance evidence

Rust checks cover independently composed layouts, shared copy/colours, pixel-
identical portable round trips, inactive image/font resources, grouping/scaling,
locked selections and materialized typography. `scripts/campaign_smoke.py` drives
real MCP previews/exports and validates ZIP checksums, manifests, PNG dimensions
and selectable PDF text. Its isolated browser workflow covers multi-select,
group/nudge/corner scaling/alignment, component insertion, shared copy across
formats, bundle download, reopening and a 700 px layout. CI repeats these along
with the existing native-window, browser and MCP workflows.
