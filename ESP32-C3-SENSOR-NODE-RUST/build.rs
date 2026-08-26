use std::path::{Path, PathBuf};

fn find_compiler(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.file_name() == Some(std::ffi::OsStr::new("riscv32-esp-elf-gcc"))
            {
                return Some(path);
            } else if path.is_dir() {
                if let Some(found) = find_compiler(&path) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn main() {
    // ESP-IDF environment variables
    embuild::espidf::sysenv::output();

    let mut build = cc::Build::new();

    // Tìm riscv32-esp-elf-gcc trong .embuild (theo pattern c-in-rust-esp32)
    let local_embuild = PathBuf::from(".embuild");
    let home_embuild = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".embuild"))
        .ok();

    let gcc_path = find_compiler(&local_embuild)
        .or_else(|| home_embuild.as_ref().and_then(|p| find_compiler(p)));

    if let Some(compiler) = gcc_path {
        println!("cargo:warning=Found GCC at: {}", compiler.display());
        build.compiler(compiler);
    } else {
        println!("cargo:warning=Could not find riscv32-esp-elf-gcc in .embuild!");
    }

    // Include paths cho ADS1X15 / FreeRTOS / ESP-IDF headers
    build
        .file("src/ffi/sensor_ffi.c")
        .include("src/ffi")
        .compile("sensor_ffi");

    println!("cargo:rerun-if-changed=src/ffi/sensor_ffi.c");
    println!("cargo:rerun-if-changed=src/ffi/sensor_ffi.h");
}
