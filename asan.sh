#!/bin/bash
echo "please be aware, this needs program.ll and will compile the whole std directory"
clang -fsanitize=address -fno-omit-frame-pointer Program.ll std/* /lib/x86_64-unknown-linux-gnu/libruntime.a -o Program
./Program