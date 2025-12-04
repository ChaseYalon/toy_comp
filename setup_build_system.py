import sys
import platform
import shutil
import subprocess
import os
import urllib.request
import ssl
import ctypes
import time


perm_granted = input(
    "This wizard will require access to your network if it needs to download dependencies, "
    "and it will require access to read and write to your whole system. Is this ok [n/Y]: "
)
print("Entering ToyLang build system setup wizard")
if perm_granted.lower() == "n":
    print("[ERROR] Permission denied")
    sys.exit(1)
os_name = platform.system()
cpu_type = platform.uname().machine
print(f"OS detected as {os_name} with a {cpu_type} cpu")

if cpu_type not in ("x86_64", "AMD64"):
    print(f"[ERROR] Detected a {cpu_type} cpu, but only x86_64 is supported")
    sys.exit(1)

if os_name == "Windows":
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

        paths = value.split(";") if value else []

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
    print("Linux detected, only Debian-based systems fully supported by this script")
    if shutil.which("clang") is None:
        print("Installing Clang + CMake + Ninja via apt...")
        subprocess.run(["sudo", "apt", "update"], check=True)
        subprocess.run(
            ["sudo", "apt", "install", "-y", "clang", "cmake", "ninja-build", "build-essential"],
            check=True
        )

    if shutil.which("rustup") is None:
        print("Installing Rustup...")
        subprocess.run(
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
            shell=True,
            check=True
        )

    subprocess.run(["rustup", "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"], check=True)

print("Build system installation complete!")
print("Run REPL: cargo run -- --repl")
print("Run file: cargo run -- <PATH>")
