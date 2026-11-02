# Changes

## 1.1.1 — production workflow fixes

- All seven included templates use exportable defaults. Badge backgrounds remain editable shapes; unsupported demo charts become labelled editable bar graphics. General chart editing still has the documented export restrictions.
- Group and multiple-selection duplication/copy/paste preserve all members with fresh element and group identities. Group handles scale proportionally and rotate around the selection centre; locked members prevent group transforms.
- Layer ordering and lock controls act on the complete selection.
- Template insertion fits every source page proportionally into the destination document, rather than copying only the first page at incompatible dimensions.
- SVG and HTML exports now return explicit errors for unsupported content. Callers validate before writing the destination file.
- Image enlargement and patch-blending controls describe the local algorithms they actually run.
- Added end-to-end template save/reopen/export checks, independent Poppler PDF renders and group/selection regression coverage.

The original 1.1.0 release remains available as a fallback. Native desktop operation and physical printing still require manual validation.
