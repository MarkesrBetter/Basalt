# Basalt

> **Disclaimer:** *NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.*

Native Windows 10/11 desktop launcher powered by **Tauri 2 + TypeScript WebUI** and a **Rust** core, running authentic **Minecraft Java Edition 1.21.4** via Fabric Loom.

## Download & Website

- Single-file Windows executable (`basalt.exe`, `13.34 MB` / `13,984,256 bytes`): [GitHub Releases](https://github.com/MarkesrBetter/Basalt/releases/latest)
- Live product page (GitHub Pages): [markesrbetter.github.io/Basalt](https://markesrbetter.github.io/Basalt/) ([site/index.html](./site/index.html))
- Verification (`v0.1.0` SHA-256): `7ac3541b37f4b7f09f03dcff037ea526e138a2105f2a1fd4f2f2757539d68023`

## Features

- **Single-Binary Portable Windows App (`basalt.exe`):** Runs without an installer wizard; extracts the `client-gradle/` Fabric template into local app data (`%LOCALAPPDATA%\Basalt\Basalt\data\client-gradle`) on first launch.
- **Global English Default + 3 Live Languages:** Defaults to English across the desktop launcher, product website, and documentation, with live switching between **English**, **Türkçe**, and **Deutsch**.
- **Hardware-Calibrated Speed & FPS:** Reads CPU cores, RAM, and GPU tier via `sysinfo` and headless `wgpu` DX12 detection, calibrating `-Xmx`, `G1GC`/`ZGC`, and `options.txt` automatically.
- **Automatic Java 21 Setup:** If no Java `>= 21` runtime is found, downloads Eclipse Temurin 21 via signed Windows `curl.exe`, verifies its SHA-256 digest in pure Rust, and installs locally without admin privileges.
- **10 Mineral Themes & Fullscreen Console Mode:** 7 dark and 3 light palettes, custom `#RRGGBB` accent contrast calibration, 3 corner profiles, 5 desktop layouts (`Sidebar`, `Topbar`, `CompactDeck`, `RightRail`, `MinimalRail`), plus fullscreen **Console Mode** with gamepad support.
- **Skins & World Management:** Import local `64x64` / `64x32` PNG skins with `Slim (3 px)` / `Classic (4 px)` arm models, inspect singleplayer saves, and launch directly via Quick Play.

## Scope & Safety Guarantees

- **Zero Copyrighted Assets Bundled:** Official Mojang/Minecraft client JARs, resource packs, and `.ogg` interface sounds are never committed to Git or embedded inside `basalt.exe`. They are cached locally from official Gradle, Fabric, and Mojang endpoints (`%LOCALAPPDATA%\Basalt\Basalt\data\`) on first launch.
- **No Authentication Bypass:** Supports singleplayer saves (`--quickPlaySingleplayer`) and local offline-mode (`online-mode=false`) development servers only. The Protocol 769 server probe disconnects on `EncryptionRequest (0x01)` and never bypasses Microsoft account authentication.
- **Open-Source Release Verification:** Release binaries (`basalt.exe`) are distributed via **GitHub Releases** alongside verifiable SHA-256 digests rather than checked into Git history.

## Build from Source

Prerequisites: **Rust (`cargo`)**, **Node.js (`npm`)**, **Java 21 JDK**, and **Windows Edge WebView2 Runtime** (preinstalled on Windows 10/11).

```powershell
cd frontend
npm.cmd install
npm.cmd run build
cd ..\client-gradle
.\gradlew.bat build --no-daemon
cd ..
cargo test --workspace
cargo build --release --workspace
.\target\release\basalt.exe
```

## Licenses & Third-Party Notices

- Basalt source code: [MIT License](./LICENSE)
- Gradle Wrapper (`client-gradle/gradle/wrapper/`): Apache License 2.0
- Desktop shell: Tauri 2 (MIT / Apache-2.0)
- Default skin asset (`assets/default_skin.png`): Replace with your own original artwork before redistributing forks.
