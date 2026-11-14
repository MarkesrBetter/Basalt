use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::config::{GraphicsSettings, JvmGcMode};
use crate::protocol::{MINECRAFT_1_21_4_NAME, local_player_uuid_bytes, local_player_uuid_dashed};

pub const MOJANG_VERSION_MANIFEST_V2_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const ASSET_RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";
pub const BASALT_SKIN_PACK_NAME: &str = "basalt_skin.zip";

/// Exact `net.minecraft.client.util.DefaultSkinHelper.SKINS` table order in Vanilla 1.21.4.
/// Slots `0..=8` resolve to `Model.SLIM`; slots `9..=17` resolve to `Model.WIDE`.
pub const DEFAULT_SKIN_PATHS: [&str; 18] = [
    "assets/minecraft/textures/entity/player/slim/alex.png",
    "assets/minecraft/textures/entity/player/slim/ari.png",
    "assets/minecraft/textures/entity/player/slim/efe.png",
    "assets/minecraft/textures/entity/player/slim/kai.png",
    "assets/minecraft/textures/entity/player/slim/makena.png",
    "assets/minecraft/textures/entity/player/slim/noor.png",
    "assets/minecraft/textures/entity/player/slim/steve.png",
    "assets/minecraft/textures/entity/player/slim/sunny.png",
    "assets/minecraft/textures/entity/player/slim/zuri.png",
    "assets/minecraft/textures/entity/player/wide/alex.png",
    "assets/minecraft/textures/entity/player/wide/ari.png",
    "assets/minecraft/textures/entity/player/wide/efe.png",
    "assets/minecraft/textures/entity/player/wide/kai.png",
    "assets/minecraft/textures/entity/player/wide/makena.png",
    "assets/minecraft/textures/entity/player/wide/noor.png",
    "assets/minecraft/textures/entity/player/wide/steve.png",
    "assets/minecraft/textures/entity/player/wide/sunny.png",
    "assets/minecraft/textures/entity/player/wide/zuri.png",
];

#[derive(Debug, Clone, Serialize)]
pub struct VanillaEnvStatus {
    pub vanilla_dir: String,
    pub runs_dir: String,
    pub saves_dir: String,
    pub client_ready: bool,
    pub assets_ready: bool,
    pub java_ready: bool,
    pub java_home: Option<String>,
    pub java_version_label: String,
    pub java_hint: String,
}

#[derive(Debug, Clone, Default)]
pub struct JavaDiscovery {
    pub java21: Option<(PathBuf, String)>,
    pub older_detected: Option<String>,
}

fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\")
            && !stripped.starts_with("UNC\\")
        {
            return PathBuf::from(stripped);
        }
    }
    path
}

/// Returns true when every character in the path is strict 7-bit US-ASCII.
pub fn is_ascii_path(path: &Path) -> bool {
    path.to_str().is_some_and(|s| s.is_ascii())
}

fn ascii_fingerprint_12(seed: &Path) -> String {
    let normalized = seed.to_string_lossy().to_ascii_lowercase();
    compute_sha256_hex(std::io::Cursor::new(normalized.as_bytes()))
        .map(|h| h[..12].to_string())
        .unwrap_or_else(|_| "default".to_string())
}

fn ascii_root_candidates(fingerprint: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    #[cfg(windows)]
    {
        for key in ["PUBLIC", "ProgramData", "ALLUSERSPROFILE"] {
            if let Some(val) = std::env::var_os(key) {
                let raw = val.to_string_lossy().trim().to_string();
                if !raw.is_empty() {
                    out.push(PathBuf::from(raw).join("Basalt").join(fingerprint));
                }
            }
        }
        let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        let clean_drive = sys_drive.trim().trim_end_matches(['\\', '/']);
        if !clean_drive.is_empty() {
            out.push(PathBuf::from(format!("{clean_drive}\\Basalt\\{fingerprint}")));
        }
    }
    #[cfg(not(windows))]
    {
        out.push(std::env::temp_dir().join(format!("basalt-{fingerprint}")));
        out.push(PathBuf::from(format!("/tmp/basalt-{fingerprint}")));
    }
    out
}

fn is_writable_ascii_dir(dir: &Path) -> bool {
    if !is_ascii_path(dir) || fs::create_dir_all(dir).is_err() {
        return false;
    }
    let resolved = fs::canonicalize(dir).map(strip_verbatim_prefix);
    let Ok(canonical) = resolved else {
        return false;
    };
    if !is_ascii_path(&canonical) {
        return false;
    }
    let probe = canonical.join(format!(".basalt-probe-{}", std::process::id()));
    if fs::write(&probe, b"ok").is_err() {
        return false;
    }
    let _ = fs::remove_file(&probe);
    true
}

fn pick_writable_ascii_base_dir(candidates: &[PathBuf]) -> Option<PathBuf> {
    for candidate in candidates {
        if is_writable_ascii_dir(candidate) {
            return fs::canonicalize(candidate)
                .map(strip_verbatim_prefix)
                .ok()
                .filter(|p| is_ascii_path(p));
        }
    }
    None
}

/// Returns an ASCII-safe base directory for JVM/Vanilla libraries when `seed_dir`
/// contains non-ASCII characters (for example Turkish `ı` in `%LOCALAPPDATA%`).
pub fn resolve_ascii_safe_base_dir(seed_dir: &Path) -> Option<PathBuf> {
    if is_ascii_path(seed_dir) {
        return Some(seed_dir.to_path_buf());
    }
    let fp = ascii_fingerprint_12(seed_dir);
    let candidates = ascii_root_candidates(&fp);
    pick_writable_ascii_base_dir(&candidates)
}

pub fn resolve_ascii_safe_runtime_dir(app_root_dir: &Path) -> PathBuf {
    if is_ascii_path(app_root_dir) {
        app_root_dir.join("runtime")
    } else if let Some(base) = resolve_ascii_safe_base_dir(app_root_dir) {
        base.join("runtime")
    } else {
        app_root_dir.join("runtime")
    }
}

pub fn resolve_vanilla_dir(app_root_dir: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("BASALT_VANILLA_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if is_ascii_path(app_root_dir) {
        app_root_dir.join("vanilla")
    } else if let Some(base) = resolve_ascii_safe_base_dir(app_root_dir) {
        base.join("vanilla")
    } else {
        app_root_dir.join("vanilla")
    }
}

