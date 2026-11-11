//! Prevent an unchanged development version from restoring checkpoints built by
//! different feature semantics. Deterministic source/target identity, no timestamps.
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
fn collect(dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            collect(&path, paths);
        } else if path.extension().is_some_and(|e| e == "rs") {
            paths.push(path);
        }
    }
}
fn main() {
    let mut paths = Vec::new();
    collect(Path::new("src"), &mut paths);
    paths.extend([
        PathBuf::from("Cargo.toml"),
        PathBuf::from("build.rs"),
        PathBuf::from("../../Cargo.lock"),
        PathBuf::from("../../third_party/solidcore.lock.json"),
    ]);
    paths.sort();
    println!("cargo:rerun-if-changed=src");
    let mut hash = Sha256::new();
    hash.update(b"colocad.execution/2\0");
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path.to_string_lossy();
        let bytes = fs::read(&path).expect("source bytes");
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hash.update(env::var("TARGET").expect("target"));
    let compiler = Command::new(env::var("RUSTC").expect("rustc"))
        .arg("--version")
        .output()
        .expect("compiler version");
    assert!(compiler.status.success());
    hash.update(compiler.stdout);
    println!(
        "cargo:rustc-env=COLOCAD_EXECUTION_POLICY=sha256:{:x}",
        hash.finalize()
    );
}
