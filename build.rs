use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
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

    let c_src = manifest_dir.join("src").join("c");

    cmake::Config::new(&c_src)
        .generator("Ninja") // always use Ninja
        .profile("Debug")
        .build();

    let out_dir = manifest_dir.join("lib").join(&target);

    if !out_dir.exists() {
        panic!(
            "Expected build output directory does not exist: {}",
            out_dir.display()
        );
    }

    let runtime = out_dir.join("libruntime.a");
    let core = out_dir.join("libcore.a");

    if !runtime.exists() {
        panic!("Missing C runtime library: {}", runtime.display());
    }
    if !core.exists() {
        panic!("Missing C core library: {}", core.display());
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());

    println!("cargo:rustc-link-arg=-lcore");
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-search=native=C:/msys64/mingw64/lib");
        println!("cargo:rustc-link-search=native=C:/msys64/mingw64/bin");
    } else {
        println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    }
    //println!("cargo:rustc-link-lib=dylib=LLVM-21");
    println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition");
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-arg=-lffi");
    }

    println!("cargo:rerun-if-changed=src/c/builtins.h");
    println!("cargo:rerun-if-changed=src/c/hashmap.h");
    println!("cargo:rerun-if-changed=src/c");

    let bindings = bindgen::Builder::default()
        .header("src/c/builtins.h")
        .header("src/c/hashmap.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("ToyArrVal")
        .allowlist_type("ToyHashMap")
        .allowlist_function("toy_.*")
        .generate()
        .expect("Unable to generate bindings");

    let bindings_string = "#![allow(unused)]\n".to_string()
        + &bindings
            .to_string()
            .replace(r#"extern "C""#, r#"unsafe extern "C""#);

    fs::write("src/ffi.rs", bindings_string).unwrap();

    println!("cargo:rustc-env=TARGET={}", target);
}
