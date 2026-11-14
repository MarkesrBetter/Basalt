#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use basalt::assets::{
    default_skin_png_bytes, generate_default_skin, load_skin_from_file, resolve_skin_is_slim,
};
use basalt::config::{
    AccentTone, AppConfig, AppPaths, CornerStyle, GraphicsSettings, LayoutMode, SavedServer,
    SkinModel, ThemeMode, UiLanguage, normalize_hex_color,
};
use basalt::gradle::{
    GradleEnvStatus, LaunchEnvConfig, build_launch_env, compute_file_sha256_hex,
    discover_java21_home_with_runtime, inspect_gradle_environment_with_runtime,
    resolve_gradle_user_home, resolve_or_extract_gradle_workspace, summarize_process_failure,
};
use basalt::hardware::HardwareProfile;
use basalt::options_txt::sync_minecraft_options;
use basalt::protocol::{
    LocalHandshakeResult, ServerPingInfo, ping_server_1_21_4, probe_local_dev_login_1_21_4,
};
use basalt::saves::{MinecraftSaveSummary, delete_save as remove_save_dir, list_saves as scan_saves, resolve_save_dir};
use basalt::sounds::{
    MinecraftSoundsDto, default_local_asset_roots, fetch_missing_minecraft_sounds,
    load_local_minecraft_sounds,
};

