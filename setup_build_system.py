import sys
import platform
import shutil
import subprocess
import os
import urllib.request
import ssl
import ctypes
import time
from pathlib import Path

ssl._create_default_https_context = ssl._create_unverified_context  # type: ignore

perm_granted = "Y" if len(sys.argv) > 1 and sys.argv[1] == "--ok" else input(
    "This wizard will require access to your network if it needs to download dependencies, "
    "and it will require access to read and write to your whole system. Is this ok [y/N]: "
)

print("Entering ToyLang build system setup wizard")
if perm_granted.lower() != "y":
    print("[ERROR] Permission denied")
    sys.exit(1)

os_name = platform.system()
cpu_type = platform.uname().machine
print(f"OS detected as {os_name} with a {cpu_type} cpu")

if cpu_type not in ("x86_64", "AMD64"):
    print(f"[ERROR] Detected a {cpu_type} cpu, but only x86_64 is supported")
    sys.exit(1)

if os_name not in ("Windows", "Linux"):
    print(f"[ERROR] Only windows and linux are supported, {os_name} was detected")
    sys.exit(1)

os.makedirs("lib", exist_ok=True)

if os_name == "Windows":
    import winreg

    def set_user_env_var(name: str, value: str):

        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Environment",
            0,
            winreg.KEY_SET_VALUE,
        )

        winreg.SetValueEx(
            key,
            name,
            0,
            winreg.REG_EXPAND_SZ,
            value,
        )

        winreg.CloseKey(key)

        ctypes.windll.user32.SendMessageTimeoutW(
            0xFFFF,
            0x001A,
            0,
            "Environment",
            0x0002,
            5000,
            None,
        )

    os.makedirs(".cargo", exist_ok=True)
    with open(".cargo/config.toml", "w") as file:
        file.write('[build]\ntarget = "x86_64-pc-windows-gnu"\n')

    MSYS2_DIR = r"C:\msys64"
    BASH_EXE = os.path.join(MSYS2_DIR, "usr", "bin", "bash.exe")
    MINGW64_PREFIX = os.path.join(MSYS2_DIR, "mingw64")
    MINGW64_BIN = os.path.join(MINGW64_PREFIX, "bin")
    CARGO_BIN = os.path.expandvars(r"%USERPROFILE%\.cargo\bin")
    RUSTUP_EXE = os.path.join(CARGO_BIN, "rustup.exe")

    def detect_mingw_clang() -> bool:
        path = MINGW64_BIN + ";" + os.environ.get("PATH", "")
        return shutil.which("clang", path=path) is not None

    def add_mingw_to_user_path():
        import winreg

        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Environment",
            0,
            winreg.KEY_READ | winreg.KEY_WRITE,
        )
        try:
            value, value_type = winreg.QueryValueEx(key, "Path")
        except FileNotFoundError:
            value, value_type = "", winreg.REG_EXPAND_SZ

        paths = value.split(";") if value else []
        if not any(p.lower() == MINGW64_BIN.lower() for p in paths):
            value = value + ";" + MINGW64_BIN if value else MINGW64_BIN
            winreg.SetValueEx(key, "Path", 0, value_type, value)

        winreg.CloseKey(key)

        ctypes.windll.user32.SendMessageTimeoutW(
            0xFFFF,
            0x001A,
            0,
            "Environment",
            0x0002,
            5000,
            None,
        )

    def install_msys2():
        if not os.path.exists(MSYS2_DIR):
            subprocess.run(
                ["winget", "install", "-e", "--id", "MSYS2.MSYS2"],
                check=True,
            )

    def msys(cmd: str):
        return subprocess.run([BASH_EXE, "-lc", cmd], check=True)

    def install_mingw_packages():
        msys("pacman -Syu --noconfirm || true")
        msys("pacman -Syu --noconfirm")
        msys(
            "pacman -S --needed --noconfirm "
            "mingw-w64-x86_64-toolchain "
            "mingw-w64-x86_64-clang "
            "mingw-w64-x86_64-llvm "
            "mingw-w64-x86_64-libffi"
        )

    def detect_rustup_windows() -> bool:
        return os.path.exists(RUSTUP_EXE)

    install_msys2()
    install_mingw_packages()
    add_mingw_to_user_path()

    os.chdir("lib")
    tar_name = "x86_64-pc-windows-gnu.tar.gz"
    urllib.request.urlretrieve(
        "https://downloads.sourceforge.net/project/toy-comp-lib-download/"
        "x86_64-pc-windows-gnu.tar.gz",
        tar_name,
    )

    if not detect_mingw_clang():
        install_mingw_packages()

    if not detect_rustup_windows():
        urllib.request.urlretrieve(
            "https://win.rustup.rs/x86_64", "rustup-init.exe"
        )
        subprocess.run(["rustup-init.exe", "-y"], check=True)

    env = os.environ.copy()
    env["PATH"] = ";".join([MINGW64_BIN, CARGO_BIN, env.get("PATH", "")])
    set_user_env_var("LLVM_SYS_211_PREFIX", MINGW64_BIN)
    print(MINGW64_BIN)
    subprocess.run(
        ["ls", "-la", MINGW64_BIN],
        check=True
    )
    subprocess.run(f"./{MINGW64_BIN}/llvm-config.exe", shell=True)
    env["LIBCLANG_PATH"] = MINGW64_BIN

    subprocess.run(
        [
            RUSTUP_EXE,
            "target",
            "add",
            "x86_64-pc-windows-gnu",
            "--toolchain",
            "nightly",
        ],
        check=True,
        env=env,
    )

    for winget_id, name in [
        ("Kitware.CMake", "cmake"),
        ("Ninja-build.Ninja", "ninja"),
    ]:
        if shutil.which(name) is None:
            subprocess.run(
                ["winget", "install", winget_id, "-e", "--silent"],
                check=True,
            )
