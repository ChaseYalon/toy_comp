fn main() {
    cc::Build::new().file("src/c/builtins.c").compile("builtins");
}
