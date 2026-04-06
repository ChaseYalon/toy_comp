import sys
import platform
import subprocess
import os
from pathlib import Path

def find_rustlib_etc():
    # Try `rustup which rustc` to find the active toolchain
    try:
        rustc = subprocess.check_output(["rustup", "which", "rustc"], text=True).strip()
        # Goes: .../bin/rustc -> .../lib/rustlib/etc
        bin_dir = Path(rustc).parent
        etc_dir = bin_dir.parent / "lib" / "rustlib" / "etc"
        if etc_dir.exists():
            return etc_dir
    except subprocess.CalledProcessError:
        pass

    # Fallback: search RUSTUP_HOME
    rustup_home = Path(os.environ.get("RUSTUP_HOME", Path.home() / ".rustup"))
    toolchains = rustup_home / "toolchains"
    if toolchains.exists():
        # Pick the first nightly, or just the first toolchain
        candidates = sorted(toolchains.iterdir(), reverse=True)
        for tc in candidates:
            etc = tc / "lib" / "rustlib" / "etc"
            if (etc / "lldb_lookup.py").exists():
                return etc

    raise FileNotFoundError("Could not find rustlib/etc directory")


def run_lldb(exe: str, extra_commands: list[str] = None): #type: ignore
    etc = find_rustlib_etc()
    lookup = etc / "lldb_lookup.py"
    commands_file = etc / "lldb_commands"

    # Build the init commands
    init = [
        f"command script import {lookup}",
        f"command source {commands_file}",
    ]
    if extra_commands:
        init.extend(extra_commands)

    # Write a temp lldbinit
    init_file = Path("_lldb_init_tmp.txt")
    init_file.write_text("\n".join(init) + "\n")

    try:
        cmd = ["lldb", "--source", str(init_file), exe]
        subprocess.run(cmd)
    finally:
        init_file.unlink(missing_ok=True)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python debug.py <executable> [extra lldb commands...]")
        sys.exit(1)

    exe = sys.argv[1]
    extras = sys.argv[2:]  # e.g. "b toy_fs_read_dir" "run"
    run_lldb(exe, extras)