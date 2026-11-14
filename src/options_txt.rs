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
    fs::create_dir_all(run_dir)
        .map_err(|e| format!("Failed to create Minecraft run directory: {e}"))?;

    let options_path = run_dir.join("options.txt");
    let existing = fs::read_to_string(&options_path).unwrap_or_default();

    let fov_val = format!("{:.4}", fov_degrees_to_options_value(gfx.fov_degrees));
    let sens_val = format!("{:.4}", gfx.mouse_sensitivity.clamp(0.0, 1.0));
    let graphics_mode = if gfx.fancy_graphics { "fancy" } else { "fast" };

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
    let mut lines: Vec<String> = Vec::new();

    for raw_line in existing.lines() {
        let trimmed = raw_line.trim_end_matches('\r');
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once(':')
            && let Some(idx) = managed.iter().position(|(k, _)| *k == key)
        {
            if !applied[idx] {
                lines.push(format!("{}:{}", managed[idx].0, managed[idx].1));
                applied[idx] = true;
            }
            continue;
        }
        lines.push(trimmed.to_string());
    }

    for (idx, (key, val)) in managed.iter().enumerate() {
        if !applied[idx] {
            lines.push(format!("{key}:{val}"));
        }
    }

    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }

    fs::write(&options_path, output)
        .map_err(|e| format!("Failed to write Minecraft options.txt: {e}"))
}