/// Migrates any saves created in older Fabric/Gradle `client-gradle/run/saves` directories
/// into the Vanilla `run/saves` folder without overwriting existing worlds.
pub fn migrate_legacy_client_gradle_saves(app_root_dir: &Path, target_saves_dir: &Path) {
    let mut legacy_dirs = vec![
        app_root_dir.join("client-gradle").join("run").join("saves"),
    ];
    if !is_ascii_path(app_root_dir) {
        let fp = ascii_fingerprint_12(app_root_dir);
        for base in ascii_root_candidates(&fp) {
            legacy_dirs.push(base.join("client-gradle").join("run").join("saves"));
        }
    }

    for src_root in legacy_dirs {
        let Ok(entries) = fs::read_dir(&src_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let src = entry.path();
            if !src.is_dir() || !src.join("level.dat").is_file() {
                continue;
            }
            let Some(folder_name) = src.file_name() else {
                continue;
            };
            let dest = target_saves_dir.join(folder_name);
            if dest.exists() {
                continue;
            }
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::rename(&src, &dest).is_err() {
                let _ = copy_dir_recursive(&src, &dest);
            }
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

fn compiler_executable_name() -> &'static str {
    if cfg!(windows) { "javac.exe" } else { "javac" }
}

fn parse_java_major(home: &Path) -> Option<(u32, String)> {
    let release_path = home.join("release");
    if let Ok(content) = fs::read_to_string(release_path) {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("JAVA_VERSION=") {
                let ver = rest.trim().trim_matches('"');
                let first = ver.split(['.', '-', '_']).next()?;
                if let Ok(major) = first.parse::<u32>() {
                    return Some((major, ver.to_string()));
                }
            }
        }
    }
    let dir_name = home.file_name()?.to_str()?;
    for digits in dir_name.split(|c: char| !c.is_ascii_digit()) {
        if !digits.is_empty()
            && let Ok(n) = digits.parse::<u32>()
            && (17..=35).contains(&n)
        {
            return Some((n, dir_name.to_string()));
        }
    }
    None
}

fn is_java_home_valid(home: &Path) -> bool {
    home.join("bin").join(java_executable_name()).is_file()
}

pub fn evaluate_jdk_candidates(candidates: &[PathBuf]) -> JavaDiscovery {
    let mut best_21: Option<(u32, PathBuf, String)> = None;
    let mut best_older: Option<(u32, String)> = None;

    for dir in candidates {
        if !is_java_home_valid(dir) {
            continue;
        }
        let Some((major, label)) = parse_java_major(dir) else {
            continue;
        };
        let has_javac = dir.join("bin").join(compiler_executable_name()).is_file();
        if (21..=25).contains(&major) {
            let jdk_bonus = if has_javac { 2000 } else { 0 };
            let version_score = if major == 21 { 1000 } else { 500 + (30 - major) };
            let score = jdk_bonus + version_score;
            if best_21.as_ref().is_none_or(|(b_score, _, _)| score > *b_score) {
                best_21 = Some((score, strip_verbatim_prefix(dir.clone()), label));
            }
        } else if best_older.as_ref().is_none_or(|(b_major, _)| major > *b_major) {
            best_older = Some((major, label));
        }
    }

    JavaDiscovery {
        java21: best_21.map(|(_, path, label)| (path, label)),
        older_detected: best_older.map(|(_, label)| format!("Java {label}")),
    }
}

pub fn probe_java21_environment(runtime_dir: Option<&Path>) -> JavaDiscovery {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(rt) = runtime_dir
        && let Ok(entries) = fs::read_dir(rt)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                candidates.push(path.clone());
                if let Ok(sub_entries) = fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        if sub.path().is_dir() {
                            candidates.push(sub.path());
                        }
                    }
                }
            }
        }
    }

    if let Ok(env_jh) = std::env::var("JAVA_HOME") {
        candidates.push(PathBuf::from(env_jh.trim()));
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path_var) {
            if entry.join(java_executable_name()).is_file()
                && let Some(home) = entry.parent()
            {
                candidates.push(home.to_path_buf());
            }
        }
    }

    #[cfg(windows)]
    {
        let mut search_roots = Vec::new();
        for env_key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA", "USERPROFILE"] {
            if let Ok(val) = std::env::var(env_key) {
                let p = PathBuf::from(val);
                search_roots.push(p.join("Eclipse Adoptium"));
                search_roots.push(p.join("Java"));
                search_roots.push(p.join("Microsoft"));
                search_roots.push(p.join("Zulu"));
                search_roots.push(p.join("BellSoft"));
                search_roots.push(p.join("Amazon Corretto"));
                search_roots.push(p.join("Semeru"));
                search_roots.push(p.join("Programs").join("Eclipse Adoptium"));
                search_roots.push(p.join(".jdks"));
            }
        }
        for root in search_roots {
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        candidates.push(path);
                    }
                }
            }
        }
    }

    evaluate_jdk_candidates(&candidates)
}

pub fn discover_java21_home() -> Option<(PathBuf, String)> {
    let default_runtime = crate::config::AppPaths::resolve().runtime_dir;
    probe_java21_environment(Some(&default_runtime)).java21
}

pub fn discover_java21_home_with_runtime(runtime_dir: &Path) -> Option<(PathBuf, String)> {
    probe_java21_environment(Some(runtime_dir)).java21
}

pub fn inspect_vanilla_environment_with_runtime(
    vanilla_dir: &Path,
    run_dir: &Path,
    runtime_dir: Option<&Path>,
) -> VanillaEnvStatus {
    let saves_dir = run_dir.join("saves");
    let client_ready = check_vanilla_client_ready(vanilla_dir);
    let assets_ready = check_vanilla_assets_ready(vanilla_dir);

    let discovery = probe_java21_environment(runtime_dir);
    let (java_ready, java_home, java_version_label, java_hint) = match discovery.java21 {
        Some((p, ver)) => {
            let is_portable = runtime_dir.is_some_and(|rt| p.starts_with(rt));
            let hint = if is_portable {
                "Portable Basalt Java 21 installation detected and ready.".to_string()
            } else {
                "Compatible system Java 21+ runtime detected.".to_string()
            };
            (
                true,
                Some(p.to_string_lossy().into_owned()),
                format!("Java {ver}"),
                hint,
            )
        }
        None => {
            let (label, hint) = if let Some(older) = discovery.older_detected {
                (
                    format!("Missing ({older} too old)"),
                    format!(
                        "Only {older} was found on this system; Vanilla Minecraft 1.21.4 requires Java 21+. Install it in one click below."
                    ),
                )
            } else {
                (
                    "Java 21 runtime not found".to_string(),
                    "No Java 21 runtime was detected on this PC. Click Install Java 21 below to set up a portable runtime without administrator rights."
                        .to_string(),
                )
            };
            (false, None, label, hint)
        }
    };

    VanillaEnvStatus {
        vanilla_dir: vanilla_dir.to_string_lossy().into_owned(),
        runs_dir: run_dir.to_string_lossy().into_owned(),
        saves_dir: saves_dir.to_string_lossy().into_owned(),
        client_ready,
        assets_ready,
        java_ready,
        java_home,
        java_version_label,
        java_hint,
    }
}

