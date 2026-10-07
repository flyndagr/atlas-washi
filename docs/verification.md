# Verification — 2026-10-06

Local macOS checks: `cargo fmt --check`, `cargo test --locked` (24 tests), `cargo clippy --all-targets --locked -- -D warnings`, release build and app signature verification.

The native verification app used only the three original demo notes. Verified sidebar-only toggling while the inspector stays visible, Focus hiding/restoring the previous layout, inline Canvas/Pin controls, existing-pin reveal, card double-click opening a note, context-menu unpin preserving the note, and re-pinning. Stored board state survives reopening. Model tests cover pin uniqueness/nonoverlap, board persistence/conflicts, and panel restoration. Automated native pointer dragging was inconclusive in this run; it is not counted as a passed end-to-end check.

The release also replaces missing fountain-font checkbox glyphs with drawn shapes. Rendered checklists remain read-only; edit Markdown to toggle completion.

This is an early prototype, not a claim of comprehensive compatibility or accessibility certification. See README for known limitations.

## 2026-10-07 — zoom and automatic refresh

28 tests pass, including cursor-anchored zoom, zoom limits and persistence, compatibility with older layouts, refresh after same-length edits, additions/deletions/renames, selection retention by path, invalid-file recovery, and deferral of dirty drafts with existing save-conflict protection. Formatting and Clippy pass. Automatic refresh polls Markdown files every two seconds; it does not synchronize external changes to canvas or appearance settings.

Release build succeeded. The local toolchain emitted a non-fatal debug-symbol stripping warning about a missing libLLVM.dylib; the executable launched successfully. A native screenshot of a disposable two-note notebook at 75% zoom was inspected: controls, card text, and the connection arrow render correctly. Pointer gestures and live refresh in the running UI were not exercised; these remain manual verification items.

## 2026-10-07 — Things to remember prototype

31 tests pass; formatting, Clippy, release build and app signature verification pass. New tests cover Gregorian date validation and leap years, dated/undated/closed eligibility, Unicode captured context, completion/reopen/reschedule, preservation of extra prose, export/reopen without hidden state, source link rewrites on rename, missing sources, malformed metadata and refusal to overwrite external edits.

The actual native review window was rendered and visually inspected using synthetic notes. An initial fade made the window translucent; the final view uses an opaque paper frame and no entry fade. Screenshot: local ignored `design-review/remember/review.png`.

Native UI automation stalled and selected the existing Atlas Vault window rather than the disposable test window; no actions were sent to that window. Consequently, mouse/keyboard capture, selection-to-context transfer, source navigation, rescheduling in the UI, and minimum-width layout are not claimed as end-to-end verified. The underlying lifecycle is covered by tests. No personal notebook files were used as fixtures.

This is an in-app review only; no background alerts, recurrence, automatic inference or calendar/contacts integration. Person is a user-entered label. Local date comes from macOS and refreshes every 30 seconds. Source context is a captured snapshot, not a live quote. The research plan includes proposed pilot gates, not observed user results.

## 2026-10-07 — cohesive controls and installation

Shared primary/secondary actions, active tabs and explicit keyboard focus unify capture and review. Capture fields are labelled for accessibility. New note uses the same action buttons. Standard widget outlines are removed in favor of paper fills; form fields retain a distinct frame. Completion and setting an item aside are separated. Rescheduling is inline; capture opens the matching review tab. Standard-size native render inspected. The same 31 tests pass; formatting, Clippy and release signature verification pass. Native interaction/minimum-width checks remain unverified, as above.

## 2026-10-07 — in-app guide

Added an on-demand How to modal with Notes & canvas, Writing, Remembering and Your files tabs. The canvas section includes a drawn two-note link example and explains shared note data, pinning, wiki-link arrows, pan/zoom, double-click navigation and safe unpinning. Help is read-only and has no persistent onboarding state. The native default tab is visually checked with synthetic notes; scrolling/tab/Escape interactions remain manually unverified. Existing 31 tests, formatting, Clippy and release/signature checks pass.
