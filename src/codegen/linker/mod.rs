use crate::errors::{ToyError, ToyErrorType};
use std::path::Path;
use std::env;
use std::process::Command;
pub struct Linker {

}
pub static FILE_EXTENSION_EXE: &str = if cfg!(target_os = "windows") {
    ".exe"
} else {
    ""
};
impl Linker {
    pub fn new() -> Linker {
        Linker {

        }
    }
    pub fn link(&mut self, files: Vec<String>, output: String) -> Result<(), ToyError> {
        //linker
        let target = env!("TARGET").replace("\"", "");
        let lib_str = format!("lib/{}/", target);
        let lib_path = Path::new(&lib_str);
        let crt2_path = lib_path.join("crt2.o");
        let crtbegin_path = lib_path.join("crtbegin.o");
        let crt1_path = lib_path.join("crt1.o");
        let crti_path = lib_path.join("crti.o");
        let lbruntime_path = lib_path.join("libruntime.a");
        let crtn_path = lib_path.join("crtn.o");
        let libc_path = lib_path.join("libc.so.6");
        let libm_path = lib_path.join("libm.so.6");
        let output_name = format!("{}{}", output, FILE_EXTENSION_EXE);
        let args: Vec<&str> = if env::consts::OS == "windows" {
            let mut args = vec![
                "-m",
                "i386pep",
                crt2_path.to_str().unwrap(),
                crtbegin_path.to_str().unwrap(),
            ];
            for file in &files {
                args.push(file.as_str());
            }
            args.extend_from_slice(&[
                "-L",
                lib_path.to_str().unwrap(),
                "-lruntime",
                "-lmingw32",
                "-lmingwex",
                "-lmsvcrt",
                "-lkernel32",
                "-luser32",
                "-lshell32",
                "-lgcc",
                "-o",
                output_name.as_str(),
            ]);
            args
        } else {
            let mut args = vec![
                "-m",
                "elf_x86_64",
                crt1_path.to_str().unwrap(),
                crti_path.to_str().unwrap(),
            ];
            for file in &files {
                args.push(file.as_str());
            }
            args.extend_from_slice(&[
                lbruntime_path.to_str().unwrap(),
                crtn_path.to_str().unwrap(),
                libc_path.to_str().unwrap(),
                libm_path.to_str().unwrap(),
                "-dynamic-linker",
                "/lib64/ld-linux-x86-64.so.2",
                "-o",
                output_name.as_str(),
            ]);
            args
        };
        let rstatus = Command::new(lib_path.join("ld.lld"))
            .args(args.clone())
            .status();
        let status = match rstatus {
            Ok(f) => f,
            Err(_) => {
                eprintln!("Linker args: {:#?}", args);
                return Err(ToyError::new(ToyErrorType::InternalLinkerFailure, None))
            },
        };
        if !status.success() {
            return Err(ToyError::new(ToyErrorType::InternalLinkerFailure, None));
        }
        Ok(())
    }
}