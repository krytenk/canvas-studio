# Changes

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
