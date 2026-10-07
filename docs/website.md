# Atlas website and Mac download

Public site: https://atlas-washi.vercel.app

The dependency-free static site lives in `website/`. It uses self-hosted OFL fonts, optimized WebP screenshots of sample notebooks, keyboard-accessible tour tabs, native expandable installation guidance, and responsive layouts. It has no forms, accounts, analytics scripts, or backend.

Preview from the repository root:

```sh
python3 -m http.server 4173 --directory website
```

## Vercel

Project: `atlas-washi` (`prj_DswZ0rydBxKStVUIy3lpbJebE6F7`). Deploy the contents of `website/` as a static site: framework Other, no install or build command, output `.`. The first production deployment was made through the connected Vercel API, with source files uploaded directly.

The Vercel GitHub integration currently cannot access `flyndagr/atlas-washi` (`repo_no_access`). Automatic Git deployments are **not connected**. After granting the Vercel GitHub app access to that repository, connect it and set its root directory to `website`. Do not deploy the repository root or the personal notebook folder.

## Mac release

```sh
bash packaging/macos.sh
bash packaging/dmg.sh
```

The second script checks the bundle signature and architecture, creates a compressed disk image with an Applications shortcut and installation notes, verifies the image, and writes `dist/SHA256SUMS.txt`. It packages the existing bundle; rebuild first to include source changes.

Release v0.2.0 packages app source commit `ac2156f7f4c1f10f1ea76371c54344e5622c1d30`. The DMG and checksum are GitHub release assets, not source-controlled binaries. The app is ad-hoc signed, not Developer ID signed or notarized. Apple Silicon only, minimum declared macOS 12. The site links Apple's current first-launch guidance and makes these limitations explicit.

## Verification (2026-10-07)

- Disk image verification passed; mounted read-only and app signature validated. Contents: app, Applications symlink, and installation notes only.
- Public GitHub download SHA-256 matched the original: `bd07ac71693a7af368a6b52c17d864f29e7c7bb60f838aed7f849aa866c1a80d`.
- Browser checks: desktop 1440px; mobile 390px and 320px with no horizontal overflow; all three tour tabs; keyboard arrow navigation; FAQ expansion; no JavaScript errors.
- Production status READY; static build logs contain no build errors.
- No personal Atlas Vault files were read, changed, or published for this site.
