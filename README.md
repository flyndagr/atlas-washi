# Atlas — the Washi Edition

[GitHub · flyndagr/atlas-washi](https://github.com/flyndagr/atlas-washi)

A quiet, native Rust notebook for macOS. Fountain-pen typography, handmade paper, and a canvas for connected ideas. Your notes remain ordinary Markdown files on your Mac.

![Atlas notebook](docs/media/notebook.png)

## A little space for big ideas

- **Write and read:** Markdown source editing, clean rendered reading, autosave, headings, lists, checklists, links, and code.
- **Connect:** `[[Wiki links]]`, backlinks, local attachments, and a canvas of draggable note cards.
- **Make it yours:** four paper/ink palettes, adjustable fibers, fountain script or typeset text.
- **Keep control:** import files/folders, export Markdown or a notebook with assets, rename, duplicate, recoverable Trash.
- **Stay local:** no accounts, analytics, server, API keys, or app-managed cloud storage.

![Atlas canvas](docs/media/canvas.png)

[Watch the 18-second demo](docs/media/atlas-demo.mp4).

## Build and run

Tested on Apple silicon macOS with Rust 1.98. The current release is an early prototype. Install a current stable Rust toolchain and Apple's command-line developer tools, then:

```sh
cargo run --release --locked
# Use a separate notebook:
cargo run --release --locked -- --vault /absolute/path/to/notebook
# Build a local macOS app:
bash packaging/macos.sh
open dist/Atlas.app
```

The default notebook is `~/Documents/Codex/Atlas Vault`. Five sample notes are created only for a new empty notebook. The app bundle is locally ad-hoc signed, not Apple-notarized. Other platforms are not tested; file-opening integration currently targets macOS.

## Controls

Hover any icon for its label. The document icon opens Files (new/import/export/Trash). The canvas icon switches between notebook and canvas; the pin adds the selected note and brings it into view.

- **Sidebar:** changes only the left notes panel.
- **Focus (⌘⇧F):** hides both panels and restores the previous layout.
- **Pencil / checkmark:** edit Markdown / return to clean reading.
- **Canvas:** drag a card to arrange it, drag empty space to pan, double-click a card to open it, right-click to unpin without deleting the note. The target icon brings the selected pinned card into view.
- **Note context menu:** right-click a sidebar note to open, rename, duplicate, export, or move it to Trash.
- **Lists:** select lines while editing, then choose bullets, numbers, or a checklist. Enter continues a list; Enter on an empty item ends it.
- **Folder / refresh:** reveal the notebook in Finder / read changes made outside Atlas.
- **⌘N / ⌘K / ⌘S / ⌘O / ⇧⌘S:** new note / search / save / import Markdown / export current draft.

## Files and recovery

Markdown notes may be nested. Attachments live in `attachments/`; `.atlas/` stores appearance, canvas positions, Trash, and rename backups. Back up the **whole notebook folder** to retain all of them. Notebook export copies visible notes/assets and intentionally excludes hidden Atlas state.

Writes use atomic replacement and check for external changes. On a note conflict, the draft stays in the app and can be saved as a new note. Canvas conflicts preserve the existing file. Use one app instance per notebook; this is not a multi-process synchronization system. Closing is blocked if a save fails.

Folder import preserves relative asset paths and copies into a unique `imports/` folder. Single-file import copies Markdown text only. Imports reject symbolic links, enforce limits (5 MB per note; 500 MB/10,000 files per folder), and never overwrite existing notes. Exports require new destinations. Rename updates resolved wiki and inline Markdown links, not reference-style link definitions.

## Current limits

No sync, plugins, inline rich-text editing, full Markdown tables/math, built-in image/PDF viewer, canvas zoom, or automatic filesystem watching. Canvas cards are fixed-size summaries; long titles/excerpts are shortened. Checklist rendering is visual; edit its Markdown to change completion. Keep backups of valuable work while evaluating the prototype.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

See [CONTRIBUTING.md](CONTRIBUTING.md). CI runs on macOS. A passed local test run is not a substitute for the GitHub Actions result.

## License and credits

Application code and original artwork: [MIT](LICENSE). Bundled fonts: SIL OFL, with license texts in `assets/`. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Inspired by the local-first spirit of [Lightcraft](https://github.com/storytold/lightcraft) and familiar note-taking interactions. Atlas is an independent project, not an affiliated or official version of those products.
