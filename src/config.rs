use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Ember,
    Light,
    Spruce,
    Cobalt,
    Nether,
    Frost,
    Sculk,
    End,
    Cherry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccentTone {
    #[default]
    Auto,
    Copper,
    Emerald,
    Cyan,
    Amber,
    Crimson,
    Amethyst,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CornerStyle {
    Sharp,
    #[default]
    Balanced,
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayoutMode {
    #[default]
    Sidebar,
    Topbar,
    CompactDeck,
    RightRail,
    MinimalRail,
    DeckMode,
}

/// Validates and normalizes `#RRGGBB` hex color strings to lowercase `#rrggbb`.
pub fn normalize_hex_color(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicsPreset {
    Low,
    Medium,
    High,
    Ultra,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SkinModel {
    #[default]
    Auto,
    Slim,
    Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UiLanguage {
    #[default]
    En,
    Tr,
    De,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JvmGcMode {
    G1GC,
    ZGC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub preset: GraphicsPreset,
    pub render_distance: u32,
    pub simulation_distance: u32,
    pub fov_degrees: f32,
    pub vsync: bool,
    pub max_fps: u32,
    pub mouse_sensitivity: f32,
    pub smooth_lighting: bool,
    pub fancy_graphics: bool,
    pub jvm_ram_mb: u32,
    pub jvm_gc: JvmGcMode,
    pub show_fps_hud: bool,
    pub auto_configured: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            preset: GraphicsPreset::High,
            render_distance: 12,
            simulation_distance: 10,
            fov_degrees: 75.0,
            vsync: true,
            max_fps: 165,
            mouse_sensitivity: 0.5,
            smooth_lighting: true,
            fancy_graphics: true,
            jvm_ram_mb: 3072,
            jvm_gc: JvmGcMode::G1GC,
            show_fps_hud: true,
            auto_configured: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedServer {
    pub name: String,
    pub address: String,
}

fn default_legacy_config_version() -> u8 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_legacy_config_version")]
    pub config_version: u8,
    pub username: String,
    pub skin_file: Option<PathBuf>,
    pub skin_model: SkinModel,
    pub theme: ThemeMode,
    pub ui_accent: AccentTone,
    pub custom_accent_hex: Option<String>,
    pub ui_corner: CornerStyle,
    pub ui_layout: LayoutMode,
    pub ui_compact: bool,
    pub ui_reduced_motion: bool,
    pub ui_sounds: bool,
    pub ui_language: UiLanguage,
    pub last_save_id: Option<String>,
    pub saved_servers: Vec<SavedServer>,
    pub graphics: GraphicsSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: 1,
            username: "Player".to_string(),
            skin_file: None,
            skin_model: SkinModel::Auto,
            theme: ThemeMode::Dark,
            ui_accent: AccentTone::Auto,
            custom_accent_hex: None,
            ui_corner: CornerStyle::Balanced,
            ui_layout: LayoutMode::Sidebar,
            ui_compact: false,
            ui_reduced_motion: false,
            ui_sounds: false,
            ui_language: UiLanguage::En,
            last_save_id: None,
            saved_servers: vec![SavedServer {
                name: "Local Dev Server".to_string(),
                address: "127.0.0.1:25565".to_string(),
            }],
            graphics: GraphicsSettings::default(),
        }
    }
}

pub struct AppPaths {
    pub root_dir: PathBuf,
    pub config_file: PathBuf,
    pub skins_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Self {
        let root_dir = if let Some(proj) = directories::ProjectDirs::from("dev", "Basalt", "Basalt")
        {
            proj.data_local_dir().to_path_buf()
        } else {
            PathBuf::from("basalt_data")
        };
        let config_file = root_dir.join("config.toml");
        let skins_dir = root_dir.join("skins");
        let runtime_dir = crate::gradle::resolve_ascii_safe_runtime_dir(&root_dir);
        Self {
            root_dir,
            config_file,
            skins_dir,
            runtime_dir,
        }
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        fs::create_dir_all(&self.root_dir)
            .map_err(|e| format!("Failed to create root data directory: {e}"))?;
        fs::create_dir_all(&self.skins_dir)
            .map_err(|e| format!("Failed to create skins directory: {e}"))?;
        fs::create_dir_all(&self.runtime_dir)
            .map_err(|e| format!("Failed to create runtime directory: {e}"))?;
        Ok(())
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => {
                let mut cfg: Self = toml::from_str(&text).unwrap_or_default();
                // Migrate pre-v1 configs so desktop starts windowed & quiet while Console Mode activates sound/fullscreen cleanly.
                if cfg.config_version == 0 {
                    cfg.config_version = 1;
                    cfg.ui_sounds = false;
                    if cfg.ui_layout == LayoutMode::DeckMode {
                        cfg.ui_layout = LayoutMode::Sidebar;
                    }
                    let _ = cfg.save(path);
                }
                cfg
            }
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {e}"))?;
        }
        let body =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize settings: {e}"))?;
        fs::write(path, body).map_err(|e| format!("Failed to write settings file: {e}"))?;
        Ok(())
    }
}
