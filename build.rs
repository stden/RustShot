use std::env;
use std::path::PathBuf;

fn main() {
    let Some(manifest_dir) = env::var_os("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest = PathBuf::from(manifest_dir).join("rustshot.exe.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bin=RustShot=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=RustShot=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
