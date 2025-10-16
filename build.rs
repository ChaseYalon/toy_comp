// build.rs
fn main() {
    //link lib tcc
    println!("cargo:rustc-link-search=native=tcc/lib"); 
    println!("cargo:rustc-link-lib=static=tcc"); 
}
