try:
    import sys
    import platform
    import shutil
    import subprocess
    import os
    import urllib.request
    import ssl
    #turn off ssl validation
    ssl._create_default_https_context = ssl._create_unverified_context

    def detect_rustup() -> bool:
        path = shutil.which("rustup")
        if path is None:
            return False

        try:
            subprocess.run(
                ["rustup", "--version"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=True
            )
            return True
        except Exception:
            return False

    print("Entering ToyLang build system setup wizard")
    perm_grated = input("This wizard will require access to your network if it needs to download dependencies, and it will require access to read and write to your whole system. Is this ok [n/Y]: ")
    if perm_grated == "n":
        print("[ERROR] Permission denied")
        sys.exit(1)

    os_name = platform.system()
    cpu_type = platform.uname().machine
    print(f"OS detected as {os_name} with a {cpu_type} cpu")

    if os_name != "Windows" and os_name != "Linux":
        print(f"[ERROR] OS {os_name} is not supported, try to use a Linux VM if possible")
        sys.exit(1)
    if cpu_type != "x86_64" and cpu_type != "AMD64":
        print(f"[ERROR] Detected a {cpu_type} cpu, but only x86_64 is supported, try QEMU emulation if possible")
        sys.exit(1)

    print("Host target validation compete, installing build dependencies [Clang, Cargo, RustC] if not already installed")
    #windows specific setup
    if os_name == "Windows":
        #windows stuff
        import winreg
        #misc setup
        def detect_windows_clang() -> bool:
            clang_path = shutil.which("clang")
            if clang_path is None:
                return False

            try:
                # Run 'clang --version' to ensure it works
                result = subprocess.run(
                    ["clang", "--version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    check=True
                )
                output = result.stdout + result.stderr
                # Optionally check that it's LLVM/Clang
                if "clang" in output.lower() and "llvm" in output.lower():
                    return True
                return False
            except Exception:
                return False
        def install_rustup_windows():
                url = "https://win.rustup.rs/x86_64"
                urllib.request.urlretrieve(url, "rustup-init.exe")
                subprocess.run(["rustup-init.exe", "-y"], check=True)
                cargo_bin = os.path.expanduser(r"~\.cargo\bin")
                rustup_exe = os.path.join(cargo_bin, "rustup.exe")

                subprocess.run([rustup_exe, "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"], check=True)
                cargo_bin = os.path.expandvars(r"%USERPROFILE%\.cargo\bin")
                subprocess.run([
                    "powershell",
                    "-Command",
                    f"[Environment]::SetEnvironmentVariable('PATH', '{cargo_bin};' + [Environment]::GetEnvironmentVariable('PATH','User'), 'User')"
                ], check=True)

                os.environ["PATH"] = cargo_bin + ";" + os.environ["PATH"]
        if not detect_windows_clang():
            print("Installing Clang")
            subprocess.run("winget install LLVM.LLVM", shell=True)
        else:
            print("Clang already installed, continuing")
        if not detect_rustup():
            print("Installing rustup")
            install_rustup_windows()
        else:
            print("Rustup already installed, switching to correct toolchain")
            subprocess.run(["rustup", "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"], check=True)

        #install cmake
        subprocess.run(["winget", "install", "CMAKE"], shell=True)

        #install ninja
        subprocess.run(["winget", "install", "Ninja-build.Ninja"], shell=True)


    if os_name == "Linux":
        #TODO: Make this not true
        print("Please note: Only Debian based distro can use this script, any GLIBC based distro can run the compiler, but you must install dependencies manually")
        if shutil.which("clang") is not None:
            print("Clang already installed, continuing")
        else:
            print("Installing Clang")
            cmds = [
                ["sudo", "apt", "update"],
                ["sudo", "apt", "install", "-y", "clang", "cmake", "ninja-build"]
            ]
            for cmd in cmds:
                subprocess.run(cmd, check=True)
            print("Clang installed")
        if not detect_rustup():
            print("Installing rustup")
            cmd = (
                "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | "
                "sh -s -- -y"
            )
            subprocess.run(cmd, shell=True, check=True)
            subprocess.run("cargo", shell=True, check=True)
        else:
            print("Rust already installed")
        #run regardless
        subprocess.run(["rustup", "target", "add", "x86_64-pc-windows-gnu", "--toolchain", "nightly"], check=True)
except Exception as e:
    print("[ERROR] Installer failed, please try again, or manually install dependencies")
    print(e)
    sys.exit(1)
else:
    print("Congrats!! Build system install complete, you can now get a repl by saying cargo run -- --repl or pass it a .toy file like cargo run -- <PATH>")
    print("Type cargo run -- --help for more details")
