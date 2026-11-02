# Canvas Studio 1.1.0 validation

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
