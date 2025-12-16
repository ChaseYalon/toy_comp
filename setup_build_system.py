import sys
import platform
import shutil
import subprocess
import os
import urllib.request
import ssl
import ctypes
import time
import glob
from pathlib import Path

ssl._create_default_https_context = ssl._create_unverified_context # type: ignore

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

if os_name != "Windows" and os_name != "Linux":
    print(f"[ERROR] Only windows and linux are supported, {os_name} was detected")
    sys.exit(1)
os.mkdir("lib")

if os_name == "Windows":
    os.makedirs(".cargo", exist_ok=True)
    with open(".cargo/config.toml", "w") as file:
        file.write('[build]\ntarget = "x86_64-pc-windows-gnu"\n')
    MSYS2_DIR = r"C:\msys64"
    BASH_EXE = os.path.join(MSYS2_DIR, "usr", "bin", "bash.exe")
    MINGW64_BIN = os.path.join(MSYS2_DIR, "mingw64", "bin")
    CARGO_BIN = os.path.expandvars(r"%USERPROFILE%\.cargo\bin")
    RUSTUP_EXE = os.path.join(CARGO_BIN, "rustup.exe")

    def detect_mingw_clang() -> bool:
        path = MINGW64_BIN + ";" + os.environ.get("PATH", "")
        return shutil.which("clang", path=path) is not None
    def add_mingw_to_user_path():
        import winreg
        target = MINGW64_BIN
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Environment",
            0,
            winreg.KEY_READ | winreg.KEY_WRITE
        )
        try:
            value, value_type = winreg.QueryValueEx(key, "Path")
        except FileNotFoundError:
            value, value_type = "", winreg.REG_EXPAND_SZ

        paths: list[str] = value.split(";") if value else []

        if not any(p.lower() == target.lower() for p in paths):
            new_value = value + ";" + target if value else target
            winreg.SetValueEx(key, "Path", 0, value_type, new_value)

        winreg.CloseKey(key)

        HWND_BROADCAST = 0xFFFF
        WM_SETTINGCHANGE = 0x001A
        SMTO_ABORTIFHUNG = 0x0002
        ctypes.windll.user32.SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            "Environment",
            SMTO_ABORTIFHUNG,
            5000,
            None
        )

    def install_msys2():
        if not os.path.exists(MSYS2_DIR):
            print("Installing MSYS2...")
            subprocess.run(["winget", "install", "-e", "--id", "MSYS2.MSYS2"], check=True)

    def msys(cmd: str):
        return subprocess.run([BASH_EXE, "-lc", cmd], check=True)

    def first_msys_update():
        """Initial MSYS2 update that may kill bash.exe (expected)."""
        try:
            msys("pacman -Syu --noconfirm")
        except subprocess.CalledProcessError:
            print("(Expected) MSYS2 terminated itself during first update.")
        time.sleep(2)

    def second_msys_update():
        """Follow-up update to stabilize environment."""
        msys("pacman -Syu --noconfirm")

    def install_mingw_packages():
        print("Updating MSYS2 base system...")
        first_msys_update()
        second_msys_update()
        print("Installing MinGW-w64 Clang and toolchain...")
        msys(
            "pacman -S --needed --noconfirm "
            "mingw-w64-x86_64-toolchain "
            "mingw-w64-x86_64-clang "
            "mingw-w64-x86_64-llvm"
            "mingw-w64-x86_64-libffi"#not sure if this is nesscary
        )

    def detect_rustup_windows() -> bool:
        if not os.path.exists(RUSTUP_EXE):
            return False
        try:
            env = os.environ.copy()
            env["PATH"] = CARGO_BIN + ";" + env.get("PATH", "")
            subprocess.run(
                [RUSTUP_EXE, "--version"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env=env,
                check=True
            )
            return True
        except Exception:
            return False

    install_msys2()
    install_mingw_packages()
    add_mingw_to_user_path()
    os.chdir("lib")
    subprocess.run(["wget", "https://downloads.sourceforge.net/project/toy-comp-lib-download/x86_64-pc-windows-gnu.tar.gz"])
    subprocess.run(["tar", "-xzvf", "x86_64-unknown-linux-gnu.tar.gz"])
    if not detect_mingw_clang():
        print("[WARNING] Clang not detected after install — retrying...")
        install_mingw_packages()

    print("MinGW/Clang installed successfully.")

    if not detect_rustup_windows():
        print("Installing Rustup...")
        url = "https://win.rustup.rs/x86_64"
        urllib.request.urlretrieve(url, "rustup-init.exe")
        subprocess.run(["rustup-init.exe", "-y"], check=True)
    else:
        print("Rustup already installed")

    env = os.environ.copy()
    env["PATH"] = CARGO_BIN + ";" + env.get("PATH", "")

    subprocess.run(
        [RUSTUP_EXE, "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"],
        check=True,
        env=env
    )

    for winget_id, name in [("cmake", "cmake"), ("Ninja-build.Ninja", "ninja")]:
        if shutil.which(name) is None:
            print(f"Installing {name} via winget...")
            subprocess.run(["winget", "install", winget_id, "-e", "--silent"], check=True)
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
    subprocess.run(["wget", "https://downloads.sourceforge.net/project/toy-comp-lib-download/x86_64-unknown-linux-gnu.tar.gz"])
    subprocess.run(["tar", "-xzvf", "x86_64-unknown-linux-gnu.tar.gz"])
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

    subprocess.run(
        ["rustup", "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"],
        check=True,
    )
lib_dir = Path("lib")
for gz_file in lib_dir.glob("*.gz"):
    gz_file.unlink()

print("Build system installation complete!")
print("Please restart your shell for path changes to take effect")
print("\n\n")
print("Run REPL: cargo run -- --repl")
print("Run file: cargo run -- <PATH>")