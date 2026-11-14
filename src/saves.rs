use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Serialize)]
pub struct MinecraftSaveSummary {
    pub id: String,
    pub modified_epoch_secs: u64,
    pub size_bytes: u64,
}

pub fn validate_save_id(save_id: &str) -> Result<(), String> {
    if save_id.is_empty()
        || save_id.len() > 128
        || save_id.contains('/')
        || save_id.contains('\\')
        || save_id.contains("..")
        || save_id.contains(':')
    {
        return Err("Invalid world folder name.".to_string());
    }
    Ok(())
}

pub fn resolve_save_dir(saves_root: &Path, save_id: &str) -> Result<PathBuf, String> {
    validate_save_id(save_id)?;

    let candidate = saves_root.join(save_id);
    if !candidate.join("level.dat").is_file() {
        return Err("Selected Minecraft world (level.dat) was not found.".to_string());
    }

    let canon_root = fs::canonicalize(saves_root)
        .map_err(|e| format!("Failed to verify saves directory: {e}"))?;
    let canon_dir = fs::canonicalize(&candidate)
        .map_err(|e| format!("Failed to verify world directory: {e}"))?;

    if !canon_dir.starts_with(&canon_root) || canon_dir == canon_root {
        return Err("World path is outside the security boundary.".to_string());
    }

    Ok(canon_dir)
}

pub fn list_saves(saves_root: &Path) -> Result<Vec<MinecraftSaveSummary>, String> {
    if !saves_root.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(saves_root)
        .map_err(|e| format!("Failed to read Minecraft saves directory: {e}"))?;

    let mut list = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let level_dat = path.join("level.dat");
        if !level_dat.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if validate_save_id(name).is_err() {
            continue;
        }
        let modified_epoch_secs = fs::metadata(&level_dat)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        list.push(MinecraftSaveSummary {
            id: name.to_string(),
            modified_epoch_secs,
            size_bytes: dir_size(&path),
        });
    }

    list.sort_by(|a, b| {
        b.modified_epoch_secs
            .cmp(&a.modified_epoch_secs)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(list)
}

pub fn delete_save(saves_root: &Path, save_id: &str) -> Result<(), String> {
    let dir = resolve_save_dir(saves_root, save_id)?;
    fs::remove_dir_all(dir).map_err(|e| format!("Failed to delete world directory: {e}"))
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            total += dir_size(&p);
        } else if let Ok(meta) = entry.metadata() {
            total += meta.len();
        }
    }
    total
}