#[derive(Debug, Clone, Serialize)]
struct RunningSessionDto {
    pid: u32,
    kind: String,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
struct BootstrapDto {
    config: AppConfig,
    saves: Vec<MinecraftSaveSummary>,
    hardware: HardwareProfile,
    gradle: GradleEnvStatus,
    skin_rgba: Vec<u8>,
    skin_is_slim: bool,
    running_session: Option<RunningSessionDto>,
    hardware_notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SkinResultDto {
    rgba: Vec<u8>,
    path: Option<String>,
    skin_model: SkinModel,
    skin_is_slim: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize)]
struct SkinModelResultDto {
    skin_model: SkinModel,
    skin_is_slim: bool,
}

#[derive(Debug, Clone, Serialize)]
struct OptimizeResultDto {
    graphics: GraphicsSettings,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct LaunchLogPayload {
    line: String,
    is_err: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SessionEndedPayload {
    kind: String,
    exit_code: Option<i32>,
    error: Option<String>,
    saves: Vec<MinecraftSaveSummary>,
    gradle: GradleEnvStatus,
}

#[derive(Debug, Deserialize)]
struct ProfileInput {
    username: String,
    theme: ThemeMode,
    #[serde(default)]
    ui_accent: AccentTone,
    #[serde(default)]
    custom_accent_hex: Option<String>,
    #[serde(default)]
    ui_corner: CornerStyle,
    #[serde(default)]
    ui_layout: LayoutMode,
    ui_compact: bool,
    ui_reduced_motion: bool,
    #[serde(default)]
    ui_sounds: bool,
    #[serde(default)]
    ui_language: Option<UiLanguage>,
}

#[derive(Debug, Deserialize)]
struct LaunchInput {
    mode: String,
    save_id: Option<String>,
    server_address: Option<String>,
}

struct AppState {
    paths: AppPaths,
    gradle_dir: PathBuf,
    gradle_user_home: Option<PathBuf>,
    config: Mutex<AppConfig>,
    hardware: Mutex<HardwareProfile>,
    hardware_notice: Mutex<Option<String>>,
    running: Arc<Mutex<Option<RunningSessionDto>>>,
    sounds_fetching: Arc<AtomicBool>,
}

impl AppState {
    fn initialize() -> Result<Self, String> {
        let paths = AppPaths::resolve();
        paths.ensure_dirs()?;

        let gradle_dir = resolve_or_extract_gradle_workspace(&paths.root_dir)?;
        let gradle_user_home = resolve_gradle_user_home(&paths.root_dir);
        if let Some(ref gh) = gradle_user_home {
            fs::create_dir_all(gh)
                .map_err(|e| format!("Failed to create Gradle cache directory: {e}"))?;
        }
        let mut config = AppConfig::load(&paths.config_file);
        let hardware = HardwareProfile::detect_headless();
        let mut hardware_notice = None;

        if !config.graphics.auto_configured {
            let (recommended, reason) = hardware.recommend_settings(&config.graphics);
            config.graphics = recommended;
            if let Err(err) = config.save(&paths.config_file) {
                hardware_notice = Some(format!(
                    "{reason} (Warning: failed to save initial calibration to disk: {err})"
                ));
            } else {
                hardware_notice = Some(reason);
            }
        }

        if let Some(ref skin_path) = config.skin_file
            && !skin_path.exists()
        {
            config.skin_file = None;
            let _ = config.save(&paths.config_file);
        }

        Ok(Self {
            paths,
            gradle_dir,
            gradle_user_home,
            config: Mutex::new(config),
            hardware: Mutex::new(hardware),
            hardware_notice: Mutex::new(hardware_notice),
            running: Arc::new(Mutex::new(None)),
            sounds_fetching: Arc::new(AtomicBool::new(false)),
        })
    }

    fn saves_dir(&self) -> PathBuf {
        self.gradle_dir.join("run").join("saves")
    }

    fn run_dir(&self) -> PathBuf {
        self.gradle_dir.join("run")
    }
}

fn validate_username(raw: &str) -> Result<String, String> {
    let clean = raw.trim();
    if clean.is_empty() || clean.len() > 16 {
        return Err("Username must be between 1 and 16 characters.".to_string());
    }
    if !clean
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(
            "Username may only contain ASCII letters, digits, '_' or '-'.".to_string(),
        );
    }
    Ok(clean.to_string())
}

fn validate_graphics(mut gfx: GraphicsSettings) -> Result<GraphicsSettings, String> {
    if !gfx.fov_degrees.is_finite() || !gfx.mouse_sensitivity.is_finite() {
        return Err("Invalid numeric value in graphics settings.".to_string());
    }
    gfx.render_distance = gfx.render_distance.clamp(2, 32);
    gfx.simulation_distance = gfx.simulation_distance.clamp(5, 32);
    gfx.fov_degrees = gfx.fov_degrees.clamp(30.0, 110.0);
    gfx.max_fps = gfx.max_fps.clamp(10, 260);
    gfx.mouse_sensitivity = gfx.mouse_sensitivity.clamp(0.05, 1.0);
    gfx.jvm_ram_mb = gfx.jvm_ram_mb.clamp(1024, 16384);
    Ok(gfx)
}

fn validate_server_address(raw: &str) -> Result<String, String> {
    let clean = raw.trim();
    if clean.is_empty() || clean.len() > 128 {
        return Err("Server address cannot be empty or longer than 128 characters.".to_string());
    }
    if clean.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Server address cannot contain spaces or control characters.".to_string());
    }
    Ok(clean.to_string())
}

fn load_active_skin_image(skin_file: Option<&Path>) -> image::RgbaImage {
    if let Some(path) = skin_file
        && let Ok(img) = load_skin_from_file(path)
    {
        return img;
    }
    generate_default_skin()
}

fn ensure_launch_skin_file(paths: &AppPaths, skin_file: Option<&PathBuf>) -> Option<PathBuf> {
    if let Some(custom) = skin_file
        && custom.is_file()
    {
        return Some(custom.clone());
    }
    let default_path = paths.skins_dir.join("default_skin.png");
    let _ = fs::create_dir_all(&paths.skins_dir);
    let _ = fs::write(&default_path, default_skin_png_bytes());
    default_path.is_file().then_some(default_path)
}

fn spawn_gradle_command(
    gradle_dir: &Path,
    gradle_user_home: Option<&Path>,
    task: &str,
    env_vars: &[(String, String)],
) -> Result<Child, String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", "gradlew.bat", task, "--no-daemon", "--console=plain"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            c.creation_flags(CREATE_NO_WINDOW);
        }
        c
    } else {
        let mut c = Command::new("./gradlew");
        c.args([task, "--no-daemon", "--console=plain"]);
        c
    };

    cmd.current_dir(gradle_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for key in [
        "BASALT_USERNAME",
        "BASALT_UUID",
        "BASALT_JVM_XMX",
        "BASALT_JVM_GC",
        "BASALT_SHOW_FPS_HUD",
        "BASALT_SKIN_PATH",
        "BASALT_SKIN_MODEL",
        "BASALT_QUICK_PLAY_SP",
        "BASALT_QUICK_PLAY_MP",
        "_JAVA_OPTIONS",
        "JAVA_TOOL_OPTIONS",
        "JAVA_OPTS",
        "GRADLE_OPTS",
    ] {
        cmd.env_remove(key);
    }

    for (k, v) in env_vars {
        cmd.env(k, v);
    }
    if let Some(g_home) = gradle_user_home {
        cmd.env("GRADLE_USER_HOME", g_home);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start Gradle process: {e}"))
}

fn spawn_log_reader<R: std::io::Read + Send + 'static>(
    stream: Option<R>,
    is_err: bool,
    app: tauri::AppHandle,
    tail: Arc<Mutex<VecDeque<String>>>,
    file_writer: Arc<Mutex<Option<BufWriter<File>>>>,
) -> Option<thread::JoinHandle<()>> {
    stream.map(|s| {
        thread::spawn(move || {
            let reader = BufReader::new(s);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim_end().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(mut guard) = file_writer.lock()
                    && let Some(ref mut w) = *guard
                {
                    let prefix = if is_err { "[ERR] " } else { "" };
                    let _ = writeln!(w, "{prefix}{trimmed}");
                    let _ = w.flush();
                }
                if let Ok(mut buf) = tail.lock() {
                    if buf.len() >= 240 {
                        buf.pop_front();
                    }
                    buf.push_back(trimmed.clone());
                }
                let _ = app.emit(
                    "basalt://launch-log",
                    LaunchLogPayload {
                        line: trimmed,
                        is_err,
                    },
                );
            }
        })
    })
}