elif os_name == "Linux":
    def apt_install(*pkgs):
        subprocess.run(["sudo", "apt", "update"], check=True)
        subprocess.run(["sudo", "apt", "install", "-y", *pkgs], check=True)

    if shutil.which("clang") is None:
        apt_install("clang")
    if shutil.which("cmake") is None:
        apt_install("cmake")
    if shutil.which("ninja") is None:
        apt_install("ninja-build")
    if shutil.which("tar") is None:
        apt_install("tar")

    os.chdir("lib")
    subprocess.run(
        [
            "wget",
            "https://downloads.sourceforge.net/project/"
            "toy-comp-lib-download/x86_64-unknown-linux-gnu.tar.gz",
        ],
        check=True,
    )
    subprocess.run(
        ["tar", "-xzvf", "x86_64-unknown-linux-gnu.tar.gz"],
        check=True,
    )

    if shutil.which("rustup") is None:
        subprocess.run(
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
            shell=True,
            check=True,
        )

    subprocess.run(
        "sudo wget -qO /etc/apt/trusted.gpg.d/apt.llvm.org.asc "
        "https://apt.llvm.org/llvm-snapshot.gpg.key",
        shell=True,
        check=True,
    )

    subprocess.run(
        'echo "deb http://apt.llvm.org/$(lsb_release -sc) '
        'llvm-toolchain-$(lsb_release -sc)-21 main" | '
        "sudo tee /etc/apt/sources.list.d/llvm.list",
        shell=True,
        check=True,
    )

    subprocess.run(["sudo", "apt", "update"], check=True)
    subprocess.run(
        [
            "sudo",
            "apt",
            "install",
            "-y",
            "clang-21",
            "llvm-21",
            "llvm-21-dev",
            "lld-21",
            "libpolly-21-dev",
            "libffi-dev",
            "libzstd-dev",
        ],
        check=True,
    )

    subprocess.run(["sudo", "mkdir", "-p", "/opt/llvm-21/bin"], check=True)
    subprocess.run(
        [
            "sudo",
            "ln",
            "-sf",
            "/usr/bin/llvm-config-21",
            "/opt/llvm-21/bin/llvm-config",
        ],
        check=True,
    )

    bashrc = os.path.expanduser("~/.bashrc")
    with open(bashrc, "a") as f:
        f.write("\nexport LLVM_SYS_211_PREFIX=/opt/llvm-21\n")
        f.write("export LLVM_SYS_211_LINK_POLLY=0\n")
        f.write("export RUST_BACKTRACE=1\n")

    subprocess.run(
        ["chmod", "+x", "./x86_64-unkown-linux-gnu/ld.lld"],
        check=True,
    )

try:
    os.rename("x86_64-unkown-linux-gnu", "x86_64-unknown-linux-gnu")
except OSError:
    pass

os.chdir("..")
lib_dir = Path("lib")
for gz_file in lib_dir.rglob("*.gz"):
    gz_file.unlink()

print("Build system installation complete!")
print("Please restart your shell for path changes to take effect")
print("Run REPL: cargo run -- --repl")
print("Run file: cargo run -- <PATH>")
