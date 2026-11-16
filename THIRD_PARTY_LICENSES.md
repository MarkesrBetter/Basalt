# Third-Party Licenses & Trademarks

Basalt is licensed under the [MIT License](./LICENSE).

> **Minecraft Disclaimer:** *NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.*
> Minecraft is a trademark of Mojang Synergies AB. All rights in official game assets, binaries, and brand marks belong to Mojang AB / Microsoft Corporation.

---

## 1. Upstream Services, Runtimes & Ecosystem Attribution

Basalt never redistributes copyrighted Minecraft client JARs, sound/asset packs, Java runtimes, mod-loader installers, OptiFine binaries, or third-party mods/shaders inside Git or release archives. Instead, Basalt fetches verified resources directly on the user's machine from official upstream services:

| Ecosystem / Service | Usage in Basalt | License / Terms |
| --- | --- | --- |
| **Mojang Studios / Minecraft** (`piston-meta.mojang.com`, `resources.download.minecraft.net`) | Official version manifests, Vanilla client JARs, libraries, assets, and optional UI `.ogg` sound cues downloaded and SHA-1 verified locally | [Minecraft EULA](https://www.minecraft.net/en-us/eula) & [Minecraft Usage Guidelines](https://www.minecraft.net/en-us/usage-guidelines) |
| **Modrinth API v2** (`api.modrinth.com/v2`, `cdn.modrinth.com`) | Catalog browsing, search, dependency resolution, and direct file downloads respecting each creator's license & attribution | [Modrinth Terms of Use](https://modrinth.com/legal/terms) |
| **FabricMC** (`meta.fabricmc.net`, `maven.fabricmc.net`) | Fabric Loader & Intermediary metadata and libraries fetched dynamically for isolated Fabric profiles | Apache-2.0 / ecosystem licenses |
| **Minecraft Forge** (`files.minecraftforge.net`, `maven.minecraftforge.net`) | Official Forge promotions metadata and installer downloaded locally on user request for isolated Forge profiles | LGPL-2.1 / Forge terms |
| **Eclipse Adoptium (Temurin)** (`api.adoptium.net`) | Optional local Java 21 JDK installation verified via SHA-256 | GPLv2 with Classpath Exception |
| **Discord** (`\\.\pipe\discord-ipc-*`) | Optional local Discord Rich Presence activity display | [Discord Developer Policy](https://discord.com/developers/docs/policies-and-agreements/developer-policy) |
| **OptiFine / Custom User Archives** | Not bundled, downloaded, or redistributed by Basalt. Users may import their own locally obtained `.jar` or `.zip` files into their local folders | Respective author licenses |

---

## 2. Direct Rust Crates (`Cargo.toml` / `src-tauri/Cargo.toml`)

All 498 Rust workspace dependencies use permissive or weak file-copyleft open-source licenses (`MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, `CC0-1.0`, `MPL-2.0`). Zero GPL-only or AGPL crates are included.

| Crate | Version | License | Upstream Repository |
| --- | --- | --- | --- |
| `tauri` | `2.12.5` | `Apache-2.0 OR MIT` | <https://github.com/tauri-apps/tauri> |
| `tauri-build` | `2.7.4` | `Apache-2.0 OR MIT` | <https://github.com/tauri-apps/tauri> |
| `wgpu` | `29.0.4` | `MIT OR Apache-2.0` | <https://github.com/gfx-rs/wgpu> |
| `pollster` | `0.4.0` | `Apache-2.0 OR MIT` | <https://github.com/zesterer/pollster> |
| `image` | `0.25.10` | `MIT OR Apache-2.0` | <https://github.com/image-rs/image> |
| `serde` | `1.0.229` | `MIT OR Apache-2.0` | <https://github.com/serde-rs/serde> |
| `serde_json` | `1.0.154` | `MIT OR Apache-2.0` | <https://github.com/serde-rs/json> |
| `toml` | `0.8.23` | `MIT OR Apache-2.0` | <https://github.com/toml-rs/toml> |
| `flate2` | `1.1.11` | `MIT OR Apache-2.0` | <https://github.com/rust-lang/flate2-rs> |
| `sysinfo` | `0.38.4` | `MIT` | <https://github.com/GuillaumeGomez/sysinfo> |
| `directories` | `6.0.0` | `MIT OR Apache-2.0` | <https://github.com/soc/directories-rs> |
| `rfd` | `0.18.0` | `MIT` | <https://github.com/PolyMeilex/rfd> |

---

## 3. Notable Transitive License Notices (Rust & WebUI)

When redistributing compiled binaries (`basalt.exe`), the following transitive licenses require notice preservation:

- **Apache-2.0:** `codespan-reporting` (`0.13.1`), `gl_generator` (`0.14.0`), `glutin_wgl_sys` (`0.6.1`), `khronos_api` (`3.1.0`), `spirv` (`0.4.0+sdk-1.4.341.0`), `sync_wrapper` (`1.0.2`), `tao` (`0.37.2`), `typescript` (`5.9.3`, build-time). Full text: <https://www.apache.org/licenses/LICENSE-2.0>.
- **Mozilla Public License 2.0 (`MPL-2.0`):** `cssparser` (`0.37.0`, `0.38.0`), `cssparser-macros` (`0.7.1`), `dtoa-short` (`0.3.5`), `option-ext` (`0.2.0`), `selectors` (`0.38.0`, `0.41.0`). Source code for unmodified upstream crates is available in their upstream repositories and under Mozilla Public License 2.0 (<https://www.mozilla.org/en-US/MPL/2.0/>).
- **BSD-3-Clause:** `alloc-no-stdlib` (`3.0.0`), `alloc-stdlib` (`0.3.0`).
- **ISC:** `libloading` (`0.7.4`, `0.8.9`).
- **Zlib:** `foldhash` (`0.1.5`, `0.2.0`), `slotmap` (`1.1.1`), `zlib-rs` (`0.6.8`).
- **Unicode-3.0:** ICU4X crates (`icu_collections`, `icu_locale_core`, `icu_normalizer`, `icu_properties`, `icu_provider`, `litemap`, `potential_utf`, `tinystr`, `writeable`, `yoke`, `zerofrom`, `zerotrie`, `zerovec`).
- **CC0-1.0:** `hexf-parse` (`0.2.1`).
- **Frontend Runtime (`frontend/package.json`):** `@tauri-apps/api` (`2.9.1`, `MIT OR Apache-2.0`); build dependencies (`vite`, `typescript`, Rollup/esbuild plugins) under `MIT`, `Apache-2.0`, `ISC`, and `BSD-3-Clause`.
- **Default Skin Asset (`assets/default_skin.png`):** Original procedural Basalt character artwork created specifically for the Basalt project and released under the project's [MIT License](./LICENSE).

You can regenerate the exact workspace dependency inventory locally at any time with:

```powershell
cargo metadata --format-version 1
```
