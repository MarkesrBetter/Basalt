use std::fs;
use std::path::Path;

use crate::config::GraphicsSettings;

/// Converts Minecraft's FOV slider scale (`70 deg` = `0.0`, `110 deg` = `1.0`, `30 deg` = `-1.0`).
pub fn fov_degrees_to_options_value(degrees: f32) -> f32 {
    ((degrees.clamp(30.0, 110.0) - 70.0) / 40.0).clamp(-1.0, 1.0)
}

/// Writes Basalt's hardware-tuned graphics parameters into Minecraft `run/options.txt`
/// while preserving unrelated user lines (keybinds, sound levels, language, packs).
pub fn sync_minecraft_options(run_dir: &Path, gfx: &GraphicsSettings) -> Result<(), String> {
    sync_minecraft_options_with_pack(run_dir, gfx, None)
}

fn merge_resource_packs(raw: &str, pack_id: &str) -> String {
    let mut packs: Vec<String> = serde_json::from_str(raw.trim()).unwrap_or_default();
    packs.retain(|entry| entry != pack_id);
    if !packs.iter().any(|entry| entry == "vanilla") {
        packs.insert(0, "vanilla".to_string());
    }
    packs.push(pack_id.to_string());
    serde_json::to_string(&packs).unwrap_or_else(|_| format!("[\"vanilla\",\"{pack_id}\"]"))
}

/// Syncs graphics options and optionally guarantees `skin_pack_id` (for example
/// `"file/basalt_skin.zip"`) is enabled in `resourcePacks`.
pub fn sync_minecraft_options_with_pack(
    run_dir: &Path,
    gfx: &GraphicsSettings,
    skin_pack_id: Option<&str>,
) -> Result<(), String> {
    fs::create_dir_all(run_dir)
        .map_err(|e| format!("Failed to create Minecraft run directory: {e}"))?;

    let options_path = run_dir.join("options.txt");
    let existing = fs::read_to_string(&options_path).unwrap_or_default();

    let fov_val = format!("{:.4}", fov_degrees_to_options_value(gfx.fov_degrees));
    let sens_val = format!("{:.4}", gfx.mouse_sensitivity.clamp(0.0, 1.0));
    let graphics_mode = if gfx.fancy_graphics { "1" } else { "0" };

    let managed: [(&str, String); 9] = [
        ("renderDistance", gfx.render_distance.clamp(2, 32).to_string()),
        (
            "simulationDistance",
            gfx.simulation_distance.clamp(5, 32).to_string(),
        ),
        ("fov", fov_val),
        ("enableVsync", gfx.vsync.to_string()),
        ("maxFps", gfx.max_fps.clamp(10, 260).to_string()),
        ("mouseSensitivity", sens_val),
        ("ao", gfx.smooth_lighting.to_string()),
        ("graphicsMode", graphics_mode.to_string()),
        ("skipMultiplayerWarning", "true".to_string()),
    ];

    let mut applied = [false; 9];
    let mut wrote_resource_packs = false;
    let mut lines: Vec<String> = Vec::new();

    for raw_line in existing.lines() {
        let trimmed = raw_line.trim_end_matches('\r');
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once(':') {
            if let Some(pack_id) = skin_pack_id
                && key == "resourcePacks"
            {
                if !wrote_resource_packs {
                    lines.push(format!("resourcePacks:{}", merge_resource_packs(val, pack_id)));
                    wrote_resource_packs = true;
                }
                continue;
            }
            if let Some(idx) = managed.iter().position(|(k, _)| *k == key) {
                if !applied[idx] {
                    lines.push(format!("{}:{}", managed[idx].0, managed[idx].1));
                    applied[idx] = true;
                }
                continue;
            }
        }
        lines.push(trimmed.to_string());
    }

    for (idx, (key, val)) in managed.iter().enumerate() {
        if !applied[idx] {
            lines.push(format!("{key}:{val}"));
        }
    }
    if let Some(pack_id) = skin_pack_id
        && !wrote_resource_packs
    {
        lines.push(format!(
            "resourcePacks:{}",
            merge_resource_packs("[\"vanilla\"]", pack_id)
        ));
    }

    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }

    fs::write(&options_path, output)
        .map_err(|e| format!("Failed to write Minecraft options.txt: {e}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::sync_minecraft_options;
    use crate::config::GraphicsSettings;

    #[test]
    fn writes_numeric_graphics_mode_for_1_21_4() {
        let dir = std::env::temp_dir().join("basalt_options_txt_test");
        let _ = fs::remove_dir_all(&dir);
        let mut gfx = GraphicsSettings::default();
        gfx.fancy_graphics = true;
        sync_minecraft_options(&dir, &gfx).unwrap();
        let written = fs::read_to_string(dir.join("options.txt")).unwrap();
        assert!(written.contains("graphicsMode:1\n"));

        gfx.fancy_graphics = false;
        sync_minecraft_options(&dir, &gfx).unwrap();
        let written_fast = fs::read_to_string(dir.join("options.txt")).unwrap();
        assert!(written_fast.contains("graphicsMode:0\n"));
        let _ = fs::remove_dir_all(&dir);
    }
}
