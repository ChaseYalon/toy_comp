fn main() {
    cc::Build::new()
        .file("src/c/builtins.c")
        .compile("builtins");

    let bindings = bindgen::Builder::default()
        .header("src/c/builtins.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_type("ToyArrVal")
        .allowlist_function("toy_.*") // all toy_ functions
        .generate()
        .expect("Unable to generate bindings");
        
        let bindings_string = "#![allow(unused)]\n".to_string() +  &bindings.to_string().replace(
            r#"extern "C""#,
            r#"unsafe extern "C""#,
        );
    std::fs::write("src/ffi.rs", bindings_string)
    .expect("Couldn't write bindings!");

}