#[derive(Debug, Deserialize)]
struct VersionManifestV2 {
    versions: Vec<VersionManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionManifestEntry {
    id: String,
    url: String,
    sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangArtifactInfo {
    #[serde(default)]
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangVersionDownloads {
    pub client: MojangArtifactInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangAssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLoggingClient {
    pub file: MojangLoggingFile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLoggingSection {
    pub client: Option<MojangLoggingClient>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangRuleOs {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangRule {
    pub action: String,
    pub os: Option<MojangRuleOs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLibDownloads {
    pub artifact: Option<MojangArtifactInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MojangLibrary {
    pub name: String,
    pub downloads: MojangLibDownloads,
    #[serde(default)]
    pub rules: Vec<MojangRule>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MojangVersionManifest {
    pub id: String,
    pub main_class: String,
    pub asset_index: MojangAssetIndexInfo,
    pub downloads: MojangVersionDownloads,
    pub logging: Option<MojangLoggingSection>,
    pub libraries: Vec<MojangLibrary>,
}

#[derive(Debug, Deserialize)]
struct MojangAssetObjectEntry {
    hash: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct MojangAssetIndexFile {
    objects: HashMap<String, MojangAssetObjectEntry>,
}

#[derive(Debug, Clone)]
struct DownloadTask {
    url: String,
    dest: PathBuf,
    sha1: String,
    size: u64,
    label: String,
}

fn current_mojang_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn current_mojang_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

/// Evaluates Mojang library `rules` + native classifier suffixes for a target OS/architecture.
pub fn is_library_allowed_for_target(
    lib_name: &str,
    rules: &[MojangRule],
    os_name: &str,
    arch: &str,
) -> bool {
    let mut allowed = rules.is_empty();
    for rule in rules {
        let os_matches = match rule.os {
            None => true,
            Some(ref os) => {
                let name_ok = os.name.as_deref().is_none_or(|n| n == os_name);
                let arch_ok = os.arch.as_deref().is_none_or(|a| match a {
                    "x86_64" | "amd64" | "x64" => arch == "x64",
                    "x86" => arch == "x86",
                    "arm64" | "aarch64" => arch == "arm64",
                    other => other == arch,
                });
                name_ok && arch_ok
            }
        };
        if os_matches {
            allowed = rule.action == "allow";
        }
    }
    if !allowed {
        return false;
    }

    if os_name == "windows" {
        match arch {
            "x64" => {
                if lib_name.ends_with(":natives-windows-x86")
                    || lib_name.ends_with(":natives-windows-arm64")
                {
                    return false;
                }
            }
            "arm64" => {
                if lib_name.ends_with(":natives-windows-x86")
                    || lib_name.ends_with(":natives-windows")
                {
                    return false;
                }
            }
            "x86" => {
                if lib_name.ends_with(":natives-windows-arm64")
                    || lib_name.ends_with(":natives-windows")
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn version_json_path(vanilla_dir: &Path) -> PathBuf {
    vanilla_dir
        .join("versions")
        .join(MINECRAFT_1_21_4_NAME)
        .join(format!("{MINECRAFT_1_21_4_NAME}.json"))
}

fn client_jar_path(vanilla_dir: &Path) -> PathBuf {
    vanilla_dir
        .join("versions")
        .join(MINECRAFT_1_21_4_NAME)
        .join(format!("{MINECRAFT_1_21_4_NAME}.jar"))
}

fn load_cached_version_manifest(vanilla_dir: &Path) -> Option<MojangVersionManifest> {
    let raw = fs::read_to_string(version_json_path(vanilla_dir)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn check_file_size(path: &Path, expected_size: u64) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == expected_size)
}

fn check_file_size_and_sha1(path: &Path, expected_size: u64, expected_sha1: &str) -> bool {
    if !check_file_size(path, expected_size) {
        return false;
    }
    compute_file_sha1_hex(path)
        .is_ok_and(|actual| actual.eq_ignore_ascii_case(expected_sha1))
}

fn check_vanilla_client_ready(vanilla_dir: &Path) -> bool {
    let Some(manifest) = load_cached_version_manifest(vanilla_dir) else {
        return false;
    };
    if !check_file_size(&client_jar_path(vanilla_dir), manifest.downloads.client.size) {
        return false;
    }
    if let Some(ref logging) = manifest.logging
        && let Some(ref client_log) = logging.client
    {
        let log_path = vanilla_dir.join("logging").join(&client_log.file.id);
        if !check_file_size(&log_path, client_log.file.size) {
            return false;
        }
    }
    let os_name = current_mojang_os_name();
    let arch = current_mojang_arch();
    for lib in &manifest.libraries {
        if !is_library_allowed_for_target(&lib.name, &lib.rules, os_name, arch) {
            continue;
        }
        let Some(ref art) = lib.downloads.artifact else {
            continue;
        };
        let Some(ref rel) = art.path else {
            continue;
        };
        let p = vanilla_dir.join("libraries").join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !check_file_size(&p, art.size) {
            return false;
        }
    }
    true
}

fn check_vanilla_assets_ready(vanilla_dir: &Path) -> bool {
    let Some(manifest) = load_cached_version_manifest(vanilla_dir) else {
        return false;
    };
    let idx_path = vanilla_dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", manifest.asset_index.id));
    if !check_file_size(&idx_path, manifest.asset_index.size) {
        return false;
    }
    let stamp = vanilla_dir
        .join("assets")
        .join(format!(".verified-{}.ok", manifest.asset_index.id));
    if stamp.is_file() {
        return true;
    }
    let Ok(idx_raw) = fs::read_to_string(&idx_path) else {
        return false;
    };
    let Ok(idx) = serde_json::from_str::<MojangAssetIndexFile>(&idx_raw) else {
        return false;
    };
    let mut seen = HashSet::new();
    for obj in idx.objects.values() {
        if obj.hash.len() < 4 || !seen.insert(obj.hash.clone()) {
            continue;
        }
        let obj_path = vanilla_dir
            .join("assets")
            .join("objects")
            .join(&obj.hash[..2])
            .join(&obj.hash);
        if !check_file_size(&obj_path, obj.size) {
            return false;
        }
    }
    true
}

#[cfg(windows)]
fn hide_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut Command) {}

fn curl_fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut cmd = Command::new("curl.exe");
    cmd.args([
        "-fsSL",
        "--retry",
        "2",
        "--connect-timeout",
        "20",
        "--max-time",
        "60",
        url,
    ])
    .stdin(Stdio::null());
    hide_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("Failed to invoke curl.exe for {url}: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!(
            "Download failed ({url}, exit code {:?}): {stderr}",
            out.status.code()
        ));
    }
    Ok(out.stdout)
}

fn escape_curl_config_value(s: &str) -> String {
    s.replace('\\', "/").replace('"', "\\\"")
}

fn download_batch_with_curl(tasks: &[&DownloadTask], temp_dir: &Path) -> Result<(), String> {
    if tasks.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(temp_dir)
        .map_err(|e| format!("Failed to create temp download dir: {e}"))?;
    let cfg_path = temp_dir.join(format!("curl-batch-{}.cfg", std::process::id()));
    let mut cfg = String::with_capacity(tasks.len() * 192);
    for t in tasks {
        if let Some(parent) = t.dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent folder for {}: {e}", t.label))?;
        }
        let _ = fs::remove_file(&t.dest);
        cfg.push_str(&format!(
            "url = \"{}\"\noutput = \"{}\"\n",
            escape_curl_config_value(&t.url),
            escape_curl_config_value(&t.dest.to_string_lossy())
        ));
    }
    fs::write(&cfg_path, cfg.as_bytes())
        .map_err(|e| format!("Failed to write curl batch config: {e}"))?;

    let mut cmd = Command::new("curl.exe");
    cmd.args([
        "-sS",
        "-fL",
        "--parallel",
        "--parallel-max",
        "16",
        "--retry",
        "2",
        "--connect-timeout",
        "20",
        "--create-dirs",
        "-K",
    ])
    .arg(&cfg_path)
    .stdin(Stdio::null());
    hide_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("Failed to invoke curl.exe parallel batch: {e}"))?;
    let _ = fs::remove_file(&cfg_path);

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!(
            "Parallel download failed (curl exit code {:?}): {stderr}",
            out.status.code()
        ));
    }

    for t in tasks {
        if !check_file_size_and_sha1(&t.dest, t.size, &t.sha1) {
            let _ = fs::remove_file(&t.dest);
            return Err(format!(
                "SHA-1 integrity check failed after downloading {}.",
                t.label
            ));
        }
    }
    Ok(())
}

fn local_seed_roots() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join(".minecraft"));
    }
    out
}

fn find_local_cached_client_jar(sha1: &str, size: u64) -> Option<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let mc_jar = PathBuf::from(appdata)
            .join(".minecraft")
            .join("versions")
            .join(MINECRAFT_1_21_4_NAME)
            .join(format!("{MINECRAFT_1_21_4_NAME}.jar"));
        if check_file_size_and_sha1(&mc_jar, size, sha1) {
            return Some(mc_jar);
        }
    }
    for env_key in ["USERPROFILE", "HOME"] {
        if let Some(profile) = std::env::var_os(env_key) {
            let loom_jar = PathBuf::from(profile)
                .join(".gradle")
                .join("caches")
                .join("fabric-loom")
                .join(MINECRAFT_1_21_4_NAME)
                .join("minecraft-client.jar");
            if check_file_size_and_sha1(&loom_jar, size, sha1) {
                return Some(loom_jar);
            }
        }
    }
    None
}

fn find_local_cached_asset(hash: &str, size: u64) -> Option<PathBuf> {
    let prefix = hash.get(..2)?;
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(PathBuf::from(appdata).join(".minecraft").join("assets"));
    }
    for env_key in ["USERPROFILE", "HOME"] {
        if let Some(profile) = std::env::var_os(env_key) {
            roots.push(
                PathBuf::from(profile)
                    .join(".gradle")
                    .join("caches")
                    .join("fabric-loom")
                    .join("assets"),
            );
        }
    }
    for root in roots {
        let candidate = root.join("objects").join(prefix).join(hash);
        if check_file_size_and_sha1(&candidate, size, hash) {
            return Some(candidate);
        }
    }
    None
}

/// Ensures official Vanilla Minecraft 1.21.4 manifest, client JAR, libraries, logging XML,
/// and assets are downloaded, SHA-1 verified, and cached inside `vanilla_dir`.
pub fn ensure_vanilla_1_21_4_ready<F>(
    vanilla_dir: &Path,
    mut log: F,
) -> Result<MojangVersionManifest, String>
where
    F: FnMut(&str, bool),
{
    fs::create_dir_all(vanilla_dir)
        .map_err(|e| format!("Failed to create Vanilla data directory: {e}"))?;
    let natives_dir = vanilla_dir.join("natives").join(MINECRAFT_1_21_4_NAME);
    let _ = fs::create_dir_all(&natives_dir);

    let vjson_path = version_json_path(vanilla_dir);
    let manifest: MojangVersionManifest = match load_cached_version_manifest(vanilla_dir) {
        Some(m) => m,
        None => {
            log(
                "Fetching official Mojang Minecraft 1.21.4 version manifest...",
                false,
            );
            let index_bytes = curl_fetch_bytes(MOJANG_VERSION_MANIFEST_V2_URL)?;
            let v2: VersionManifestV2 = serde_json::from_slice(&index_bytes)
                .map_err(|e| format!("Failed to parse Mojang version_manifest_v2.json: {e}"))?;
            let entry = v2
                .versions
                .into_iter()
                .find(|v| v.id == MINECRAFT_1_21_4_NAME)
                .ok_or_else(|| {
                    "Minecraft 1.21.4 was not found in Mojang version manifest.".to_string()
                })?;
            let ver_bytes = curl_fetch_bytes(&entry.url)?;
            let actual_sha = compute_sha1_hex(std::io::Cursor::new(&ver_bytes))
                .map_err(|e| format!("Failed to hash 1.21.4.json: {e}"))?;
            if !actual_sha.eq_ignore_ascii_case(&entry.sha1) {
                return Err("SHA-1 verification failed for Mojang 1.21.4.json.".to_string());
            }
            if let Some(parent) = vjson_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create version directory: {e}"))?;
            }
            fs::write(&vjson_path, &ver_bytes)
                .map_err(|e| format!("Failed to write 1.21.4.json: {e}"))?;
            serde_json::from_slice(&ver_bytes)
                .map_err(|e| format!("Failed to parse 1.21.4.json: {e}"))?
        }
    };

