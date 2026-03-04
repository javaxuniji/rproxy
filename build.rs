use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // 告诉 Cargo，如果这个文件变了就重新编译
    println!("cargo:rerun-if-changed=wintun.dll");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_dir = manifest_dir.join("target").join(env::var("PROFILE").unwrap());

    let src = manifest_dir.join("wintun.dll");
    let dest = target_dir.join("wintun.dll");

    // 如果根目录下有 wintun.dll，就把它拷贝到输出目录 (target/debug 或 target/release)
    if src.exists() {
        fs::copy(src, dest).ok();
    }
}