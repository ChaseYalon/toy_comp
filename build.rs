use std::env;
use std::path::PathBuf;
use std::fs;
fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("lib")
        .join(&target);
    std::fs::create_dir_all(&out_dir).unwrap();
    let c_out_dir = out_dir.join("temp");

    cc::Build::new()
        .file("src/c/builtins.c")
        .file("src/c/stub.c")
        .out_dir(&c_out_dir)
        .compile("runtime");

    //Remove build artifacts
    let _ = fs::rename(c_out_dir.join("libruntime.a"), out_dir.join("libruntime.a")).unwrap();
    let _ = fs::remove_dir_all(c_out_dir);
        
    let bindings = bindgen::Builder::default()
        .header("src/c/builtins.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("ToyArrVal")
        .allowlist_function("toy_.*")
        .generate()
        .expect("Unable to generate bindings");

    let bindings_string = "#![allow(unused)]\n".to_string()
        + &bindings.to_string().replace(r#"extern "C""#, r#"unsafe extern "C""#);
    std::fs::write("src/ffi.rs", bindings_string).unwrap();

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=runtime");


}
