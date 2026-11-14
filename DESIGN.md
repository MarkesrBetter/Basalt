# Basalt Design Read

## Architecture Split
- **Desktop Client Shell (`basalt.exe` / `src-tauri` + `frontend/`):** Semantic HTML, CSS, and TypeScript WebUI running on Tauri 2 and Windows WebView2. Persistent config, save folder management, hardware telemetry, local skin validation, TCP SLP ping, and Gradle process tracking run in the Rust core (`basalt` + `src-tauri`).
- **Real Minecraft 1.21.4 Client (`client-gradle/`):** The desktop shell spawns `gradlew.bat runClient` to run authentic Minecraft Java Edition `1.21.4` (Fabric Loader `0.20.1`, Loom `1.15.5`, Fabric API `0.119.4+1.21.4`); `options.txt` synchronization and `BASALT_*` environment variables pass player name, UUID, RAM (`-Xmx`), GC (`G1GC` / `ZGC`), local PNG skin, and Quick Play parameters.

## Dial & Aesthetic Posture
`ENERGY 1 / RHYTHM 2 / MOTION 1`

Built around an Obsidian Violet mineral identity with 10 hardware-calibrated mineral/biome themes (**Obsidian Violet**, **Charcoal Copper**, **Limestone**, **Spruce Emerald**, **Deep Lapis**, **Crimson Basalt**, **Glacier Frost**, **Ancient City**, **End Stone**, **Cherry Blossom**), independent accent presets and live WCAG-calibrated custom `#RRGGBB` accents, 3 corner profiles (`Sharp`, `Balanced`, `Soft`), 5 desktop layouts (**Left Rail**, **Top Strip**, **Compact Focus**, **Right Rail**, **Minimal Dock**), 3 live UI languages (**English** default, **Türkçe**, **Deutsch** via `i18n.ts`), true OS-fullscreen **Console Mode (`DeckMode`, `#console-home`)**, custom thin scrollbars, and authentic Minecraft `1.21.4` `.ogg` sound cues (`src/sounds.rs` + `sounds.ts`).

## Color System & WCAG 2.2 AA Tokens (10 Themes)
- **Obsidian Violet (`dark`, default, `frontend/src/styles/tokens.css`):** `#131118` obsidian canvas, `#201C29` surface, `#A47CFF` / `#B593FF` signature violet accent (`#0E091B` button ink).
- **Charcoal Copper (`ember`):** `#121417` basalt canvas, `#1D2127` surface, `#E0824A` / `#EB9560` warm copper accent (`#140B06` button ink).
- **Limestone (`light`):** `#F3F1EC` stone canvas, `#FFFFFF` surface, `#B34C19` burnt copper accent (`#FFFFFF` button ink).
- **Spruce Emerald (`spruce`):** `#101613` dark spruce canvas, `#1A241F` surface, `#56C27E` emerald accent (`#08170E` button ink).
- **Deep Lapis (`cobalt`):** `#0F141C` slate canvas, `#1A2230` surface, `#53B8EC` glacier cyan accent (`#06141D` button ink).
- **Crimson Basalt (`nether`):** `#161113` Nether canvas, `#241B1F` surface, `#EA6B66` redstone coral accent (`#1A0707` button ink).
- **Glacier Frost (`frost`):** `#EDF2F7` cool ice canvas, `#FFFFFF` surface, `#1D5FB8` cobalt accent (`#FFFFFF` button ink).
- **Ancient City (`sculk`):** `#0A1013` deep sculk canvas, `#131E25` surface, `#35CAD0` bioluminescent cyan accent (`#03181A` button ink).
- **End Stone (`end`):** `#14130F` void canvas, `#211F18` surface, `#E4BE52` warm gold accent (`#1A1302` button ink).
- **Cherry Blossom (`cherry`):** `#F7EEF2` soft pastel grove canvas, `#FFF9FB` surface, `#B52A5C` ruby blossom accent (`#FFFFFF` button ink).
- **Independent Accent Calibration (`colors.ts`):** Presets `Auto`, `Copper`, `Emerald`, `Cyan`, `Amber`, `Crimson`, `Amethyst` plus live contrast-calibrated custom `#RRGGBB` picker.

## Layout Modes (6 Views)
- **Left Rail (`Sidebar`):** Fixed vertical left rail (`214px`) with two-column launch workbench.
- **Top Strip (`Topbar`):** Full-width horizontal top navigation bar and wide launch stage.
- **Compact Focus (`CompactDeck`):** Centered dense instrument frame (`max-width: 980px`).
- **Right Rail (`RightRail`):** Right-anchored rail with launch controls prioritized on the leading edge.
- **Minimal Dock (`MinimalRail`):** Icon-driven narrow (`88px`) vertical navigation dock.
- **Console Mode (`DeckMode`):** Starts in the selected desktop layout (`Sidebar` by default, desktop audio muted). Clicking **Fullscreen Console** (`#btn-enter-deck`) or selecting Console layout triggers `set_fullscreen(true)` and enters a Steam Deck / PlayStation Big Picture layout featuring a dynamic SVG hero stage, player skin plinth, horizontal world/server/utility carousel (`#deck-carousel`), digital clock (`#deck-clock`), Gamepad (`A`/`B`/`Y`/`L1`/`R1` + analog/D-Pad) and keyboard (`Arrow keys`/`Q`/`E`/`M`/`Esc`) hint strip (`#deck-controls-bar`), and automatic console sound effects. `Esc`, `B`, or **Return to Desktop** restores windowed desktop mode (`set_fullscreen(false)`).

## Audio, Frameless Window Chrome & Proportional Scaling
- `src/sounds.rs` + `frontend/src/ts/sounds.ts`: Without bundling copyrighted audio assets in `basalt.exe`, caches authentic Minecraft `1.21.4` `.ogg` cues (`random/click.ogg`, `random/click_stereo.ogg`, `ui/toast/in.ogg`, `ui/toast/out.ogg`, `random/orb.ogg`, `random/wood_click.ogg`, `random/levelup.ogg`, `note/bass.ogg`) from local Fabric Loom / `.minecraft/assets` caches or the official `resources.download.minecraft.net` endpoint into `%LOCALAPPDATA%\Basalt\Basalt\data\sounds\`, playing via Web Audio `AudioContext.decodeAudioData` + `HTMLAudioElement` `blob:` pool.
- **Custom Frameless Window Chrome (`decorations: false` + `.app-titlebar`):** Replaces the OS title bar with a draggable, theme-matched Basalt chrome bar wired to Tauri IPC commands (`window_start_drag`, `window_minimize`, `window_toggle_maximize`, `window_is_maximized`, `window_close`) and crisp SVG window controls.
- **Proportional Viewport Scaling & Clean Scroll Framing (`base.css`, `layout.css`, `components.css`):** Root font size scales proportionally (`clamp(11.2px, min(1.06vw, 1.7vh), 22px)`), viewport width caps are removed, and standard desktop panels fit comfortably without heavy scrollbars (`scrollbar-width: none`).
- Desktop Play uses a unified hero workbench (`.play-hero`) paired with a hairline-separated lower data grid (`.play-under-grid`); Console Mode hides duplicate headers, combining clock and player badge into a single strip above the cover-art carousel (`.deck-card-cover`).

## Typography & Deliberate Exclusions
- `Segoe UI Variable` / `Segoe UI` system sans UI typography paired with `Consolas` (`tabular-nums`) for telemetry values.
- No box-in-box nesting clutter, decorative emojis, synthetic KPI filler blocks, or em-dash characters (`U+2014`).
