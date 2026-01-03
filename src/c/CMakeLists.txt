cmake_minimum_required(VERSION 3.10)
project(toy_runtime C)

# Detect architecture
if(CMAKE_HOST_SYSTEM_PROCESSOR MATCHES "AMD64|x86_64|amd64")
    set(TARGET_ARCH "x86_64")
elseif(CMAKE_HOST_SYSTEM_PROCESSOR MATCHES "ARM64|aarch64")
    set(TARGET_ARCH "aarch64")
else()
    set(TARGET_ARCH "${CMAKE_HOST_SYSTEM_PROCESSOR}")
endif()

# Detect OS
if(CMAKE_SYSTEM_NAME STREQUAL "Windows")
    set(TARGET_OS "pc-windows-gnu")

    # Force GNU ABI with MinGW Clang
    set(CLANG_TARGET "x86_64-w64-windows-gnu")

    # Force compilers
    set(CMAKE_C_COMPILER clang CACHE STRING "" FORCE)
    set(CMAKE_CXX_COMPILER clang++ CACHE STRING "" FORCE)

    # Force target + sysroot
    set(CMAKE_C_COMPILER_TARGET ${CLANG_TARGET})
    set(CMAKE_CXX_COMPILER_TARGET ${CLANG_TARGET})

    # Add explicit flags (required for MinGW ABI)
    add_compile_options(
        --target=${CLANG_TARGET}
        -fuse-ld=lld
    )
    add_link_options(
        --target=${CLANG_TARGET}
        -fuse-ld=lld
    )
elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(TARGET_OS "unknown-linux-gnu")

    # Use system clang normally
    set(CMAKE_C_COMPILER clang)
    set(CMAKE_CXX_COMPILER clang++)
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    set(TARGET_OS "apple-darwin")
endif()

# Final target triple
set(TARGET_TRIPLE "${TARGET_ARCH}-${TARGET_OS}")
string(TOLOWER "${TARGET_TRIPLE}" TARGET_TRIPLE)

# Output directory
set(OUT_BASE "$ENV{CARGO_MANIFEST_DIR}/lib/${TARGET_TRIPLE}")
file(MAKE_DIRECTORY ${OUT_BASE})

# Libraries
add_library(runtime STATIC
    builtins.c
    hashmap.c
    ctla/ctla.c
    ctla/hashmap.c
    std/math.c
    stub.c
)

add_library(core STATIC
    builtins.c
    hashmap.c
    ctla/ctla.c
    ctla/hashmap.c
    llvm_stubs.c
    std/math.c
)

set_target_properties(runtime core PROPERTIES
    ARCHIVE_OUTPUT_DIRECTORY "${OUT_BASE}"
)

install(TARGETS runtime core
    ARCHIVE DESTINATION "${OUT_BASE}"
)