use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::JvmGcMode;
use crate::protocol::local_player_uuid_dashed;

include!(concat!(env!("OUT_DIR"), "/embedded_gradle.rs"));

#[derive(Debug, Clone, Serialize)]
pub struct GradleEnvStatus {
    pub gradle_dir: String,
    pub runs_dir: String,
    pub saves_dir: String,
    pub gradlew_ready: bool,
    pub mod_jar_built: bool,
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

pub fn is_valid_gradle_workspace(dir: &Path) -> bool {
    let wrapper = if cfg!(windows) {
        dir.join("gradlew.bat").is_file()
    } else {
        dir.join("gradlew").is_file()
    };
    wrapper && dir.join("build.gradle").is_file() && dir.join("gradle.properties").is_file()
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

pub fn ensure_embedded_gradle_workspace(target_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(target_dir)
        .map_err(|e| format!("Gradle çalışma dizini oluşturulamadı: {e}"))?;

    for (rel, bytes) in EMBEDDED_GRADLE_FILES {
        let dest = target_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gradle alt klasörü oluşturulamadı ({rel}): {e}"))?;
        }
        let needs_write = match fs::read(&dest) {
            Ok(existing) => existing.as_slice() != *bytes,
            Err(_) => true,
        };
        if needs_write {
            fs::write(&dest, bytes)
                .map_err(|e| format!("Gömülü Gradle dosyası yazılamadı ({rel}): {e}"))?;
        }
    }

    fs::canonicalize(target_dir)
        .map(strip_verbatim_prefix)
        .map_err(|e| format!("Gradle çalışma dizini çözümlenemedi: {e}"))
}

pub fn resolve_or_extract_gradle_workspace(app_root_dir: &Path) -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("BASALT_GRADLE_DIR") {
        let candidate = PathBuf::from(raw.trim());
        if is_valid_gradle_workspace(&candidate) {
            return fs::canonicalize(&candidate)
                .map(strip_verbatim_prefix)
                .map_err(|e| format!("Gradle çalışma dizini çözümlenemedi: {e}"));
        }
    }

    ensure_embedded_gradle_workspace(&app_root_dir.join("client-gradle"))
}

