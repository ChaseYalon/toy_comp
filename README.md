To print debug info run with --features debug 
Before that do $env:DEBUG_TARGET= <target>
Note: There is a potentially nasty scoping bug around func literals, where parameters are declared in the parent scope
I think there is also a nasty bug with scope hoisting not working because of how TIR::Scope::set_var works