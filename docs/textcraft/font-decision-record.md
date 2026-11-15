# Font Decision Record — Basalt (`2026-11-15`)

## Context
- **Users / tasks / reading density:** Windows 10/11 desktop Minecraft Java Edition (Vanilla `1.21.4` – `1.16.5`) players scanning launch readiness, world save lists (`size`, `last saved`), JVM/graphics parameters (`-Xmx`, `G1GC`/`ZGC`, chunk radius), and SHA-256 verification strings.
- **Target platforms and offline needs:** Single-binary portable Windows x64 Tauri 2 + WebView2 application (`basalt.exe`) that must work 100% offline without external font CDNs (`fonts.googleapis.com` is forbidden).
- **Primary locales and Unicode scripts:** English (`en-US`), Turkish (`tr-TR`: `İ I ı i Ğ ğ Ş ş Ç ç Ö ö Ü ü`), and German (`de-DE`: `Ä ä Ö ö Ü ü ß`).
- **Actual UI surfaces:** Desktop launcher (`frontend/index.html`) and product website (`site/index.html`).

## Candidates

| Family | Source link | License verified? | Specific file(s) | TR glyphs verified? | 12-16px legibility | Weight/axes | Bundle bytes | Fallback risk |
|---|---|---|---|---|---|---|---|---|
| **1. Windows Native System UI (`Segoe UI Variable Text` / `Segoe UI`) + `Cascadia Code` / `Consolas`** | Built into Windows 10/11 OS (no file redistribution) | Yes — referenced via CSS system font stack only; zero font files redistributed | OS-installed `SegUIVar.ttf` / `segoeui.ttf`, `CascadiaCode.ttf` / `consola.ttf` | Yes — full Latin Extended-A (`İ I ı i Ğ ğ Ş ş Ç ç Ö ö Ü ü`) | High x-height, optical size adaptation (`Segoe UI Variable Text`), clear `0/O` and `1/I/l` in `Consolas` / `Cascadia Code` | `400`, `600`, `700` | `0 B` | Zero network latency, zero FOUT/CLS |
| **2. Inter (OFL 1.1)** | `https://github.com/rsms/inter` | SIL Open Font License 1.1 | Not bundled | Yes | High | Variable `wght` | `~310 KB` (if bundled) | Adds binary weight and startup decode overhead compared to native OS text stack |
| **3. IBM Plex Sans + IBM Plex Mono (OFL 1.1)** | `https://github.com/IBM/plex` | SIL Open Font License 1.1 | Not bundled | Yes | High | Static/Variable | `~420 KB` (if bundled) | Unnecessary bundle weight for a single-file native Windows utility |

## Chosen Families and Usage
- **Body & Headings (`--font-sans`):** `"Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, -apple-system, BlinkMacSystemFont, Arial, sans-serif`
- **Numeric / Code / Paths / Hashes (`--font-mono`):** `"Cascadia Code", "Cascadia Mono", "Consolas", ui-monospace, "Liberation Mono", monospace` with `font-variant-ligatures: none` and `font-variant-numeric: tabular-nums`
- **Why chosen over alternatives:**
  1. **Zero CLS & Instant First Paint:** Because `basalt.exe` is an offline-capable portable Windows desktop app, referencing the native Windows system stack guarantees `0 ms` font download wait and `0.00` font-swap layout shift.
  2. **Complete Turkish & German Glyph Support:** Native Windows `Segoe UI Variable Text` / `Segoe UI` and `Consolas` render `İ I ı i Ğ ğ Ş ş Ç ç Ö ö Ü ü ß` cleanly at `12px–32px` without mixed-font glyph fallback.
  3. **Strict License Compliance:** No proprietary font binaries are embedded or redistributed inside the repository or executable.

## QA Evidence
- **Font fallback simulation:** Tested with primary families disabled down to `Arial, sans-serif` and `monospace`; container heights use `min-height` (`2.65rem`) and `overflow-wrap: anywhere` so no labels clip.
- **Turkish sample verification:** Verified `"İzmir, ışık, şüphe, çığ, öğün, küçük, Şişli, Iğdır"` across `toLocaleLowerFold("tr-TR")` and `Intl.Collator("tr-TR")`.
- **CLS measurement:** `0` webfont network requests; no asynchronous font swap occurs.
- **License review:** 100% OS-referenced CSS stacks; `0` third-party font files distributed.