fn monitor_child_process(
    app: tauri::AppHandle,
    mut child: Child,
    kind: String,
    running_arc: Arc<Mutex<Option<RunningSessionDto>>>,
    gradle_dir: PathBuf,
    saves_dir: PathBuf,
    runtime_dir: PathBuf,
    log_file: PathBuf,
) {
    thread::spawn(move || {
        let tail = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(240)));
        if let Some(parent) = log_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let log_created = File::create(&log_file).ok().map(BufWriter::new);
        let log_available = log_created.is_some();
        let file_writer = Arc::new(Mutex::new(log_created));

        let out_handle = spawn_log_reader(
            child.stdout.take(),
            false,
            app.clone(),
            Arc::clone(&tail),
            Arc::clone(&file_writer),
        );
        let err_handle = spawn_log_reader(
            child.stderr.take(),
            true,
            app.clone(),
            Arc::clone(&tail),
            Arc::clone(&file_writer),
        );

        let status_res = child.wait();
        if let Some(h) = out_handle {
            let _ = h.join();
        }
        if let Some(h) = err_handle {
            let _ = h.join();
        }
        if let Ok(mut guard) = file_writer.lock()
            && let Some(mut w) = guard.take()
        {
            let _ = w.flush();
        }

        if let Ok(mut guard) = running_arc.lock() {
            *guard = None;
        }

        let saves = scan_saves(&saves_dir).unwrap_or_default();
        let gradle = inspect_gradle_environment_with_runtime(&gradle_dir, Some(&runtime_dir));

        let (exit_code, error) = match status_res {
            Ok(status) => {
                let code = status.code();
                if status.success() {
                    (code, None)
                } else {
                    let captured: Vec<String> = tail
                        .lock()
                        .ok()
                        .map(|buf| buf.iter().cloned().collect())
                        .unwrap_or_default();
                    let msg = summarize_process_failure(
                        &captured,
                        code,
                        log_available.then_some(log_file.as_path()),
                    );
                    (code, Some(msg))
                }
            }
            Err(e) => (None, Some(format!("Process monitor error: {e}"))),
        };

        let _ = app.emit(
            "basalt://session-ended",
            SessionEndedPayload {
                kind,
                exit_code,
                error,
                saves,
                gradle,
            },
        );
    });
}

#[tauri::command]
fn get_bootstrap_state(state: tauri::State<AppState>) -> Result<BootstrapDto, String> {
    let config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?
        .clone();
    let hardware = state
        .hardware
        .lock()
        .map_err(|_| "Failed to unlock hardware profile".to_string())?
        .clone();
    let hardware_notice = state
        .hardware_notice
        .lock()
        .map_err(|_| "Failed to unlock notice state".to_string())?
        .take();
    let running_session = state
        .running
        .lock()
        .map_err(|_| "Failed to unlock session state".to_string())?
        .clone();

    let saves = scan_saves(&state.saves_dir())?;
    let gradle = inspect_gradle_environment_with_runtime(&state.gradle_dir, Some(&state.paths.runtime_dir));
    let skin_img = load_active_skin_image(config.skin_file.as_deref());
    let skin_is_slim = resolve_skin_is_slim(config.skin_model, &skin_img);

    Ok(BootstrapDto {
        config,
        saves,
        hardware,
        gradle,
        skin_rgba: skin_img.into_raw(),
        skin_is_slim,
        running_session,
        hardware_notice,
    })
}

#[tauri::command]
fn list_saves(state: tauri::State<AppState>) -> Result<Vec<MinecraftSaveSummary>, String> {
    scan_saves(&state.saves_dir())
}

#[tauri::command]
fn select_save(state: tauri::State<AppState>, save_id: Option<String>) -> Result<(), String> {
    if let Some(ref id) = save_id {
        let _ = resolve_save_dir(&state.saves_dir(), id)?;
    }
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.last_save_id = save_id;
    config.save(&state.paths.config_file)?;
    Ok(())
}

#[tauri::command]
fn delete_save(
    state: tauri::State<AppState>,
    save_id: String,
) -> Result<Vec<MinecraftSaveSummary>, String> {
    {
        let running = state
            .running
            .lock()
            .map_err(|_| "Failed to unlock session state".to_string())?;
        if running.is_some() {
            return Err(
                "Cannot delete a world while Minecraft or Gradle is running; close the game first.".to_string(),
            );
        }
    }

    remove_save_dir(&state.saves_dir(), &save_id)?;

    let saves = scan_saves(&state.saves_dir())?;
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    if config.last_save_id.as_deref() == Some(save_id.as_str()) {
        config.last_save_id = saves.first().map(|s| s.id.clone());
        config.save(&state.paths.config_file)?;
    }

    Ok(saves)
}