    // Asset index JSON
    let idx_path = vanilla_dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", manifest.asset_index.id));
    if !check_file_size_and_sha1(&idx_path, manifest.asset_index.size, &manifest.asset_index.sha1) {
        let mut seeded = false;
        for root in local_seed_roots() {
            let local_idx = root
                .join("assets")
                .join("indexes")
                .join(format!("{}.json", manifest.asset_index.id));
            if check_file_size_and_sha1(
                &local_idx,
                manifest.asset_index.size,
                &manifest.asset_index.sha1,
            ) {
                if let Some(parent) = idx_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&local_idx, &idx_path).is_ok() {
                    seeded = true;
                    break;
                }
            }
        }
        if !seeded {
            log(
                &format!(
                    "Downloading Minecraft asset index ({}.json)...",
                    manifest.asset_index.id
                ),
                false,
            );
            let idx_bytes = curl_fetch_bytes(&manifest.asset_index.url)?;
            let actual_sha = compute_sha1_hex(std::io::Cursor::new(&idx_bytes))
                .map_err(|e| format!("Failed to hash asset index: {e}"))?;
            if !actual_sha.eq_ignore_ascii_case(&manifest.asset_index.sha1) {
                return Err("SHA-1 verification failed for asset index JSON.".to_string());
            }
            if let Some(parent) = idx_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create asset index dir: {e}"))?;
            }
            fs::write(&idx_path, &idx_bytes)
                .map_err(|e| format!("Failed to write asset index JSON: {e}"))?;
        }
    }

    let mut missing_tasks: Vec<DownloadTask> = Vec::new();
    let mut reused_count: usize = 0;

    // 1. Client JAR
    let jar_dest = client_jar_path(vanilla_dir);
    if !check_file_size(&jar_dest, manifest.downloads.client.size) {
        if let Some(cached) = find_local_cached_client_jar(
            &manifest.downloads.client.sha1,
            manifest.downloads.client.size,
        ) {
            if let Some(parent) = jar_dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::copy(&cached, &jar_dest)
                .map_err(|e| format!("Failed to copy cached 1.21.4 client.jar: {e}"))?;
            reused_count += 1;
        } else {
            missing_tasks.push(DownloadTask {
                url: manifest.downloads.client.url.clone(),
                dest: jar_dest,
                sha1: manifest.downloads.client.sha1.clone(),
                size: manifest.downloads.client.size,
                label: "Vanilla 1.21.4 client.jar".to_string(),
            });
        }
    }

    // 2. Log4j logging config XML
    if let Some(ref logging) = manifest.logging
        && let Some(ref client_log) = logging.client
    {
        let log_dest = vanilla_dir.join("logging").join(&client_log.file.id);
        if !check_file_size(&log_dest, client_log.file.size) {
            missing_tasks.push(DownloadTask {
                url: client_log.file.url.clone(),
                dest: log_dest,
                sha1: client_log.file.sha1.clone(),
                size: client_log.file.size,
                label: client_log.file.id.clone(),
            });
        }
    }

    // 3. Libraries
    let os_name = current_mojang_os_name();
    let arch = current_mojang_arch();
    let seed_roots = local_seed_roots();
    for lib in &manifest.libraries {
        if !is_library_allowed_for_target(&lib.name, &lib.rules, os_name, arch) {
            continue;
        }
        let Some(ref art) = lib.downloads.artifact else {
            continue;
        };
        let Some(ref rel) = art.path else {
            continue;
        };
        let dest = vanilla_dir
            .join("libraries")
            .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if check_file_size(&dest, art.size) {
            continue;
        }
        let mut copied = false;
        for root in &seed_roots {
            let local_lib = root
                .join("libraries")
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
            if check_file_size_and_sha1(&local_lib, art.size, &art.sha1) {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&local_lib, &dest).is_ok() {
                    reused_count += 1;
                    copied = true;
                    break;
                }
            }
        }
        if !copied {
            missing_tasks.push(DownloadTask {
                url: art.url.clone(),
                dest,
                sha1: art.sha1.clone(),
                size: art.size,
                label: rel.clone(),
            });
        }
    }

    // 4. Asset objects
    let idx_raw = fs::read_to_string(&idx_path)
        .map_err(|e| format!("Failed to read asset index {}: {e}", idx_path.display()))?;
    let idx: MojangAssetIndexFile = serde_json::from_str(&idx_raw)
        .map_err(|e| format!("Failed to parse asset index JSON: {e}"))?;

    let mut seen_hashes = HashSet::new();
    for obj in idx.objects.values() {
        if obj.hash.len() < 4 || !seen_hashes.insert(obj.hash.clone()) {
            continue;
        }
        let dest = vanilla_dir
            .join("assets")
            .join("objects")
            .join(&obj.hash[..2])
            .join(&obj.hash);
        if check_file_size(&dest, obj.size) {
            continue;
        }
        if let Some(local_asset) = find_local_cached_asset(&obj.hash, obj.size) {
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::copy(&local_asset, &dest).is_ok() {
                reused_count += 1;
                continue;
            }
        }
        missing_tasks.push(DownloadTask {
            url: format!("{ASSET_RESOURCES_BASE_URL}/{}/{}", &obj.hash[..2], obj.hash),
            dest,
            sha1: obj.hash.clone(),
            size: obj.size,
            label: format!("asset {}", &obj.hash[..8]),
        });
    }

    if reused_count > 0 {
        log(
            &format!("Reused {reused_count} verified file(s) from local Minecraft caches."),
            false,
        );
    }

    if !missing_tasks.is_empty() {
        let total = missing_tasks.len();
        log(
            &format!(
                "Downloading {total} verified Vanilla 1.21.4 file(s) from Mojang CDN..."
            ),
            false,
        );
        let temp_dir = vanilla_dir.join("tmp");
        const CHUNK_SIZE: usize = 160;
        let mut completed = 0usize;
        for chunk in missing_tasks.chunks(CHUNK_SIZE) {
            let refs: Vec<&DownloadTask> = chunk.iter().collect();
            download_batch_with_curl(&refs, &temp_dir)?;
            completed += chunk.len();
            log(
                &format!("Vanilla 1.21.4 download progress: {completed} / {total} verified."),
                false,
            );
        }
        let _ = fs::remove_dir_all(&temp_dir);
    } else {
        log(
            "Vanilla Minecraft 1.21.4 client, libraries, and assets are verified and ready.",
            false,
        );
    }

    let stamp = vanilla_dir
        .join("assets")
        .join(format!(".verified-{}.ok", manifest.asset_index.id));
    let _ = fs::write(&stamp, b"1.21.4\n");

    Ok(manifest)
}

