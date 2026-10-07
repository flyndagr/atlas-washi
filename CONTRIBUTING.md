# Contributing

Atlas is an early macOS-focused Rust app. Small, well-tested improvements are welcome.

1. Open an issue for substantial behavior or architecture changes.
2. Use a disposable notebook (`cargo run -- --vault /path/to/test-notes`).
3. Keep notes portable as Markdown, preserve external files on conflicts, and retain recovery paths.
4. Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and `cargo test --locked`.
5. Include before/after screenshots for visual changes and describe the behavior you checked.

The washi palette, quiet motion, accessible control names, and optional fountain-pen typography are intentional. Prefer small changes to decorative redesigns. Never include your personal notebook, credentials, or absolute user paths in a pull request.

Pull requests are submitted under the project's MIT license. Bundled font licenses remain SIL OFL.