#[tauri::command]
fn save_profile_settings(
    state: tauri::State<AppState>,
    input: ProfileInput,
) -> Result<AppConfig, String> {
    let username = validate_username(&input.username)?;
    let clean_hex = match input.custom_accent_hex.as_deref() {
        Some(raw) if !raw.trim().is_empty() => {
            let normalized = normalize_hex_color(raw)
                .ok_or_else(|| "Custom accent color must be a valid #RRGGBB hex code.".to_string())?;
            Some(normalized)
        }
        _ => None,
    };

    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.username = username;
    config.theme = input.theme;
    config.ui_accent = input.ui_accent;
    if let Some(hex) = clean_hex {
        config.custom_accent_hex = Some(hex);
    }
    config.ui_corner = input.ui_corner;
    config.ui_layout = input.ui_layout;
    config.ui_compact = input.ui_compact;
    config.ui_reduced_motion = input.ui_reduced_motion;
    config.config_version = 1;
    config.ui_sounds = input.ui_sounds;
    if let Some(lang) = input.ui_language {
        config.ui_language = lang;
    }
    config.save(&state.paths.config_file)?;
    Ok(config.clone())
}

#[tauri::command]
fn set_fullscreen(window: tauri::Window, fullscreen: bool) -> Result<(), String> {
    window
        .set_fullscreen(fullscreen)
        .map_err(|e| format!("Failed to set fullscreen mode: {e}"))
}

#[tauri::command]
fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window
        .minimize()
        .map_err(|e| format!("Failed to minimize window: {e}"))
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::Window) -> Result<bool, String> {
    let is_max = window.is_maximized().map_err(|e| e.to_string())?;
    if is_max {
        window
            .unmaximize()
            .map_err(|e| format!("Failed to restore window size: {e}"))?;
        Ok(false)
    } else {
        window
            .maximize()
            .map_err(|e| format!("Failed to maximize window: {e}"))?;
        Ok(true)
    }
}

#[tauri::command]
fn window_is_maximized(window: tauri::Window) -> Result<bool, String> {
    window.is_maximized().map_err(|e| e.to_string())
}

#[tauri::command]
fn window_close(window: tauri::Window) -> Result<(), String> {
    window
        .close()
        .map_err(|e| format!("Failed to close window: {e}"))
}

#[tauri::command]
fn window_start_drag(window: tauri::Window) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| format!("Failed to drag window: {e}"))
}

fn spawn_missing_sounds_fetch(
    app: Option<tauri::AppHandle>,
    sounds_dir: PathBuf,
    guard: Arc<AtomicBool>,
) {
    if guard.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(move || {
        let fetched = fetch_missing_minecraft_sounds(&sounds_dir);
        guard.store(false, Ordering::SeqCst);
        if fetched > 0
            && let Some(app_handle) = app
        {
            let _ = app_handle.emit("basalt://minecraft-sounds-ready", fetched);
        }
    });
}

#[tauri::command]
fn load_minecraft_sounds(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<MinecraftSoundsDto, String> {
    let sounds_dir = state.paths.root_dir.join("sounds");
    let roots = default_local_asset_roots(Some(&state.gradle_dir));
    let dto = load_local_minecraft_sounds(&sounds_dir, &roots);
    if !dto.missing_keys.is_empty() {
        spawn_missing_sounds_fetch(
            Some(app),
            sounds_dir,
            Arc::clone(&state.sounds_fetching),
        );
    }
    Ok(dto)
}

#[tauri::command]
fn save_graphics_settings(
    state: tauri::State<AppState>,
    graphics: GraphicsSettings,
) -> Result<AppConfig, String> {
    let validated = validate_graphics(graphics)?;
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.graphics = validated;
    config.save(&state.paths.config_file)?;
    let _ = sync_minecraft_options(&state.run_dir(), &config.graphics);
    Ok(config.clone())
}

#[tauri::command]
fn auto_optimize_graphics(state: tauri::State<AppState>) -> Result<OptimizeResultDto, String> {
    let hardware = HardwareProfile::detect_headless();
    let mut hw_guard = state
        .hardware
        .lock()
        .map_err(|_| "Failed to unlock hardware profile".to_string())?;
    *hw_guard = hardware.clone();
    drop(hw_guard);

    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    let (recommended, reason) = hardware.recommend_settings(&config.graphics);
    config.graphics = recommended.clone();
    config.save(&state.paths.config_file)?;
    let _ = sync_minecraft_options(&state.run_dir(), &config.graphics);

    Ok(OptimizeResultDto {
        graphics: recommended,
        reason,
    })
}

#[tauri::command]
async fn pick_and_import_skin(
    state: tauri::State<'_, AppState>,
) -> Result<Option<SkinResultDto>, String> {
    let picked = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Select Local Skin PNG File")
            .add_filter("PNG Image", &["png"])
            .pick_file()
    })
    .await
    .map_err(|e| format!("File dialog failed: {e}"))?;

    let Some(src_path) = picked else {
        return Ok(None);
    };

    let skins_dir = state.paths.skins_dir.clone();
    let (img, saved_path, file_label) = tauri::async_runtime::spawn_blocking(move || {
        let img = load_skin_from_file(&src_path)?;
        fs::create_dir_all(&skins_dir)
            .map_err(|e| format!("Failed to prepare skins directory: {e}"))?;
        let dest = skins_dir.join("active_skin.png");
        img.save(&dest)
            .map_err(|e| format!("Failed to save normalized skin to disk: {e}"))?;
        let file_label = src_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("skin.png")
            .to_string();
        Ok::<_, String>((img, dest, file_label))
    })
    .await
    .map_err(|e| format!("Skin loader task failed: {e}"))??;

    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    let saved_str = saved_path.to_string_lossy().into_owned();
    config.skin_file = Some(saved_path);
    config.save(&state.paths.config_file)?;
    let skin_is_slim = resolve_skin_is_slim(config.skin_model, &img);
    let model_text = if skin_is_slim { "Slim (3 px)" } else { "Classic / Wide (4 px)" };
    let note = format!("Imported {file_label} (64x64 PNG, {model_text}).");

    Ok(Some(SkinResultDto {
        rgba: img.into_raw(),
        path: Some(saved_str),
        skin_model: config.skin_model,
        skin_is_slim,
        note,
    }))
}