/// Computes `DefaultSkinHelper` slot index `0..18` matching Java `Math.floorMod(uuid.hashCode(), 18)`.
pub fn default_skin_slot(uuid_bytes: &[u8; 16]) -> usize {
    let msb = u64::from_be_bytes(uuid_bytes[0..8].try_into().unwrap());
    let lsb = u64::from_be_bytes(uuid_bytes[8..16].try_into().unwrap());
    let hilo = msb ^ lsb;
    let hash = (((hilo >> 32) as u32) ^ (hilo as u32)) as i32;
    hash.rem_euclid(18) as usize
}

/// Aligns a player UUID so Vanilla `DefaultSkinHelper.getSkinTextures(UUID)` selects
/// Slim (`slim/alex.png`, index 0) or Classic (`wide/steve.png`, index 15) deterministically
/// in singleplayer while staying RFC 4122 version/variant valid.
pub fn align_uuid_for_skin_model(base_uuid: [u8; 16], is_slim: bool) -> [u8; 16] {
    let target_slot = if is_slim { 0 } else { 15 };
    let base_lo = u32::from_be_bytes(base_uuid[12..16].try_into().unwrap());
    for delta in 0u32..128 {
        let mut candidate = base_uuid;
        candidate[12..16].copy_from_slice(&base_lo.wrapping_add(delta).to_be_bytes());
        if default_skin_slot(&candidate) == target_slot {
            return candidate;
        }
    }
    base_uuid
}

