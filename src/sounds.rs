use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub struct McSoundSpec {
    pub key: &'static str,
    pub asset_path: &'static str,
    pub sha1_1_21_4: &'static str,
}

/// Real Mojang Minecraft 1.21.4 (`1.21.4-19.json`) UI and feedback `.ogg` sound assets.
pub const MC_SOUND_SPECS: &[McSoundSpec] = &[
    McSoundSpec {
        key: "click",
        asset_path: "minecraft/sounds/random/click.ogg",
        sha1_1_21_4: "3455ca942556c7d5eac7dd5e458e7fb3bad564c9",
    },
    McSoundSpec {
        key: "focus",
        asset_path: "minecraft/sounds/random/click_stereo.ogg",
        sha1_1_21_4: "f0ca66561f832bf2f60b393837297c2692367cd5",
    },
    McSoundSpec {
        key: "tab",
        asset_path: "minecraft/sounds/ui/toast/in.ogg",
        sha1_1_21_4: "506f2fdb1b7530df66134aa04c71e66513df0c93",
    },
    McSoundSpec {
        key: "back",
        asset_path: "minecraft/sounds/ui/toast/out.ogg",
        sha1_1_21_4: "984a7d9501fd7d679c1c91af756ca8633beb09b6",
    },
    McSoundSpec {
        key: "toggleOn",
        asset_path: "minecraft/sounds/random/orb.ogg",
        sha1_1_21_4: "8a04a60d5c28fc60df472a877ca57f37eabc78d7",
    },
    McSoundSpec {
        key: "toggleOff",
        asset_path: "minecraft/sounds/random/wood_click.ogg",
        sha1_1_21_4: "cb7fd9405357f21decb5de0b373b837dadcbca9c",
    },
    McSoundSpec {
        key: "launch",
        asset_path: "minecraft/sounds/random/levelup.ogg",
        sha1_1_21_4: "19034765ba8ba5389b35804ab213537ab5cf706f",
    },
    McSoundSpec {
        key: "error",
        asset_path: "minecraft/sounds/note/bass.ogg",
        sha1_1_21_4: "e28d844995368de1d45c724a5d371c9952ae5ed4",
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct MinecraftSoundsDto {
    pub ready_count: usize,
    pub total_count: usize,
    pub missing_keys: Vec<String>,
    pub sounds: BTreeMap<String, String>,
}

pub fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn is_valid_ogg(bytes: &[u8]) -> bool {
    bytes.len() > 32 && bytes.starts_with(b"OggS")
}

pub fn default_local_asset_roots(vanilla_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(v) = vanilla_dir {
        roots.push(v.join("assets"));
    }
    let app_paths = crate::config::AppPaths::resolve();
    roots.push(crate::vanilla::resolve_vanilla_dir(&app_paths.root_dir).join("assets"));
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(appdata).join(".minecraft").join("assets"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        roots.push(
            PathBuf::from(profile)
                .join(".gradle")
                .join("caches")
                .join("fabric-loom")
                .join("assets"),
        );
    }
    roots
}

fn find_hash_object(asset_root: &Path, hash: &str) -> Option<PathBuf> {
    if hash.len() < 4 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let path = asset_root.join("objects").join(&hash[..2]).join(hash);
    if path.is_file() { Some(path) } else { None }
}

fn resolve_from_asset_roots(spec: &McSoundSpec, asset_roots: &[PathBuf]) -> Option<Vec<u8>> {
    for root in asset_roots {
        if let Some(direct) = find_hash_object(root, spec.sha1_1_21_4)
            && let Ok(bytes) = fs::read(&direct)
            && is_valid_ogg(&bytes)
        {
            return Some(bytes);
        }
    }

    for root in asset_roots {
        let indexes_dir = root.join("indexes");
        let Ok(entries) = fs::read_dir(&indexes_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            if let Some(hash) = parsed
                .get("objects")
                .and_then(|o| o.get(spec.asset_path))
                .and_then(|v| v.get("hash"))
                .and_then(|h| h.as_str())
                && let Some(obj_path) = find_hash_object(root, hash)
                && let Ok(bytes) = fs::read(&obj_path)
                && is_valid_ogg(&bytes)
            {
                return Some(bytes);
            }
        }
    }

    None
}

pub fn load_local_minecraft_sounds(sounds_dir: &Path, asset_roots: &[PathBuf]) -> MinecraftSoundsDto {
    let _ = fs::create_dir_all(sounds_dir);
    let mut sounds = BTreeMap::new();
    let mut missing_keys = Vec::new();

    for spec in MC_SOUND_SPECS {
        let cached_path = sounds_dir.join(format!("{}.ogg", spec.key));
        let mut ogg_bytes = fs::read(&cached_path).ok().filter(|b| is_valid_ogg(b));

        if ogg_bytes.is_none()
            && let Some(found) = resolve_from_asset_roots(spec, asset_roots)
        {
            let _ = fs::write(&cached_path, &found);
            ogg_bytes = Some(found);
        }

        if let Some(bytes) = ogg_bytes {
            sounds.insert(
                spec.key.to_string(),
                format!("data:audio/ogg;base64,{}", encode_base64(&bytes)),
            );
        } else {
            missing_keys.push(spec.key.to_string());
        }
    }

    MinecraftSoundsDto {
        ready_count: sounds.len(),
        total_count: MC_SOUND_SPECS.len(),
        missing_keys,
        sounds,
    }
}

pub fn fetch_missing_minecraft_sounds(sounds_dir: &Path) -> usize {
    if fs::create_dir_all(sounds_dir).is_err() {
        return 0;
    }
    let mut downloaded = 0usize;

    for spec in MC_SOUND_SPECS {
        let dest = sounds_dir.join(format!("{}.ogg", spec.key));
        if fs::read(&dest).ok().is_some_and(|b| is_valid_ogg(&b)) {
            continue;
        }
        let hash = spec.sha1_1_21_4;
        let url = format!("https://resources.download.minecraft.net/{}/{hash}", &hash[..2]);
        let tmp = sounds_dir.join(format!("{}.ogg.tmp", spec.key));
        let _ = fs::remove_file(&tmp);

        let mut cmd = std::process::Command::new("curl.exe");
        cmd.args([
            "-fsSL",
            "--retry",
            "1",
            "--connect-timeout",
            "8",
            "--max-time",
            "20",
            "-o",
        ])
        .arg(&tmp)
        .arg(&url);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Ok(status) = cmd.status()
            && status.success()
            && let Ok(bytes) = fs::read(&tmp)
            && is_valid_ogg(&bytes)
            && fs::rename(&tmp, &dest).is_ok()
        {
            downloaded += 1;
        }
        let _ = fs::remove_file(&tmp);
    }

    downloaded
}