#[tauri::command]
fn reset_skin(state: tauri::State<AppState>) -> Result<SkinResultDto, String> {
    let active_skin = state.paths.skins_dir.join("active_skin.png");
    if active_skin.exists() {
        let _ = fs::remove_file(active_skin);
    }

    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.skin_file = None;
    config.skin_model = SkinModel::Auto;
    config.save(&state.paths.config_file)?;

    let img = generate_default_skin();
    let skin_is_slim = resolve_skin_is_slim(config.skin_model, &img);

    Ok(SkinResultDto {
        rgba: img.into_raw(),
        path: None,
        skin_model: config.skin_model,
        skin_is_slim,
        note: "Restored default Basalt Slim skin.".to_string(),
    })
}

#[tauri::command]
fn save_skin_model(
    state: tauri::State<AppState>,
    skin_model: SkinModel,
) -> Result<SkinModelResultDto, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.skin_model = skin_model;
    config.save(&state.paths.config_file)?;

    let img = load_active_skin_image(config.skin_file.as_deref());
    let skin_is_slim = resolve_skin_is_slim(skin_model, &img);
    Ok(SkinModelResultDto {
        skin_model,
        skin_is_slim,
    })
}

#[tauri::command]
fn add_server(
    state: tauri::State<AppState>,
    name: String,
    address: String,
) -> Result<Vec<SavedServer>, String> {
    let clean_addr = validate_server_address(&address)?;
    let clean_name = {
        let n = name.trim();
        if n.is_empty() {
            "Local Server".to_string()
        } else {
            n.chars().take(48).collect()
        }
    };

    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    config.saved_servers.push(SavedServer {
        name: clean_name,
        address: clean_addr,
    });
    config.save(&state.paths.config_file)?;
    Ok(config.saved_servers.clone())
}

#[tauri::command]
fn remove_server(
    state: tauri::State<AppState>,
    index: usize,
) -> Result<Vec<SavedServer>, String> {
    let mut config = state
        .config
        .lock()
        .map_err(|_| "Failed to unlock configuration".to_string())?;
    if index >= config.saved_servers.len() {
        return Err("Server entry to delete was not found.".to_string());
    }
    config.saved_servers.remove(index);
    config.save(&state.paths.config_file)?;
    Ok(config.saved_servers.clone())
}

#[tauri::command]
async fn ping_server_cmd(address: String) -> Result<ServerPingInfo, String> {
    let clean_addr = validate_server_address(&address)?;
    tauri::async_runtime::spawn_blocking(move || ping_server_1_21_4(&clean_addr))
        .await
        .map_err(|e| format!("Ping worker thread error: {e}"))?
}

#[tauri::command]
async fn probe_local_server_cmd(
    address: String,
    username: String,
) -> Result<LocalHandshakeResult, String> {
    let clean_addr = validate_server_address(&address)?;
    let clean_user = validate_username(&username)?;
    tauri::async_runtime::spawn_blocking(move || {
        probe_local_dev_login_1_21_4(&clean_addr, &clean_user)
    })
    .await
    .map_err(|e| format!("Handshake worker thread error: {e}"))?
}

#[tauri::command]
fn recheck_java_21(state: tauri::State<AppState>) -> Result<GradleEnvStatus, String> {
    Ok(inspect_gradle_environment_with_runtime(
        &state.gradle_dir,
        Some(&state.paths.runtime_dir),
    ))
}

#[tauri::command]
fn open_java_download_page() -> Result<(), String> {
    const ADOPTIUM_JDK21_URL: &str =
        "https://adoptium.net/temurin/releases/?version=21&os=windows&arch=x64&package=jdk";
    #[cfg(windows)]
    {
        Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", ADOPTIUM_JDK21_URL])
            .spawn()
            .map_err(|e| format!("Failed to open browser: {e}"))?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        Err(format!("Please install Java 21 from: {ADOPTIUM_JDK21_URL}"))
    }
}

