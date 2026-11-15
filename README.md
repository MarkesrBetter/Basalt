# Basalt

> **Disclaimer:** *NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.*

Native Windows 10/11 desktop launcher powered by **Tauri 2 + TypeScript WebUI** and a **Rust** core, running authentic **Minecraft Java Edition (Vanilla `1.21.4` – `1.16.5`)** directly via `java.exe` without mod loaders.

## Download & Website

- Single-file Windows executable (`basalt.exe`, `13.68 MB` / `14,348,288 bytes`): [GitHub Releases](https://github.com/Markesrrr/Basalt/releases/latest)
- Live product page (GitHub Pages): [markesrrr.github.io/Basalt](https://markesrrr.github.io/Basalt/) ([site/index.html](./site/index.html))
- Verification (`v0.1.0` SHA-256): `8c748d53f2235a0b3b7aa445172c2805a7906a81de9dbb0e987389f74eb9303f`

## Features

- **Single-Binary Portable Windows App (`basalt.exe`):** Runs without an installer wizard; supports 15 official Mojang Vanilla versions (`1.21.4`, `1.21.3`, `1.21.2`, `1.21.1`, `1.21`, `1.20.6`, `1.20.4`, `1.20.2`, `1.20.1`, `1.20`, `1.19.4`, `1.19.2`, `1.18.2`, `1.17.1`, `1.16.5`) and SHA-1-verifies client JARs, native libraries, and asset indexes directly into an ASCII-safe local cache (reusing verified local `.minecraft` files when present) while keeping `run/` saves and `options.txt` in app data.
- **Global English Default + 3 Live Languages:** Defaults to English across the desktop launcher, product website, and documentation, with live switching between **English**, **Türkçe**, and **Deutsch**.
- **Hardware-Calibrated Speed & FPS:** Reads CPU cores, RAM, and GPU tier via `sysinfo` and headless `wgpu` DX12 detection, calibrating `-Xmx`, `G1GC`/`ZGC`, and `options.txt` automatically.
- **Automatic Java 21 Setup:** If no Java `>= 21` runtime is found, downloads Eclipse Temurin 21 via signed Windows `curl.exe`, verifies its SHA-256 digest in pure Rust, and installs locally without admin privileges.
- **10 Mineral Themes & Fullscreen Console Mode:** 7 dark and 3 light palettes, custom `#RRGGBB` accent contrast calibration, 3 corner profiles, 5 desktop layouts (`Sidebar`, `Topbar`, `CompactDeck`, `RightRail`, `MinimalRail`), plus fullscreen **Console Mode** with gamepad support.
- **Pure Vanilla Skins & World Management:** Import local `64x64` / `64x32` PNG skins with `Slim (3 px)` / `Classic (4 px)` arm models (packaged automatically as `run/resourcepacks/basalt_skin.zip` with UUID arm-model alignment), inspect singleplayer saves, and launch directly via Quick Play.

## Scope & Safety Guarantees

- **Zero Copyrighted Assets Bundled:** Official Mojang/Minecraft client JARs, resource packs, and `.ogg` interface sounds are never committed to Git or embedded inside `basalt.exe`. They are cached locally from official Mojang endpoints on first launch.
- **No Authentication Bypass:** Supports singleplayer saves (`--quickPlaySingleplayer`) and local offline-mode (`online-mode=false`) development servers only. The Protocol 769 server probe disconnects on `EncryptionRequest (0x01)` and never bypasses Microsoft account authentication.
- **Open-Source Release Verification:** Release binaries (`basalt.exe`) are distributed via **GitHub Releases** alongside verifiable SHA-256 digests rather than checked into Git history.

## Build from Source

Prerequisites: **Rust (`cargo`)**, **Node.js (`npm`)**, and **Windows Edge WebView2 Runtime** (preinstalled on Windows 10/11). **Java 21 JDK** is detected automatically at runtime or auto-installed by Basalt.

```powershell
cd frontend
npm.cmd install
npm.cmd run build
cd ..
cargo test --workspace
cargo build --release --workspace
.\target\release\basalt.exe
```

## Licenses & Third-Party Notices

- Basalt source code: [MIT License](./LICENSE)
- Desktop shell: Tauri 2 (MIT / Apache-2.0)
- Default skin asset (`assets/default_skin.png`): Replace with your own original artwork before redistributing forks.
