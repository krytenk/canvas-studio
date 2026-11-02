# Third-party notices

Canvas Studio embeds the unmodified Noto Sans, Noto Serif and Noto Sans Mono
fonts. Their copyright notices and SIL Open Font License 1.1 are preserved in
[assets/fonts/LICENSE-OFL.txt](assets/fonts/LICENSE-OFL.txt). The fonts are also
embedded in relevant document exports.

Rust dependencies are recorded in Cargo.toml and Cargo.lock. Each dependency
retains its own licence and copyright notices. Dependency source and licence
files can be obtained through Cargo's package registry.

Optional motion rendering installs HyperFrames 0.8.206 and its locked npm
dependencies through the supplied setup helper. They retain their own package
licenses/notices; their code and Chromium/Node/FFmpeg are not embedded in the
Canvas Studio executable or the Linux archive. Generated source kits carry the
Noto font license when native SVG artwork embeds these fonts.