#[cfg(windows)]
fn hide_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(windows)]
fn install_java21_blocking(app: &tauri::AppHandle, runtime_dir: &Path) -> Result<(), String> {
    let log = |line: String, is_err: bool| {
        let _ = app.emit("basalt://launch-log", LaunchLogPayload { line, is_err });
    };

    fs::create_dir_all(runtime_dir)
        .map_err(|e| format!("Failed to create runtime directory: {e}"))?;
    log(
        "Querying Adoptium official API for Eclipse Temurin JDK 21 (Windows x64)...".to_string(),
        false,
    );

    let api_url =
        "https://api.adoptium.net/v3/assets/latest/21/hotspot?os=windows&architecture=x64&image_type=jdk";
    let mut meta_cmd = Command::new("curl.exe");
    meta_cmd
        .args(["-fsSL", "--connect-timeout", "20", "--max-time", "30", api_url])
        .stdin(Stdio::null());
    hide_console_window(&mut meta_cmd);
    let meta_out = meta_cmd
        .output()
        .map_err(|e| format!("Failed to query Adoptium API via curl.exe: {e}"))?;
    if !meta_out.status.success() {
        return Err(format!(
            "Adoptium API did not respond (curl exit code: {}).",
            meta_out.status.code().unwrap_or(-1)
        ));
    }

    let releases: serde_json::Value = serde_json::from_slice(&meta_out.stdout)
        .map_err(|e| format!("Failed to parse Adoptium JSON response: {e}"))?;
    let pkg = releases
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("binary"))
        .and_then(|bin| bin.get("package"))
        .ok_or_else(|| "No JDK 21 package found in Adoptium response.".to_string())?;

    let link = pkg
        .get("link")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Adoptium package download link is missing.".to_string())?;
    let name = pkg
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("temurin-jdk-21.zip");
    let size_bytes = pkg.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
    let expected_sha = pkg.get("checksum").and_then(|v| v.as_str()).map(|s| s.to_ascii_lowercase());

    let size_mb = (size_bytes as f64) / 1_048_576.0;
    log(format!("Found package: {name} ({size_mb:.1} MB)"), false);

    let zip_path = runtime_dir.join("temurin-jdk21-tmp.zip");
    let ext_dir = runtime_dir.join("extract-tmp");
    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&ext_dir);

    log("Downloading JDK 21 archive, please wait...".to_string(), false);
    let mut dl_cmd = Command::new("curl.exe");
    dl_cmd
        .args([
            "-fL",
            "--retry",
            "2",
            "--connect-timeout",
            "20",
            "-s",
            "-S",
            "-o",
        ])
        .arg(&zip_path)
        .arg(link)
        .stdin(Stdio::null());
    hide_console_window(&mut dl_cmd);
    let dl_out = dl_cmd
        .output()
        .map_err(|e| format!("Failed to start JDK 21 download: {e}"))?;
    if !dl_out.status.success() {
        let err_tail = String::from_utf8_lossy(&dl_out.stderr).trim().to_string();
        return Err(format!(
            "Failed to download JDK 21 (code: {}). {err_tail}",
            dl_out.status.code().unwrap_or(-1)
        ));
    }

    if let Some(ref expected) = expected_sha {
        log("Verifying SHA-256 integrity checksum...".to_string(), false);
        let actual = compute_file_sha256_hex(&zip_path)?;
        if &actual != expected {
            let _ = fs::remove_file(&zip_path);
            return Err("Downloaded Java 21 archive SHA-256 checksum mismatch.".to_string());
        }
    }

    log("Extracting Java 21 archive...".to_string(), false);
    fs::create_dir_all(&ext_dir)
        .map_err(|e| format!("Failed to create temporary extraction directory: {e}"))?;
    let mut tar_cmd = Command::new("tar.exe");
    tar_cmd
        .args(["-xf"])
        .arg(&zip_path)
        .arg("-C")
        .arg(&ext_dir)
        .stdin(Stdio::null());
    hide_console_window(&mut tar_cmd);
    let tar_out = tar_cmd
        .output()
        .map_err(|e| format!("Failed to unpack archive via tar.exe: {e}"))?;
    if !tar_out.status.success() {
        let err_tail = String::from_utf8_lossy(&tar_out.stderr).trim().to_string();
        return Err(format!("Failed to extract Java 21 archive: {err_tail}"));
    }

    let jdk_src = fs::read_dir(&ext_dir)
        .map_err(|e| format!("Failed to read extracted directory: {e}"))?
        .flatten()
        .map(|entry| entry.path())
        .find(|p| p.is_dir() && p.join("bin").join("java.exe").is_file())
        .ok_or_else(|| "bin\\java.exe was not found inside the extracted archive.".to_string())?;

    let target = runtime_dir.join("jdk-21");
    let _ = fs::remove_dir_all(&target);
    fs::rename(&jdk_src, &target)
        .map_err(|e| format!("Failed to move JDK 21 into runtime directory: {e}"))?;
    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&ext_dir);

    let ver_line = fs::read_to_string(target.join("release"))
        .ok()
        .and_then(|txt| txt.lines().find(|l| l.starts_with("JAVA_VERSION=")).map(str::to_string))
        .unwrap_or_else(|| "JDK 21".to_string());
    log(
        format!("Java 21 ready ({ver_line}): {}", target.display()),
        false,
    );
    Ok(())
}

