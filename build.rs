use std::env;
use std::fs;
use std::path::PathBuf;
fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("lib")
        .join(&target);
    std::fs::create_dir_all(&out_dir).unwrap();
    let c_out_dir = out_dir.join("temp");
    println!("cargo:rerun-if-changed=src/c/builtins.c");
    println!("cargo:rerun-if-changed=src/c/hashmap.c");
    println!("cargo:rerun-if-changed=src/c/hashmap.h");
    println!("cargo:rerun-if-changed=src/c/builtins.h");
    println!("cargo:rerun-if-changed=src/c/stub.c");
    println!("cargo:rerun-if-changed=src/c/stub.h");

    cc::Build::new()
        .file("src/c/builtins.c")
        .file("src/c/stub.c")
        .file("src/c/hashmap.c")
        .out_dir(&c_out_dir)
        .flag("-g")
        .compile("runtime");

    //Remove build artifacts
    let _ = fs::rename(c_out_dir.join("libruntime.a"), out_dir.join("libruntime.a")).unwrap();
    let _ = fs::remove_dir_all(c_out_dir);
    /*
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
    std::fs::write("src/ffi.rs", bindings_string).unwrap();
    */
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=runtime");
    //inject environment variables
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
