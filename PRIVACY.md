# Privacy Policy & Network Disclosure

Basalt is designed to run **locally on your Windows PC with zero telemetry, zero advertising, and zero user tracking**.

## 1. Data Basalt Does NOT Collect

- No usage analytics, crash telemetry uploads, device fingerprinting, or advertising SDKs.
- No account harvesting: Basalt never asks for, transmits, or bypasses Microsoft / Xbox Live passwords or OAuth credentials.
- On the Basalt GitHub Pages website (`https://markesrrr.github.io/Basalt/`), no analytics scripts or tracking cookies are used.

## 2. Local Data Stored on Your Device

Basalt stores your settings, saves, resource packs, shaderpacks, and isolated mod folders only on your local drive:

- Configuration, skin PNG, server list, and logs: `%APPDATA%\Basalt\` (`directories::ProjectDirs::from("dev", "Basalt", "Basalt")`)
- Downloaded Minecraft runtime files, Java 21 runtime, isolated `loader_mods/` (`Fabric`), `profiles/` (`Forge`), and shared `saves/`, `shaderpacks/`, `resourcepacks/`, `screenshots/`: Basalt data / ASCII-safe cache directory configured on your machine.
- Website preference (`site` theme / active display language in memory): not transmitted to any backend server.

## 3. Outbound Network Connections & Why They Occur

Basalt only contacts external endpoints when directly required for an action you initiate:

| Endpoint | Trigger | Purpose |
| --- | --- | --- |
| `piston-meta.mojang.com`, `launchermeta.mojang.com`, `libraries.minecraft.net`, `resources.download.minecraft.net` | Prepare / Play Minecraft | Fetch official Mojang version manifests, client JARs, libraries, and game assets with SHA-1 verification |
| `api.adoptium.net`, GitHub Adoptium release mirrors | Click **Auto-Install Java 21** | Download official Eclipse Temurin 21 JDK and verify its SHA-256 digest |
| `meta.fabricmc.net`, `maven.fabricmc.net` | Prepare / Play **Fabric** | Fetch official Fabric loader metadata and libraries |
| `files.minecraftforge.net`, `maven.minecraftforge.net` | Prepare / Play **Forge** | Fetch Forge promotions metadata and official installer |
| `api.modrinth.com/v2`, `cdn.modrinth.com` | Open / Search **Library** or Install Mod / Shader Kit | Query public Modrinth v2 catalog and download creator-hosted `.jar` / `.zip` files with `User-Agent: Basalt-Launcher/0.3.0 (https://github.com/Markesrrr/Basalt)` |
| `api.github.com/repos/Markesrrr/Basalt/releases` | Website page load (`site/index.html`) | Read latest public GitHub release version, asset download URLs, file size, and SHA-256 checksum |
| `\\.\pipe\discord-ipc-0..9` (local IPC only) | When Discord Rich Presence is enabled | Display current Minecraft activity locally in your Discord desktop client |
| Self-hosted / LAN Server (`host:port`) | Server Ping / Probe / Play | Protocol 769 status/login query to user-entered `online-mode=false` local or LAN development servers |