#[tauri::command]
fn install_java_21(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<RunningSessionDto, String> {
    #[cfg(not(windows))]
    {
        let _ = (app, state);
        return Err("Automatic portable Java 21 installer is currently only supported on Windows.".to_string());
    }

    #[cfg(windows)]
    {
        {
            let running = state
                .running
                .lock()
                .map_err(|_| "Failed to unlock session state".to_string())?;
            if running.is_some() {
                return Err(
                    "Another operation (Minecraft, Gradle, or Java setup) is currently running."
                        .to_string(),
                );
            }
        }

        let session = RunningSessionDto {
            pid: std::process::id(),
            kind: "java-install".to_string(),
            label: "Eclipse Temurin Java 21 Setup".to_string(),
        };

        {
            let mut running = state
                .running
                .lock()
                .map_err(|_| "Failed to unlock session state".to_string())?;
            *running = Some(session.clone());
        }

        let running_arc = Arc::clone(&state.running);
        let gradle_dir = state.gradle_dir.clone();
        let saves_dir = state.saves_dir();
        let runtime_dir = state.paths.runtime_dir.clone();

        thread::spawn(move || {
            let result = install_java21_blocking(&app, &runtime_dir);
            if let Ok(mut guard) = running_arc.lock() {
                *guard = None;
            }
            let saves = scan_saves(&saves_dir).unwrap_or_default();
            let gradle = inspect_gradle_environment_with_runtime(&gradle_dir, Some(&runtime_dir));
            let (exit_code, error) = match result {
                Ok(()) => (Some(0), None),
                Err(msg) => (Some(1), Some(msg)),
            };
            let _ = app.emit(
                "basalt://session-ended",
                SessionEndedPayload {
                    kind: "java-install".to_string(),
                    exit_code,
                    error,
                    saves,
                    gradle,
                },
            );
        });

        Ok(session)
    }
}

#[tauri::command]
fn prepare_gradle_client(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<RunningSessionDto, String> {
    {
        let running = state
            .running
            .lock()
            .map_err(|_| "Failed to unlock session state".to_string())?;
        if running.is_some() {
            return Err("A Gradle or Minecraft process is already running.".to_string());
        }
    }

    let (java_home, _) = discover_java21_home_with_runtime(&state.paths.runtime_dir).ok_or_else(|| {
        "Java 21 JDK was not found. Use 'Install Java 21' on the Play screen to set up a portable runtime."
            .to_string()
    })?;

    let env_vars = vec![("JAVA_HOME".to_string(), java_home.to_string_lossy().into_owned())];
    let child = spawn_gradle_command(
        &state.gradle_dir,
        state.gradle_user_home.as_deref(),
        "build",
        &env_vars,
    )?;

    let session = RunningSessionDto {
        pid: child.id(),
        kind: "prepare".to_string(),
        label: "Gradle Fabric 1.21.4 Prebuild".to_string(),
    };

    {
        let mut running = state
            .running
            .lock()
            .map_err(|_| "Failed to unlock session state".to_string())?;
        *running = Some(session.clone());
    }

    monitor_child_process(
        app,
        child,
        "prepare".to_string(),
        Arc::clone(&state.running),
        state.gradle_dir.clone(),
        state.saves_dir(),
        state.paths.runtime_dir.clone(),
        state.paths.root_dir.join("logs").join("last-launch.log"),
    );

    Ok(session)
}

#[tauri::command]
fn launch_client(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    input: LaunchInput,
) -> Result<RunningSessionDto, String> {
    {
        let running = state
            .running
            .lock()
            .map_err(|_| "Failed to unlock session state".to_string())?;
        if running.is_some() {
            return Err("Minecraft or Gradle is already running.".to_string());
        }
    }

    let (java_home, _) = discover_java21_home_with_runtime(&state.paths.runtime_dir).ok_or_else(|| {
        "Java 21 JDK was not found. Use 'Install Java 21' on the Play screen to install it."
            .to_string()
    })?;

    let (config_snapshot, label, quick_sp, quick_mp) = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| "Failed to unlock configuration".to_string())?;

        match input.mode.as_str() {
            "singleplayer" => {
                let save_id = input
                    .save_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "Select a world save to quick-launch singleplayer.".to_string())?
                    .to_string();
                let _ = resolve_save_dir(&state.saves_dir(), &save_id)?;
                config.last_save_id = Some(save_id.clone());
                config.save(&state.paths.config_file)?;
                (
                    config.clone(),
                    format!("Minecraft 1.21.4 ({save_id})"),
                    Some(save_id),
                    None,
                )
            }
            "multiplayer" => {
                let raw_addr = input
                    .server_address
                    .as_deref()
                    .ok_or_else(|| "No server address specified.".to_string())?;
                let addr = validate_server_address(raw_addr)?;
                (
                    config.clone(),
                    format!("Minecraft 1.21.4 ({addr})"),
                    None,
                    Some(addr),
                )
            }
            "title" => (
                config.clone(),
                "Minecraft 1.21.4 (Fabric Gradle)".to_string(),
                None,
                None,
            ),
            _ => return Err("Invalid launch mode.".to_string()),
        }
    };

    sync_minecraft_options(&state.run_dir(), &config_snapshot.graphics)?;

    let skin_img = load_active_skin_image(config_snapshot.skin_file.as_deref());
    let skin_is_slim = resolve_skin_is_slim(config_snapshot.skin_model, &skin_img);
    let skin_path = ensure_launch_skin_file(&state.paths, config_snapshot.skin_file.as_ref());

    let env_cfg = LaunchEnvConfig {
        username: config_snapshot.username.clone(),
        jvm_ram_mb: config_snapshot.graphics.jvm_ram_mb,
        jvm_gc: config_snapshot.graphics.jvm_gc,
        show_fps_hud: config_snapshot.graphics.show_fps_hud,
        skin_path,
        skin_is_slim,
        quick_play_singleplayer: quick_sp,
        quick_play_multiplayer: quick_mp,
        java_home: Some(java_home),
    };

    let env_vars = build_launch_env(&env_cfg);
    let child = spawn_gradle_command(
        &state.gradle_dir,
        state.gradle_user_home.as_deref(),
        "runClient",
        &env_vars,
    )?;

    let session = RunningSessionDto {
        pid: child.id(),
        kind: "launch".to_string(),
        label,
    };

    {
        let mut running = state
            .running
            .lock()
            .map_err(|_| "Failed to unlock session state".to_string())?;
        *running = Some(session.clone());
    }

    monitor_child_process(
        app,
        child,
        "launch".to_string(),
        Arc::clone(&state.running),
        state.gradle_dir.clone(),
        state.saves_dir(),
        state.paths.runtime_dir.clone(),
        state.paths.root_dir.join("logs").join("last-launch.log"),
    );

    Ok(session)
}

