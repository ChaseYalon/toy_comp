# Force Clang
set(CMAKE_C_COMPILER "clang")
set(CMAKE_CXX_COMPILER "clang++")
set(CMAKE_C_COMPILER_TARGET "x86_64-w64-windows-gnu")


cmake_minimum_required(VERSION 3.10)
project(toy_runtime C)

#Maybe in the future replace all of this with the TARGET variable injected by build.rs
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
elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(TARGET_OS "unknown-linux-gnu")
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    set(TARGET_OS "apple-darwin")
else()
    set(TARGET_OS "${CMAKE_SYSTEM_NAME}")
endif()

# Target triple
set(TARGET_TRIPLE "${TARGET_ARCH}-${TARGET_OS}")
string(TOLOWER "${TARGET_TRIPLE}" TARGET_TRIPLE)

# Output directory
set(OUT_BASE "$ENV{CARGO_MANIFEST_DIR}/lib/${TARGET_TRIPLE}")
file(MAKE_DIRECTORY ${OUT_BASE})

# Libraries
add_library(runtime STATIC
    builtins.c
    hashmap.c
    stub.c
    ctla/ctla.c
    ctla/hashmap.c
)

add_library(core STATIC
    builtins.c
    hashmap.c
    ctla/ctla.c
    ctla/hashmap.c
)

# Set archive output directories
set_target_properties(runtime core PROPERTIES
    ARCHIVE_OUTPUT_DIRECTORY "${OUT_BASE}"
)

# Install (optional, works with `cmake --install`)
install(TARGETS runtime core
    ARCHIVE DESTINATION "${OUT_BASE}"
)
