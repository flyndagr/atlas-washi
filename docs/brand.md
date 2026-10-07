# Atlas identity

Atlas has one mark: an open ink circle with an upright red fountain-pen nib centered inside. Its slit and breather hole are transparent. The original sidebar circle radius (14 points), wordmark alignment, and transparent background are preserved.

`assets/brand.json` is the canonical geometry and color definition. `src/washi.rs` reads it for native rendering; `packaging/icon.py` reads it to export the transparent 512px mark, website favicon, and macOS PNG/ICNS icon. The Dock icon alone adds the rounded washi tile. Do not independently redraw, rotate, move, or substitute the nib in another surface.

Regenerate with `python3 packaging/icon.py` (Pillow required). Website header and footer use the same high-resolution transparent PNG. Native screenshots must be captured from the current app with a disposable sample notebook, never repaired by painting the logo over an old screenshot.

Design decision: retain the established upright, centered nib rather than the website's unrelated diagonal pen. Preserve the native mark's geometry rather than shrinking a square Dock icon into the header. The web header's image box includes the same proportional clear space as the app's 38-point logo allocation.

Verification: review transparent exports on paper and white, 16/28/34/64px displays, the native sidebar, and fresh notebook/canvas/review captures. Website checks include desktop and 320/390px layouts. These are implementation and visual checks, not a claim of independent third-party design certification.
