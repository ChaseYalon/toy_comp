<h2>Welcome to ToyLang</h2>
A demonstration of the CTLA garbage collection model.
Please reach out to chaseyalon@gmail or chase@chaseyalon.com with questions/comments.
This repository is actively maintated, issues and PR's are appreciated.

<h2>DOCS</h2>
ToyLang is a compiled language, meaning that it produces binary executable files. The currently supported platforms are x86_64-Windows and x86_64-Linux, both use the GNU abi. By default if you run the binary it expects a path to a .toy file that it will compile. By default it is in JIT mode, meaning it will take the toy code and just in-time compile and run it, if you pass the --aot the compiler will instead produce a binary named output.exe (64-bit PE) or ouptut (64-bit ELF). You will be able to pass a -o parameter in a future version and if you (yes you ya lazy bum) want to make a pr with the feature, the change needs to happen in /src/codegen/mod.rs. Thanks!
