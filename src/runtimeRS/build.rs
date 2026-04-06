use std::env;
use std::fs;
use std::path::PathBuf;
use cc;
fn main() {

    println!("cargo:rerun-if-env-changed=TOY_COMP_ROOT");
    println!("cargo:rerun-if-env-changed=TOY_RUNTIME_COPY_TRIGGER");

    //this is terrible and can be deleted i think?
    if env::var("TOY_RUNTIME_COPY_TRIGGER").ok().as_deref() != Some("copy") {
        return;
    }

    let root = PathBuf::from(
        env::var("TOY_COMP_ROOT").expect("TOY_COMP_ROOT must be set when copying runtime archive"),
    );
    let target = env::var("TARGET").expect("TARGET must be set");

    let source = root
        .join("src")
        .join("runtimeRS")
        .join("target")
        .join(&target)
        .join("release")
        .join("libruntimers.a");
    let out_dir = root.join("lib").join(&target);
    let dest = out_dir.join("libruntime.a");

    if !source.exists() {
        panic!("Missing runtime staticlib: {}", source.display());
    }
    fs::create_dir_all(&out_dir).expect("Failed to create runtime output directory");
    fs::copy(&source, &dest).unwrap_or_else(|e| {
        panic!(
            "Failed copying runtime archive from {} to {}: {}",
            source.display(),
            dest.display(),
            e
        )
    });
}