pub fn format_uuid_dashed(bytes: &[u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15],
    )
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Builds a valid uncompressed (STORED method 0) PKZIP archive in pure Rust.
pub fn build_stored_zip(entries: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut central = Vec::new();

    for &(name, data) in entries {
        let name_bytes = name.as_bytes();
        let name_len = u16::try_from(name_bytes.len())
            .map_err(|_| format!("ZIP entry path too long: {name}"))?;
        let data_len = u32::try_from(data.len())
            .map_err(|_| format!("ZIP entry exceeds 4 GiB: {name}"))?;
        let offset = u32::try_from(out.len())
            .map_err(|_| "ZIP archive offset exceeds 4 GiB".to_string())?;
        let crc = crc32_ieee(data);

        // Local file header (30 bytes + name)
        out.extend_from_slice(&0x0403_4B50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed: 2.0
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 filename flag
        out.extend_from_slice(&0u16.to_le_bytes()); // method 0: STORED
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0x5421u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(&data_len.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);

        // Central directory header (46 bytes + name)
        central.extend_from_slice(&0x0201_4B50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
        central.extend_from_slice(&0u16.to_le_bytes()); // STORED
        central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        central.extend_from_slice(&0x5421u16.to_le_bytes()); // mod date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&data_len.to_le_bytes());
        central.extend_from_slice(&data_len.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // disk number
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }

    let central_offset = u32::try_from(out.len())
        .map_err(|_| "ZIP central directory offset exceeds 4 GiB".to_string())?;
    let central_size = u32::try_from(central.len())
        .map_err(|_| "ZIP central directory exceeds 4 GiB".to_string())?;
    let count = u16::try_from(entries.len())
        .map_err(|_| "Too many ZIP entries".to_string())?;

    out.extend_from_slice(&central);

    // End of central directory record (22 bytes)
    out.extend_from_slice(&0x0605_4B50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());

    Ok(out)
}

/// Writes `run/resourcepacks/basalt_skin.zip` for pure Vanilla 1.21.4 (`pack_format: 46`)
/// overriding both the Slim/Classic aligned singleplayer slot and the offline multiplayer
/// fallback slot, returning the aligned player UUID string.
pub fn write_basalt_skin_resource_pack(
    run_dir: &Path,
    username: &str,
    skin_png_bytes: &[u8],
    is_slim: bool,
) -> Result<String, String> {
    let base_uuid = local_player_uuid_bytes(username);
    let aligned_uuid = align_uuid_for_skin_model(base_uuid, is_slim);
    let aligned_slot = default_skin_slot(&aligned_uuid);
    let offline_slot = default_skin_slot(&base_uuid);

    let mcmeta = br#"{"pack":{"pack_format":46,"description":"Basalt active skin"}}"#;
    let primary_path = DEFAULT_SKIN_PATHS[aligned_slot];
    let offline_path = DEFAULT_SKIN_PATHS[offline_slot];

    let mut entries: Vec<(&str, &[u8])> = vec![
        ("pack.mcmeta", mcmeta),
        (primary_path, skin_png_bytes),
    ];
    if offline_path != primary_path {
        entries.push((offline_path, skin_png_bytes));
    }

    let zip_bytes = build_stored_zip(&entries)?;
    let rp_dir = run_dir.join("resourcepacks");
    fs::create_dir_all(&rp_dir)
        .map_err(|e| format!("Failed to create resourcepacks directory: {e}"))?;
    let zip_path = rp_dir.join(BASALT_SKIN_PACK_NAME);
    fs::write(&zip_path, &zip_bytes)
        .map_err(|e| format!("Failed to write {}: {e}", zip_path.display()))?;

    Ok(format_uuid_dashed(&aligned_uuid))
}

#[derive(Debug, Clone)]
pub struct VanillaLaunchConfig {
    pub username: String,
    pub player_uuid: String,
    pub graphics: GraphicsSettings,
    pub quick_play_singleplayer: Option<String>,
    pub quick_play_multiplayer: Option<String>,
    pub java_home: PathBuf,
}

/// Builds direct JVM and Minecraft `net.minecraft.client.main.Main` command-line arguments
/// from the parsed Mojang 1.21.4 manifest and Basalt's hardware settings.
pub fn build_vanilla_launch_args(
    vanilla_dir: &Path,
    run_dir: &Path,
    manifest: &MojangVersionManifest,
    cfg: &VanillaLaunchConfig,
) -> Vec<String> {
    let mut args = Vec::new();
    let max_mb = cfg.graphics.jvm_ram_mb.clamp(1024, 16384);
    let init_mb = (max_mb / 2).max(512);
    args.push(format!("-Xmx{max_mb}M"));
    args.push(format!("-Xms{init_mb}M"));

    match cfg.graphics.jvm_gc {
        JvmGcMode::G1GC => {
            args.extend([
                "-XX:+UseG1GC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ]);
        }
        JvmGcMode::ZGC => {
            args.extend([
                "-XX:+UseZGC".to_string(),
                "-XX:+ZGenerational".to_string(),
            ]);
        }
    }

    if cfg!(windows) {
        args.push(
            "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"
                .to_string(),
        );
    }

    if let Some(ref logging) = manifest.logging
        && let Some(ref client_log) = logging.client
    {
        let log_xml = vanilla_dir.join("logging").join(&client_log.file.id);
        if log_xml.is_file() {
            args.push(format!("-Dlog4j.configurationFile={}", log_xml.display()));
        }
    }

    let natives_dir = vanilla_dir.join("natives").join(&manifest.id);
    let natives_str = natives_dir.to_string_lossy().into_owned();
    args.push(format!("-Djava.library.path={natives_str}"));
    args.push(format!("-Djna.tmpdir={natives_str}"));
    args.push(format!(
        "-Dorg.lwjgl.system.SharedLibraryExtractPath={natives_str}"
    ));
    args.push(format!("-Dio.netty.native.workdir={natives_str}"));
    args.push("-Dminecraft.launcher.brand=Basalt".to_string());
    args.push(format!(
        "-Dminecraft.launcher.version={}",
        env!("CARGO_PKG_VERSION")
    ));

    let os_name = current_mojang_os_name();
    let arch = current_mojang_arch();
    let mut cp_entries = Vec::new();
    for lib in &manifest.libraries {
        if !is_library_allowed_for_target(&lib.name, &lib.rules, os_name, arch) {
            continue;
        }
        let Some(ref art) = lib.downloads.artifact else {
            continue;
        };
        let Some(ref rel) = art.path else {
            continue;
        };
        cp_entries.push(
            vanilla_dir
                .join("libraries")
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
                .to_string_lossy()
                .into_owned(),
        );
    }
    cp_entries.push(client_jar_path(vanilla_dir).to_string_lossy().into_owned());

    let cp_sep = if cfg!(windows) { ";" } else { ":" };
    args.push("-cp".to_string());
    args.push(cp_entries.join(cp_sep));

    args.push(manifest.main_class.clone());

    let uuid = if cfg.player_uuid.trim().is_empty() {
        local_player_uuid_dashed(&cfg.username)
    } else {
        cfg.player_uuid.clone()
    };

    args.extend([
        "--username".to_string(),
        cfg.username.clone(),
        "--version".to_string(),
        manifest.id.clone(),
        "--gameDir".to_string(),
        run_dir.to_string_lossy().into_owned(),
        "--assetsDir".to_string(),
        vanilla_dir.join("assets").to_string_lossy().into_owned(),
        "--assetIndex".to_string(),
        manifest.asset_index.id.clone(),
        "--uuid".to_string(),
        uuid,
        "--accessToken".to_string(),
        "0".to_string(),
        "--clientId".to_string(),
        "0".to_string(),
        "--xuid".to_string(),
        "0".to_string(),
        "--userType".to_string(),
        "legacy".to_string(),
        "--versionType".to_string(),
        "release".to_string(),
    ]);

    if let Some(ref sp) = cfg.quick_play_singleplayer {
        args.push("--quickPlaySingleplayer".to_string());
        args.push(sp.clone());
    } else if let Some(ref mp) = cfg.quick_play_multiplayer {
        args.push("--quickPlayMultiplayer".to_string());
        args.push(mp.clone());
    }

    args
}

/// Spawns Vanilla Minecraft 1.21.4 directly with `java.exe` (no Gradle/Fabric intermediary).
pub fn spawn_vanilla_client(
    vanilla_dir: &Path,
    run_dir: &Path,
    manifest: &MojangVersionManifest,
    cfg: &VanillaLaunchConfig,
) -> Result<Child, String> {
    fs::create_dir_all(run_dir)
        .map_err(|e| format!("Failed to create Minecraft game directory: {e}"))?;
    let java_bin = cfg.java_home.join("bin").join(java_executable_name());
    if !java_bin.is_file() {
        return Err(format!(
            "Java executable was not found at {}",
            java_bin.display()
        ));
    }

    let args = build_vanilla_launch_args(vanilla_dir, run_dir, manifest, cfg);
    let mut cmd = Command::new(&java_bin);
    cmd.args(&args)
        .current_dir(run_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for key in [
        "_JAVA_OPTIONS",
        "JAVA_TOOL_OPTIONS",
        "JAVA_OPTS",
        "GRADLE_OPTS",
    ] {
        cmd.env_remove(key);
    }

    hide_window(&mut cmd);

    cmd.spawn()
        .map_err(|e| format!("Failed to start Vanilla Minecraft 1.21.4 process: {e}"))
}

/// Pure-Rust FIPS 180-4 SHA-1 digest helper for verifying official Mojang downloads.
pub fn compute_sha1_hex<R: Read>(mut reader: R) -> std::io::Result<String> {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];

    let compress = |block: &[u8; 64], state: &mut [u32; 5]| {
        let mut w = [0u32; 80];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) =
            (state[0], state[1], state[2], state[3], state[4]);

        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDCu32),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
    };

    let mut total_bytes: u64 = 0;
    let mut pending = [0u8; 64];
    let mut pending_len = 0usize;
    let mut buf = [0u8; 65536];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total_bytes = total_bytes
            .checked_add(n as u64)
            .ok_or_else(|| std::io::Error::other("input too large for SHA-1"))?;
        let mut slice = &buf[..n];
        if pending_len > 0 {
            let need = 64 - pending_len;
            let take = need.min(slice.len());
            pending[pending_len..pending_len + take].copy_from_slice(&slice[..take]);
            pending_len += take;
            slice = &slice[take..];
            if pending_len == 64 {
                compress(&pending, &mut h);
                pending_len = 0;
            }
        }
        while slice.len() >= 64 {
            let block: &[u8; 64] = slice[..64].try_into().expect("64-byte slice");
            compress(block, &mut h);
            slice = &slice[64..];
        }
        if !slice.is_empty() {
            pending[..slice.len()].copy_from_slice(slice);
            pending_len = slice.len();
        }
    }

    pending[pending_len] = 0x80;
    pending_len += 1;
    if pending_len > 56 {
        pending[pending_len..].fill(0);
        compress(&pending, &mut h);
        pending_len = 0;
    }
    pending[pending_len..56].fill(0);
    pending[56..64].copy_from_slice(&(total_bytes * 8).to_be_bytes());
    compress(&pending, &mut h);

    let mut hex = String::with_capacity(40);
    for word in h {
        hex.push_str(&format!("{word:08x}"));
    }
    Ok(hex)
}

pub fn compute_file_sha1_hex(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open file for SHA-1: {e}"))?;
    compute_sha1_hex(std::io::BufReader::new(file))
        .map_err(|e| format!("Failed to compute SHA-1: {e}"))
}

/// Pure-Rust FIPS 180-4 SHA-256 digest helper for verifying Adoptium JDK downloads.
pub fn compute_sha256_hex<R: Read>(mut reader: R) -> std::io::Result<String> {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let compress = |block: &[u8; 64], s: &mut [u32; 8]| {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *s;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        s[0] = s[0].wrapping_add(a);
        s[1] = s[1].wrapping_add(b);
        s[2] = s[2].wrapping_add(c);
        s[3] = s[3].wrapping_add(d);
        s[4] = s[4].wrapping_add(e);
        s[5] = s[5].wrapping_add(f);
        s[6] = s[6].wrapping_add(g);
        s[7] = s[7].wrapping_add(h);
    };

    let mut total_bytes: u64 = 0;
    let mut pending = [0u8; 64];
    let mut pending_len = 0usize;
    let mut buf = [0u8; 65536];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total_bytes = total_bytes
            .checked_add(n as u64)
            .ok_or_else(|| std::io::Error::other("input too large for SHA-256"))?;
        let mut slice = &buf[..n];
        if pending_len > 0 {
            let need = 64 - pending_len;
            let take = need.min(slice.len());
            pending[pending_len..pending_len + take].copy_from_slice(&slice[..take]);
            pending_len += take;
            slice = &slice[take..];
            if pending_len == 64 {
                compress(&pending, &mut state);
                pending_len = 0;
            }
        }
        while slice.len() >= 64 {
            let block: &[u8; 64] = slice[..64].try_into().expect("64-byte slice");
            compress(block, &mut state);
            slice = &slice[64..];
        }
        if !slice.is_empty() {
            pending[..slice.len()].copy_from_slice(slice);
            pending_len = slice.len();
        }
    }

    pending[pending_len] = 0x80;
    pending_len += 1;
    if pending_len > 56 {
        pending[pending_len..].fill(0);
        compress(&pending, &mut state);
        pending_len = 0;
    }
    pending[pending_len..56].fill(0);
    pending[56..64].copy_from_slice(&(total_bytes * 8).to_be_bytes());
    compress(&pending, &mut state);

    let mut hex = String::with_capacity(64);
    for word in state {
        hex.push_str(&format!("{word:08x}"));
    }
    Ok(hex)
}

pub fn compute_file_sha256_hex(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open file for SHA-256: {e}"))?;
    compute_sha256_hex(std::io::BufReader::new(file))
        .map_err(|e| format!("Failed to compute SHA-256: {e}"))
}

fn is_high_signal_error_line(line: &str) -> bool {
    let s = line.trim();
    s.starts_with("Caused by:")
        || s.starts_with("Exception in thread")
        || s.starts_with("Error:")
        || s.contains("OutOfMemoryError")
        || s.contains("UnsupportedClassVersionError")
        || s.contains("NoClassDefFoundError")
        || s.contains("UnsatisfiedLinkError")
        || s.contains("GLFW error")
        || s.contains("WGL:")
        || s.contains("Pixel format not accelerated")
        || s.contains("[main/ERROR]")
        || s.contains("[main/FATAL]")
        || s.contains("[Render thread/ERROR]")
        || s.contains("[Render thread/FATAL]")
        || s.contains("Could not reserve enough space")
}

fn push_unique_line(picks: &mut Vec<String>, raw: &str) {
    let clean = raw.trim().to_string();
    if !clean.is_empty() && !picks.contains(&clean) {
        picks.push(clean);
    }
}

/// Extracts high-signal JVM / Minecraft error lines from captured process output.
pub fn summarize_process_failure(
    lines: &[String],
    exit_code: Option<i32>,
    log_path: Option<&Path>,
) -> String {
    let mut picks: Vec<String> = Vec::new();

    for line in lines.iter().rev() {
        if is_high_signal_error_line(line) {
            push_unique_line(&mut picks, line);
            if picks.len() >= 3 {
                break;
            }
        }
    }
    picks.reverse();

    if picks.is_empty() {
        for line in lines.iter().rev() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                push_unique_line(&mut picks, trimmed);
                if picks.len() >= 5 {
                    break;
                }
            }
        }
        picks.reverse();
    }

    let mut summary = if picks.is_empty() {
        format!(
            "Minecraft process exited with error status (code: {}).",
            exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".into())
        )
    } else {
        picks.join(" | ")
    };

    if let Some(path) = log_path {
        summary.push_str(&format!(" (Full log: {})", path.display()));
    }
    summary
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Read};
    use std::path::{Path, PathBuf};

    use super::{
        DEFAULT_SKIN_PATHS, MojangAssetIndexInfo, MojangArtifactInfo, MojangLibDownloads,
        MojangLibrary, MojangLoggingClient, MojangLoggingFile, MojangLoggingSection, MojangRule,
        MojangRuleOs, MojangVersionDownloads, MojangVersionManifest, VanillaLaunchConfig,
        align_uuid_for_skin_model, build_stored_zip, build_vanilla_launch_args, compute_sha1_hex,
        compute_sha256_hex, default_skin_slot, evaluate_jdk_candidates, is_ascii_path,
        is_library_allowed_for_target, summarize_process_failure, write_basalt_skin_resource_pack,
    };
    use crate::config::{GraphicsSettings, JvmGcMode};
    use crate::protocol::local_player_uuid_bytes;

    #[test]
    fn matches_fips_180_4_sha1_and_sha256_vectors() {
        assert_eq!(
            compute_sha1_hex(&b""[..]).unwrap(),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            compute_sha1_hex(&b"abc"[..]).unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            compute_sha256_hex(&b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256_hex(&b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn aligns_uuid_for_slim_and_wide_default_skins() {
        let base = local_player_uuid_bytes("Markesr");
        let slim_uuid = align_uuid_for_skin_model(base, true);
        let wide_uuid = align_uuid_for_skin_model(base, false);

        assert_eq!(default_skin_slot(&slim_uuid), 0);
        assert_eq!(DEFAULT_SKIN_PATHS[default_skin_slot(&slim_uuid)], "assets/minecraft/textures/entity/player/slim/alex.png");
        assert_eq!(default_skin_slot(&wide_uuid), 15);
        assert_eq!(DEFAULT_SKIN_PATHS[default_skin_slot(&wide_uuid)], "assets/minecraft/textures/entity/player/wide/steve.png");

        // Known Java reference: UUID "ffffffff-ffff-3fff-8000-000000000001".hashCode() == -2147434495, floorMod 18 == 11
        let neg_bytes: [u8; 16] = [
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x3f, 0xff, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01,
        ];
        assert_eq!(default_skin_slot(&neg_bytes), 11);
    }

    #[test]
    fn evaluates_mojang_windows_x64_library_rules() {
        let win_rule = vec![MojangRule {
            action: "allow".to_string(),
            os: Some(MojangRuleOs {
                name: Some("windows".to_string()),
                arch: None,
            }),
        }];
        let linux_rule = vec![MojangRule {
            action: "allow".to_string(),
            os: Some(MojangRuleOs {
                name: Some("linux".to_string()),
                arch: None,
            }),
        }];

        assert!(is_library_allowed_for_target(
            "org.lwjgl:lwjgl:3.3.3:natives-windows",
            &win_rule,
            "windows",
            "x64"
        ));
        assert!(!is_library_allowed_for_target(
            "org.lwjgl:lwjgl:3.3.3:natives-windows-x86",
            &win_rule,
            "windows",
            "x64"
        ));
        assert!(!is_library_allowed_for_target(
            "org.lwjgl:lwjgl:3.3.3:natives-windows-arm64",
            &win_rule,
            "windows",
            "x64"
        ));
        assert!(!is_library_allowed_for_target(
            "com.mojang:jtracy:1.0.29:natives-linux",
            &linux_rule,
            "windows",
            "x64"
        ));
        assert!(is_library_allowed_for_target(
            "com.google.guava:guava:33.3.1-jre",
            &[],
            "windows",
            "x64"
        ));
    }

    #[test]
    fn builds_valid_skin_resource_pack_zip_and_launch_args() {
        let tmp = std::env::temp_dir().join(format!("basalt_vanilla_pack_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let run_dir = tmp.join("run");
        let vanilla_dir = tmp.join("vanilla");
        fs::create_dir_all(vanilla_dir.join("logging")).unwrap();
        fs::write(vanilla_dir.join("logging").join("client-1.21.2.xml"), b"<Configuration/>").unwrap();

        let zip_bytes = build_stored_zip(&[("a.txt", b"hello"), ("b/c.bin", b"\x01\x02")]).unwrap();
        assert!(zip_bytes.starts_with(b"PK\x03\x04"));
        let mut dec_check = Cursor::new(&zip_bytes);
        let mut head = [0u8; 4];
        dec_check.read_exact(&mut head).unwrap();
        assert_eq!(&head, b"PK\x03\x04");

        let uuid = write_basalt_skin_resource_pack(&run_dir, "Markesr", b"PNGDATA", true).unwrap();
        assert_eq!(uuid.len(), 36);
        assert!(run_dir.join("resourcepacks").join("basalt_skin.zip").is_file());

        let manifest = MojangVersionManifest {
            id: "1.21.4".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            asset_index: MojangAssetIndexInfo {
                id: "19".to_string(),
                sha1: "00".to_string(),
                size: 10,
                url: String::new(),
            },
            downloads: MojangVersionDownloads {
                client: MojangArtifactInfo {
                    path: None,
                    sha1: "00".to_string(),
                    size: 10,
                    url: String::new(),
                },
            },
            logging: Some(MojangLoggingSection {
                client: Some(MojangLoggingClient {
                    file: MojangLoggingFile {
                        id: "client-1.21.2.xml".to_string(),
                        sha1: "00".to_string(),
                        size: 10,
                        url: String::new(),
                    },
                }),
            }),
            libraries: vec![MojangLibrary {
                name: "com.google.code.gson:gson:2.11.0".to_string(),
                downloads: MojangLibDownloads {
                    artifact: Some(MojangArtifactInfo {
                        path: Some("com/google/code/gson/gson/2.11.0/gson-2.11.0.jar".to_string()),
                        sha1: "00".to_string(),
                        size: 10,
                        url: String::new(),
                    }),
                },
                rules: vec![],
            }],
        };

        let mut gfx = GraphicsSettings::default();
        gfx.jvm_ram_mb = 4096;
        gfx.jvm_gc = JvmGcMode::ZGC;

        let args = build_vanilla_launch_args(
            &vanilla_dir,
            &run_dir,
            &manifest,
            &VanillaLaunchConfig {
                username: "Markesr".to_string(),
                player_uuid: uuid.clone(),
                graphics: gfx,
                quick_play_singleplayer: Some("MyWorld".to_string()),
                quick_play_multiplayer: None,
                java_home: tmp.join("jdk-21"),
            },
        );

        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-XX:+UseZGC".to_string()));
        assert!(args.contains(&"net.minecraft.client.main.Main".to_string()));
        assert!(args.contains(&"--quickPlaySingleplayer".to_string()));
        assert!(args.contains(&"MyWorld".to_string()));
        assert!(args.contains(&uuid));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn summarizes_vanilla_jvm_errors_and_picks_jdk21() {
        let lines: Vec<String> = vec![
            "[Render thread/INFO]: Starting client".into(),
            "Caused by: java.lang.UnsupportedClassVersionError: bad version".into(),
            "[main/FATAL]: Uncaught exception".into(),
        ];
        let summary = summarize_process_failure(
            &lines,
            Some(1),
            Some(PathBuf::from("C:\\Logs\\last-launch.log").as_path()),
        );
        assert!(summary.contains("UnsupportedClassVersionError"));
        assert!(summary.contains("last-launch.log"));

        assert!(is_ascii_path(Path::new("C:\\Basalt\\vanilla")));
        assert!(!is_ascii_path(Path::new("C:\\Users\\rab\u{0131}abetl\\vanilla")));

        let tmp = std::env::temp_dir().join("basalt_vanilla_jdk_probe");
        let _ = fs::remove_dir_all(&tmp);
        let j21 = tmp.join("jdk-21");
        let j26 = tmp.join("jdk-26");
        for (dir, ver) in [(&j21, "21.0.5"), (&j26, "26.0.1")] {
            fs::create_dir_all(dir.join("bin")).unwrap();
            let java_bin = if cfg!(windows) { "java.exe" } else { "java" };
            let javac_bin = if cfg!(windows) { "javac.exe" } else { "javac" };
            fs::write(dir.join("bin").join(java_bin), b"").unwrap();
            fs::write(dir.join("bin").join(javac_bin), b"").unwrap();
            fs::write(dir.join("release"), format!("JAVA_VERSION=\"{ver}\"\n")).unwrap();
        }
        let disc = evaluate_jdk_candidates(&[j26.clone(), j21.clone()]);
        assert_eq!(disc.java21.unwrap().1, "21.0.5");
        let _ = fs::remove_dir_all(&tmp);
    }
}
