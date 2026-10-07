# Verification — 2026-10-06

Local macOS checks: `cargo fmt --check`, `cargo test --locked` (24 tests), `cargo clippy --all-targets --locked -- -D warnings`, release build and app signature verification.

The native verification app used only the three original demo notes. Verified sidebar-only toggling while the inspector stays visible, Focus hiding/restoring the previous layout, inline Canvas/Pin controls, existing-pin reveal, card double-click opening a note, context-menu unpin preserving the note, and re-pinning. Stored board state survives reopening. Model tests cover pin uniqueness/nonoverlap, board persistence/conflicts, and panel restoration. Automated native pointer dragging was inconclusive in this run; it is not counted as a passed end-to-end check.

The release also replaces missing fountain-font checkbox glyphs with drawn shapes. Rendered checklists remain read-only; edit Markdown to toggle completion.

This is an early prototype, not a claim of comprehensive compatibility or accessibility certification. See README for known limitations.
