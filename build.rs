use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    cc::Build::new()
        .file("llvm_stubs.c")
        .compile("temp_llvm_thing");
    println!("cargo:rustc-link-arg=-Wl,--whole-archive");
    println!("cargo:rustc-link-arg=-ltemp_llvm_thing");
    println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
    if env::var("CARGO_CFG_RUST_ANALYZER").is_ok() {
        return;
    }
    println!(
        "LLVM_SYS_211_PREFIX: {:?}",
        env::var("LLVM_SYS_211_PREFIX").unwrap_or("LLVM_SYS_211_NOT_FOUND".to_string())
    );
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();
    if profile == "test" {
        unsafe {
            env::set_var("TOY_DEBUG", "TRUE");
        }
    } else {
        unsafe {
            env::set_var("TOY_DEBUG", "FALSE");
        }
    }
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let runtime_dir = manifest_dir.join("src").join("runtimeRS");

    let runtime_build_status = Command::new("cargo")
        .args(["build", "--release", "--target", &target])
        .current_dir(&runtime_dir)
        .env("TOY_COMP_ROOT", &manifest_dir)
        .env("TOY_RUNTIME_COPY_TRIGGER", "build")
        .status()
        .expect("Failed to build runtimeRS crate");
    if !runtime_build_status.success() {
        panic!("runtimeRS build failed with status: {}", runtime_build_status);
    }

    let runtime_copy_status = Command::new("cargo")
        .args(["build", "--release", "--target", &target])
        .current_dir(&runtime_dir)
        .env("TOY_COMP_ROOT", &manifest_dir)
        .env("TOY_RUNTIME_COPY_TRIGGER", "copy")
        .status()
        .expect("Failed to run runtimeRS copy build");
    if !runtime_copy_status.success() {
        panic!(
            "runtimeRS copy build failed with status: {}",
            runtime_copy_status
        );
    }

    let out_dir = manifest_dir.join("lib").join(&target);

    if !out_dir.exists() {
        panic!(
            "Expected build output directory does not exist: {}",
            out_dir.display()
        );
    }

    let runtime = out_dir.join("libruntime.a");

    if !runtime.exists() {
        panic!("Missing runtime library: {}", runtime.display());
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());

    if cfg!(target_os = "windows") {
        let lib_dir = manifest_dir.join("lib").join(&target);
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        
        println!("cargo:rustc-link-arg=lib/x86_64-pc-windows-gnu/cacert.o");

        println!("cargo:rustc-link-arg=-lntdll");
        println!("cargo:rustc-link-arg=-luserenv");
        println!("cargo:rustc-link-arg=-lgcc");
        println!("cargo:rustc-link-arg=-lffi");
        println!("cargo:rustc-link-arg=-lucrt");
    } else {
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    }
    println!("cargo:rustc-link-lib=dylib=LLVM-21");
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-arg=-lffi");
        println!("cargo:rustc-link-arg=-lucrt");
    }

    println!("cargo:rerun-if-changed=src/runtimeRS/Cargo.toml");
    println!("cargo:rerun-if-changed=src/runtimeRS/src");
    println!("cargo:rerun-if-changed=src/runtimeRS/build.rs");

    println!("cargo:rustc-env=TARGET={}", target);
}
