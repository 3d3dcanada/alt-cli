use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
fn collect(base: &Path, path: &Path, out: &mut BTreeMap<String, String>) {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("source directory") {
            collect(base, &entry.expect("source entry").path(), out)
        }
    } else if path.is_file() {
        let bytes = std::fs::read(path).expect("source input");
        out.insert(
            path.strip_prefix(base)
                .expect("source relative path")
                .to_string_lossy()
                .replace('\\', "/"),
            format!("{:x}", Sha256::digest(bytes)),
        );
    }
}
fn output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|r| r.status.success())
        .map(|r| String::from_utf8_lossy(&r.stdout).trim().into())
}
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let mut inputs = BTreeMap::new();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
        "build.rs",
        "src",
        "assets",
        "prompts",
        "skills",
        ".cargo/config.toml",
    ] {
        collect(&root, &root.join(name), &mut inputs);
    }
    let mut h = Sha256::new();
    for (path, sha) in &inputs {
        h.update((path.len() as u64).to_le_bytes());
        h.update(path.as_bytes());
        h.update(sha.as_bytes());
    }
    for name in ["SOURCE_DATE_EPOCH", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    for name in [".git/HEAD", ".git/index", ".git/refs"] {
        println!("cargo:rerun-if-changed={}", root.join(name).display());
    }
    let rustc = std::env::var("RUSTC").expect("Rust compiler");
    let version = std::env::var("CARGO_PKG_VERSION").expect("version");
    let commit = output("git", &["rev-parse", "HEAD"]);
    let dirty = output("git", &["status", "--porcelain"]).map(|s| !s.is_empty());
    let release_tag = output("git", &["describe", "--tags", "--exact-match", "HEAD"])
        .filter(|tag| tag.starts_with(&format!("v{version}-beta.")) && dirty == Some(false));
    let label = release_tag.clone().unwrap_or_else(|| {
        format!(
            "{version} source {}{}",
            commit
                .as_deref()
                .map(|c| &c[..c.len().min(7)])
                .unwrap_or("unknown"),
            if dirty == Some(true) { " modified" } else { "" }
        )
    });
    println!("cargo:rustc-env=ALT_BUILD_LABEL={label}");
    let value = serde_json::json!({"schema":1,"version":version,"release_tag":release_tag,"build_label":label,"source_sha256":format!("{:x}",h.finalize()),"inputs":inputs,"commit":commit,"dirty":dirty,"rustc":output(&rustc,&["--version"]),"target":std::env::var("TARGET").expect("target"),"profile":std::env::var("PROFILE").expect("profile"),"rustflags":std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default(),"source_date_epoch":std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_|"0".into())});
    std::fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").expect("build output")).join("build-info.json"),
        serde_json::to_vec_pretty(&value).expect("build metadata"),
    )
    .expect("write build metadata");
}
