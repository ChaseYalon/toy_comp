<h1>Welcome to ToyLang</h1>
A demonstration of the CTLA garbage collection model.
Please reach out to chaseyalon@gmail.com or chase@chaseyalon.com with questions/comments.
This repository is actively maintained, issues and PR's are appreciated.

<h1>DOCS</h1>
ToyLang is a compiled language, meaning that it produces binary executable files. The currently supported platforms are x86_64-Windows and x86_64-Linux, both use the GNU abi. By default if you run the binary it expects a path to a .toy file that it will compile. By default it is in JIT mode, meaning it will take the toy code and just in-time compile and run it, if you pass the --aot the compiler will instead produce a binary named output.exe (64-bit PE) or output (64-bit ELF). You will be able to pass a -o parameter in a future version and if you (yes you ya lazy bum) want to make a pr with the feature, the change needs to happen in /src/codegen/mod.rs.

<h2>Datatypes</h2>
Toy lang supports the following first class datatypes - meaning they are fully supported for all situations
<ul>
    <li> <code>int</code>: 64 bit signed integer</li>
    <li> <code>float</code>: 64 bit signed float</li>
    <!-- the reason I mention the 64-bit is in case somebody is doing C-ABI stuff, in reality the compiler will not let you do arithmetic on them -->
    <li> <code>bool</code>: 64 bit integer representing a bool (value of 1 for true, 0 for false)</li>
    <li> <code>str</code>: Represents a string of characters, you can call <code>len(str)</code> to get the length, but strings are immutable under the hood, so be aware of that</li>
    <!-- TODO: Struct implementation is terrible, fix-->
    <li> <code>Struct</code>: structs are special first class datatypes because while you can pass and return them from functions, there is no <code>MY_STRUCT[]</code> type, instead use <code>struct[]</code> (which is "first class" in the same way structs are) or <code>any[]</code> (which is second class) </li>
    <li>I touched on it above but all first class types can have n dimensional arrays declared like <code>int[][]</code> for an integer matrix. Lengths are not necessary and the arrays behave like vectors in the strict sense, as they are contiguously allocated in memory, but can grow, and will be able to shrink in the future</li>
</ul>
The following are second class datatypes, which may be used in certain scenarios however should be avoided by default, and are not guaranteed to work in all scenarios, 
<ul>
    <li> <code>Void</code>: represents a function that does not return, stability and compilation are guaranteed when using it as a return type from a function, and for no other place </li>
    <li> <code>Any</code>: Will disable the typechecker when used, so it assumes you have manually checked your types, if they are wrong you will get a nasty error, while the following is allowed: functions may return <code>any</code>, and take <code>any</code> as parameters, variables may assigned to any, and you may make <code>any[]</code>'s it is highly unstable for all other use cases and should be avoided if at all possible</li>
    <li> Functions: This language has 0, nada, nil functional features. Maybe they will be added later but for now treat all functions like C functions</li>
</ul>

<h2>Syntax</h2>

To declare a variable, use:
```toy
let x = 9;
```
The type is automatically inferred.
If you want to explicitly specify a type:
```toy
let x: str = "hello world";
```
Notice the semicolons!!!

Variable reassignment and compound operators are also supported
```toy
let x: float = 9.2;
x += 315.2;
```

You can use an if statement as follows
```toy
let x = true && !false;

if x {
    println("it worked");
} else {
    println("it failed");
}
```
Else is supported, but else-if chains are not.

You already saw a function call above but here is the general syntax for functions
```toy
fn add(a: int, b: int): int {
    return a + b;
}

let x = add(9, 3);
```
All function parameters and return types (including void) must be stated explicitly, here is a void example
```toy
fn my_println(s: str): void {
    println(s);
}
my_println("hi mom!!!");
```
You can see a list of builtin functions at the bottom

Currently only while loops are supported, but iteration loops (for) will be in the (very) distant future
```toy
let i = 0;
while i < 7 {
    if i == 3{
        continue;
    }
    if i == 6 {
        break;
    }
    println(i);
}
```
will print 1, 2, 4, 5,

Arrays are also fully supported
```toy
let arr = [0, 1, 2, 3, 4];
arr = [9, 2];
let x = arr[0];
println(arr);
```
prints [9, 2]

Structs are also fully supported
```toy
struct Point{
    x: float,
    y: float
}
let origin = Point{x: 0.0, y: 0.0};
fn print_point(p: Point): void{
    print("Point{x: ");
    print(p.x);
    print(", y: ");
    print(p.y);
    print("};");
}
```

<h2>Builtin functions</h2>
<ul>
    <li> <code>print(s: any): void</code> prints an output to the standard output </li>
    <li> <code>println(s: any): void</code> prints an output to the standard output with a newline</li>
    <li> <code> len(v: any): int</code> the signature takes an any but can only be called on a string or an array, will return the number of characters or elements respectively</li>
    <li> <code>int(i: any): int </code> returns the integer value of a value an object if it is a string of "17" it will become 17 otherwise it will panic, and for floats it will round</li>
    <li> <code>str(s: any): str</code> returns the string value of any convertible object, for booleans false => "false", true is the same, for an integer 7 => "7", and same with floats</li>
    <li> <code> float(f: any): float</code> returns a float from a convertible value, will turn "5.3" => 5.3, false => 0.0, true => 1.0, will turn 1 => 1.0
    <li> <code> bool(b: any): bool </code> will turn "true" => true, same with false and will panic for any other string, will turn 1 => true, 0 => false and wil panic otherwise, and will round a float and use do the same</li>
</ul>

<h2> Build Instructions </h2>
If you do not have the build system setup (mys2 - clang/llvm), rust on the correct toolchain, cmake, and ninja run the following.
<pre><code class="language-python">
python setup_build_system.py #will install build system
</code></pre>
then you can do 
<pre><code class="language-shell">
cargo run -- --repl # will get you a repl
cargo run -- PATH_TO_FILE # will compile a .toy file
</code></pre>