pub fn resolve_gradle_workspace() -> Result<PathBuf, String> {
    if let Ok(raw) = std::env::var("BASALT_GRADLE_DIR") {
        let candidate = PathBuf::from(raw.trim());
        if is_valid_gradle_workspace(&candidate) {
            return fs::canonicalize(&candidate)
                .map_err(|e| format!("Gradle çalışma dizini çözümlenemedi: {e}"));
        }
    }

    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..6 {
            if let Some(dir) = cur {
                roots.push(dir.join("client-gradle"));
                cur = dir.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = Some(cwd);
        for _ in 0..5 {
            if let Some(dir) = cur {
                roots.push(dir.join("client-gradle"));
                cur = dir.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
    }

    for candidate in roots {
        if is_valid_gradle_workspace(&candidate) {
            return fs::canonicalize(&candidate)
                .map_err(|e| format!("Gradle çalışma dizini çözümlenemedi: {e}"));
        }
    }

    Err("client-gradle dizini (gradlew.bat / build.gradle) bulunamadı. BASALT_GRADLE_DIR ayarlayın veya proje kökünden çalıştırın.".to_string())
}

fn java_executable_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
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

fn is_jdk_dir_valid(home: &Path) -> bool {
    home.join("bin").join(java_executable_name()).is_file()
}

pub fn evaluate_jdk_candidates(candidates: &[PathBuf]) -> JavaDiscovery {
    let mut best_21: Option<(u32, PathBuf, String)> = None;
    let mut best_older: Option<(u32, String)> = None;

    for dir in candidates {
        if !is_jdk_dir_valid(dir) {
            continue;
        }
        let Some((major, label)) = parse_java_major(dir) else {
            continue;
        };
        if major >= 21 {
            let score = if major == 21 { 1000 } else { 500 + major };
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

pub fn inspect_gradle_environment(gradle_dir: &Path) -> GradleEnvStatus {
    let default_runtime = crate::config::AppPaths::resolve().runtime_dir;
    inspect_gradle_environment_with_runtime(gradle_dir, Some(&default_runtime))
}

pub fn inspect_gradle_environment_with_runtime(
    gradle_dir: &Path,
    runtime_dir: Option<&Path>,
) -> GradleEnvStatus {
    let run_dir = gradle_dir.join("run");
    let saves_dir = run_dir.join("saves");
    let libs_dir = gradle_dir.join("build").join("libs");
    let mod_jar_built = fs::read_dir(&libs_dir)
        .ok()
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
            })
        })
        .unwrap_or(false);

    let discovery = probe_java21_environment(runtime_dir);
    let (java_ready, java_home, java_version_label, java_hint) = match discovery.java21 {
        Some((p, ver)) => {
            let is_portable = runtime_dir.is_some_and(|rt| p.starts_with(rt));
            let hint = if is_portable {
                "Basalt taşınabilir Java 21 kurulumu algılandı ve kullanıma hazır.".to_string()
            } else {
                "Sistemde uyumlu Java 21+ JDK kurulumu algılandı.".to_string()
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
                    format!("Eksik ({older} yetersiz)"),
                    format!(
                        "Sistemde yalnızca {older} bulundu; Minecraft 1.21.4 + Fabric için Java 21 JDK gerekir. Aşağıdan tek tıkla kurabilirsiniz."
                    ),
                )
            } else {
                (
                    "Java 21 JDK bulunamadı".to_string(),
                    "Bu bilgisayarda Java 21 JDK algılanmadı. Yönetici izni gerektirmeden Otomatik Java 21 Kur ile yükleyebilirsiniz."
                        .to_string(),
                )
            };
            (false, None, label, hint)
        }
    };

    GradleEnvStatus {
        gradle_dir: gradle_dir.to_string_lossy().into_owned(),
        runs_dir: run_dir.to_string_lossy().into_owned(),
        saves_dir: saves_dir.to_string_lossy().into_owned(),
        gradlew_ready: is_valid_gradle_workspace(gradle_dir),
        mod_jar_built,
        java_ready,
        java_home,
        java_version_label,
        java_hint,
    }
}

#[derive(Debug, Clone)]
pub struct LaunchEnvConfig {
    pub username: String,
    pub jvm_ram_mb: u32,
    pub jvm_gc: JvmGcMode,
    pub show_fps_hud: bool,
    pub skin_path: Option<PathBuf>,
    pub skin_is_slim: bool,
    pub quick_play_singleplayer: Option<String>,
    pub quick_play_multiplayer: Option<String>,
    pub java_home: Option<PathBuf>,
}

pub fn build_launch_env(cfg: &LaunchEnvConfig) -> Vec<(String, String)> {
    let mut vars = vec![
        ("BASALT_USERNAME".to_string(), cfg.username.clone()),
        (
            "BASALT_UUID".to_string(),
            local_player_uuid_dashed(&cfg.username),
        ),
        (
            "BASALT_JVM_XMX".to_string(),
            cfg.jvm_ram_mb.clamp(1024, 16384).to_string(),
        ),
        (
            "BASALT_JVM_GC".to_string(),
            match cfg.jvm_gc {
                JvmGcMode::G1GC => "G1GC".to_string(),
                JvmGcMode::ZGC => "ZGC".to_string(),
            },
        ),
        (
            "BASALT_SHOW_FPS_HUD".to_string(),
            cfg.show_fps_hud.to_string(),
        ),
        (
            "BASALT_SKIN_MODEL".to_string(),
            if cfg.skin_is_slim {
                "slim".to_string()
            } else {
                "wide".to_string()
            },
        ),
    ];

    if let Some(ref skin) = cfg.skin_path {
        vars.push((
            "BASALT_SKIN_PATH".to_string(),
            skin.to_string_lossy().into_owned(),
        ));
    }
    if let Some(ref sp) = cfg.quick_play_singleplayer {
        vars.push(("BASALT_QUICK_PLAY_SP".to_string(), sp.clone()));
    }
    if let Some(ref mp) = cfg.quick_play_multiplayer {
        vars.push(("BASALT_QUICK_PLAY_MP".to_string(), mp.clone()));
    }
    if let Some(ref jh) = cfg.java_home {
        vars.push(("JAVA_HOME".to_string(), jh.to_string_lossy().into_owned()));
    }

    vars
}

/// Pure-Rust FIPS 180-4 SHA-256 digest helper (avoids invoking PowerShell for checksum verification).
pub fn compute_sha256_hex<R: std::io::Read>(mut reader: R) -> std::io::Result<String> {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let mut block = [0u8; 64];
    let mut block_len = 0usize;
    let mut total_bits = 0u64;
    let mut buf = [0u8; 16384];

    let compress = |s: &mut [u32; 8], blk: &[u8; 64]| {
        let mut w = [0u32; 64];
        for (i, chunk) in blk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *s;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
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

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total_bits = total_bits.wrapping_add((n as u64) * 8);
        let mut offset = 0usize;
        while offset < n {
            let take = (64 - block_len).min(n - offset);
            block[block_len..block_len + take].copy_from_slice(&buf[offset..offset + take]);
            block_len += take;
            offset += take;
            if block_len == 64 {
                compress(&mut state, &block);
                block_len = 0;
            }
        }
    }

    block[block_len] = 0x80;
    block_len += 1;
    if block_len > 56 {
        block[block_len..64].fill(0);
        compress(&mut state, &block);
        block_len = 0;
    }
    block[block_len..56].fill(0);
    block[56..64].copy_from_slice(&total_bits.to_be_bytes());
    compress(&mut state, &block);

    let mut hex = String::with_capacity(64);
    for word in state {
        hex.push_str(&format!("{word:08x}"));
    }
    Ok(hex)
}

pub fn compute_file_sha256_hex(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| format!("SHA-256 için dosya açılamadı: {e}"))?;
    compute_sha256_hex(std::io::BufReader::new(file))
        .map_err(|e| format!("SHA-256 hesaplanamadı: {e}"))
}

#[cfg(test)]
mod sha256_tests {
    use super::compute_sha256_hex;

    #[test]
    fn matches_fips_180_4_vectors() {
        assert_eq!(
            compute_sha256_hex(&b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute_sha256_hex(&b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
