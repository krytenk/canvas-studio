# Canvas Studio validation

## 1.2.0 local validation

Verified on 1 November 2026 on Linux x86-64 / Ubuntu 24.04:

| Check | Result |
| --- | --- |
| Formatting | Passed |
| Release tests, offline and locked | 38 passed; 0 failed |
| Release build | Passed |
| Seven built-in templates, eight pages total | Edited title, saved/reopened, preflight at 300 DPI, PDF/PNG/SVG/HTML export at proof settings |
| Independent PDF verification | Poppler text extraction and renders succeeded; eight rendered pages visually reviewed |
| Group operations | Fresh copy identities, proportional scaling, rotation, protected original, undo/redo and save/reopen checked |
| Different-size template insertion | All pages inserted proportionally, unique identities, source preserved and undo checked |
| SVG/HTML unsupported effects | Explicit errors; no silent effect omission through the checked export APIs |
| Personal masters | Image assets and previews saved; independent copy identities and master preservation checked |
| Moved template library | Opened and rendered successfully in a fresh child process |
| Invalid metadata / failed template saves | Healthy entries retained, errors reported and no partial entries left behind |
| Inspector geometry | Inch/mm conversion and aspect ratio checked; non-finite edits rejected without partial changes |
| Typography, layers and template panels | Headless frames rendered without dirtying or mutating the document |

Template proof PDFs use 96-DPI raster fallback settings to keep regression checks
small, while preflight runs at 300 DPI. These are layout proofs, not a claim that
each low-resolution sample image is suitable for commercial print. The suite also
retains physical-size, font, Unicode, image and independent-render checks from the
previous release.

Manual native desktop interaction and physical printer acceptance are still open.
The new font controls use the bundled families; imported-font and rich-text support
are not part of 1.2.0. GitHub workflow status is reported separately from these
local results.

## 1.1.0 original release validation

Verified on 1 November 2026 with Rust 1.99.1 on Linux x86-64 / Ubuntu 24.04.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --release --offline --locked` | 28 passed; 0 failed |
| `cargo build --release --offline --locked` | Passed |
| Launcher shell syntax | Passed |
| Executable copied into an unrelated temporary directory | Version and self-check passed |
| All ten embedded font faces | Loaded without an external font directory |
| CLI example layout to PDF | One A4 page; text extracted independently with Poppler |
| Independent render of the example PDF | Visually checked; no clipped or overlapping content |
| Moved layout with relative image paths | Imported, saved and reopened successfully |
| Deleted referenced image | Correctly rejected |
| Release source and executable content review | No private catalogues, workstation paths or account credentials included in the source; no private product identifiers found in the executable |

The test suite also covers failed-save safety, cold-process image loading,
per-document recovery, undo branches, same-frame editing/save behaviour,
fractional physical sizes, export allocation limits, Unicode PDF text,
rotation, tables, gradients and photo adjustments.

The native file dialogs, graphics-driver/window-manager behaviour and physical
printers were not manually operated for this release. Windows and macOS binary
builds were not tested. Unsupported print features remain subject to export
preflight; see the README for the current feature limits.

![Example layout exported by Canvas Studio and independently rendered](example-layout.png)
