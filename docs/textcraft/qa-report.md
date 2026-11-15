# TEXTCRAFT QA Report — Basalt (`2026-11-15`)

## Build
- **Project:** Basalt (`v0.1.0`) — Tauri 2 + TypeScript WebUI + Rust
- **Browser / WebView:** Microsoft Edge WebView2 Runtime (Windows 10/11 x64)
- **Locales tested:** `en-US` (default), `tr-TR`, `de-DE`
- **Themes tested:** All 10 mineral themes (`Dark`, `Ember`, `Light`, `Spruce`, `Cobalt`, `Nether`, `Frost`, `Sculk`, `End`, `Cherry`) + `site/index.html` (`dark` / `light`)
- **Fonts configured:** Native Windows UI stack (`Segoe UI Variable Text`, `Segoe UI`, system sans fallback) + `Cascadia Code` / `Consolas` monospace

## Automated Checks (Executed)
- **Skill Package Structure Audit (`tools/package_audit.py`):** `total files: 100 references: 41 recipes: 31 — STRUCTURE PASS`
- **Locale Key & Variable Parity Audit (`tools/i18n_audit.py docs/textcraft/locale/tr.json docs/textcraft/locale/en.json`):** `locale keys: 352 TR / 352 EN — audit: PASS` (`0` key mismatches, `0` variable mismatches, `0` empty strings)
- **Generic AI / Vague Copy Audit (`tools/copy_audit.py` across `tr.json`, `en.json`, `i18n.ts`, `frontend/index.html`, `site/index.html`):** `warnings: 0` (replaced vague `"Beklenmeyen bir hata oluştu."` with actionable recovery copy)
- **WCAG 2.2 Contrast Audit (`tools/contrast.py`):**
  - Desktop `tokens.css` (all 10 themes × 7 foreground tokens × 4 surface tokens + primary button `--accent-text` on `--accent`): `0` pairs below `4.5:1` (`PASS`)
  - Website `site/index.html` dark `--text-subtle` (`#8b96a5` on `#20252d`): `5.13:1` (`WCAG AA normal: PASS`)
  - Website `site/index.html` light `--text-subtle` (`#59616d` on `#f7f5f0`): `5.74:1` (`WCAG AA normal: PASS`)
- **Kinetic Motion Static Audit (`motion_audit.py --strict`):** `0 possible risks` across `frontend/src` (12 files) and `site` (1 file)
- **Rust Test Suite (`cargo test --workspace`):** `16/16 passed` (`8` unit + `8` integration tests)

## Typography & Accessibility Engineering Checks
- **Root Font Scaling (`base.css`):** Replaced viewport-shrinking `clamp(11.2px, ...)` with `clamp(14px, 0.82rem + 0.22vw, 18px)` and `text-size-adjust: 100%` so 320px/720px windows and 125%/150%/200% zoom preserve legible text sizes.
- **Turkish Dotted/Dotless `İ/I/ı/i` & Collation (`formatting.ts`):** Implemented `toLocaleLowerFold()`, `toLocaleUpperSafe()`, and `compareLocaleStrings()` using `Intl.Collator("tr-TR")` and Unicode `NFC` normalization.
- **Numerals & Dates (`formatting.ts`):** Standardized on `Intl.NumberFormat` and `Intl.DateTimeFormat` with `.tabular` (`font-variant-numeric: tabular-nums`) on all metrics, byte sizes, RAM values, and timestamps.
- **Form Labels & Error Associations (`frontend/index.html`):** Persistent `<label for>` on every input/select (including the new `#worlds-filter-input` and `#worlds-sort-select`), plus explicit `aria-describedby` linking inputs to `role="alert"` error messages.
- **Context-Specific Row Action Labels (`main.ts`):** Added world-name and server-name specific `aria-label` attributes to repeated row buttons (`Hedef seç`, `Başlat`, `Sil`, `Durum (SLP)`, `El sıkışma`, `Bağlan`, `Kaldır`).

## Findings & Fixes Applied

| Severity | Surface | Issue Found | Fix Applied | Status |
|---|---|---|---|---|
| Medium | `site/index.html` | Dark mode `--text-subtle` (`#788393`) on `--card-alt` (`#20252d`) had `4.01:1` contrast (`< 4.5:1`) | Updated dark `--text-subtle` to `#8b96a5` (`5.13:1`) and light `--text-subtle` to `#59616d` (`5.74:1`) | Fixed |
| Medium | `frontend/src/ts/i18n.ts` | `err.unexpected` used vague phrase `"Beklenmeyen bir hata oluştu."` (flagged by `copy_audit.py`) | Replaced with actionable recovery copy (`"İstek tamamlanamadı. Çalışma zamanı günlüğünü kontrol edip işlemi tekrar deneyin."`) | Fixed |
| Medium | `frontend/src/styles/base.css` | `html` root font size used `clamp(11.2px, min(1.06vw, 1.7vh), 22px)`, shrinking body/caption text below `12px` on compact windows | Changed to `clamp(14px, 0.82rem + 0.22vw, 18px)` with `text-size-adjust: 100%` | Fixed |
| Low | `frontend/index.html` (`#view-worlds`) | Saved Worlds view lacked name filtering, locale collation sorting, and separate empty-library vs zero-filter-match states | Added `.worlds-toolbar` (`#worlds-filter-input`, `#worlds-sort-select`, `#btn-clear-world-filter`) and `worlds.zeroFilterBanner` | Fixed |
| Low | `frontend/src/ts/main.ts` | `handleOptimizeClientNow()` displayed hardcoded English strings (`"DNS flushed"`, `"TCP tuned"`, `"active JVM/Netty flags"`) | Localized via `turbo.optResult`, `turbo.dnsFlushed`, `turbo.tcpTuned`, `turbo.pingOptimized`, `turbo.pingNa` | Fixed |

## Explicit Limitations
- **Not tested automatically:** Physical screen-reader speech output (NVDA / Narrator live voice playback) and multi-monitor mixed-DPI dragging in live Windows sessions.
- **Why:** Headless CLI verification runs static/DOM/contrast/locale checks and Rust unit/integration tests, which cannot substitute for human listening tests with NVDA/Narrator.