fn write_startup_log(line: &str) {
    use std::io::Write;
    let log_path = std::env::temp_dir().join("basalt-startup.log");
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(f, "{line}");
    }
}

#[cfg(windows)]
fn show_windows_alert(title: &str, body: &str, ask_yes_no: bool) -> bool {
    unsafe extern "system" {
        fn MessageBoxW(
            hwnd: *mut core::ffi::c_void,
            lp_text: *const u16,
            lp_caption: *const u16,
            u_type: u32,
        ) -> i32;
    }
    const MB_OK: u32 = 0x0000_0000;
    const MB_YESNO: u32 = 0x0000_0004;
    const MB_ICONERROR: u32 = 0x0000_0010;
    const IDYES: i32 = 6;

    let mut wide_text: Vec<u16> = body.encode_utf16().collect();
    wide_text.push(0);
    let mut wide_title: Vec<u16> = title.encode_utf16().collect();
    wide_title.push(0);
    let flags = if ask_yes_no { MB_YESNO | MB_ICONERROR } else { MB_OK | MB_ICONERROR };
    // SAFETY: Null-terminated UTF-16 buffers outlive synchronous MessageBoxW call.
    let res = unsafe {
        MessageBoxW(
            core::ptr::null_mut(),
            wide_text.as_ptr(),
            wide_title.as_ptr(),
            flags,
        )
    };
    res == IDYES
}

#[cfg(not(windows))]
fn show_windows_alert(_title: &str, body: &str, _ask_yes_no: bool) -> bool {
    eprintln!("{body}");
    false
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Basalt encountered an unexpected startup error:\n{info}");
        write_startup_log(&msg);
        show_windows_alert("Basalt Startup Error", &msg, false);
    }));

    let app_state = match AppState::initialize() {
        Ok(s) => s,
        Err(err) => {
            let msg = format!("Failed to initialize Basalt workspace:\n{err}");
            write_startup_log(&msg);
            show_windows_alert("Basalt Startup Error", &msg, false);
            std::process::exit(1);
        }
    };

    let run_res = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_state,
            list_saves,
            select_save,
            delete_save,
            save_profile_settings,
            set_fullscreen,
            window_minimize,
            window_toggle_maximize,
            window_is_maximized,
            window_close,
            window_start_drag,
            load_minecraft_sounds,
            save_graphics_settings,
            auto_optimize_graphics,
            pick_and_import_skin,
            reset_skin,
            save_skin_model,
            add_server,
            remove_server,
            ping_server_cmd,
            probe_local_server_cmd,
            prepare_gradle_client,
            launch_client,
            install_java_21,
            recheck_java_21,
            open_java_download_page,
        ])
        .setup(|app| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_title("Basalt - Minecraft 1.21.4 Gradle Client");
            }
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(err) = run_res {
        let log_note = format!("Failed to initialize Tauri/WebView2: {err}");
        write_startup_log(&log_note);
        let msg = format!(
            "Basalt could not open its window ({err}).\n\n\
             Fresh Windows installations may be missing the Microsoft Edge WebView2 Runtime.\n\
             Would you like to open the official Microsoft WebView2 download page in your browser?"
        );
        if show_windows_alert("Basalt - WebView2 Required", &msg, true) {
            let _ = Command::new("rundll32.exe")
                .args([
                    "url.dll,FileProtocolHandler",
                    "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
                ])
                .spawn();
        }
        std::process::exit(1);
    }
}
