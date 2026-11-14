use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_template_files(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if dir == base
            && matches!(
                name_str.as_ref(),
                "build" | ".gradle" | "run" | ".idea" | ".settings" | "bin" | ".vscode"
            )
        {
            continue;
        }
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
            collect_template_files(base, &path, out);
        } else if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            let rel = path
                .strip_prefix(base)
                .expect("template file must live inside client-gradle")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, path));
        }
    }
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let gradle_dir = manifest_dir.join("client-gradle");
    println!("cargo:rerun-if-changed={}", gradle_dir.display());
    println!("cargo:rerun-if-changed=assets/default_skin.png");

    let mut files = Vec::new();
    collect_template_files(&gradle_dir, &gradle_dir, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let dest = out_dir.join("embedded_gradle.rs");

    let mut code = String::from("pub const EMBEDDED_GRADLE_FILES: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in &files {
        let abs_literal = abs.to_string_lossy().replace('\\', "/");
        code.push_str(&format!(
            "    ({rel:?}, include_bytes!({abs_literal:?})),\n"
        ));
    }
    code.push_str("];\n");

    fs::write(&dest, code).expect("failed to write embedded_gradle.rs");
}
