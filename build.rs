//! Compiles the per-OS shim with the system compiler. No `cc` crate on purpose.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::process::Command;

fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to start {cmd:?}: {e}"));
    assert!(status.success(), "command failed: {cmd:?}");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=shim");

    let out = env::var("OUT_DIR").unwrap();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let out = Path::new(&out);
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=oeshim");

    match os.as_str() {
        "macos" => build_macos(out),
        "linux" => build_linux(out),
        "windows" => build_windows(out),
        other => panic!("unsupported target OS: {other}"),
    }
}

fn build_macos(out: &Path) {
    let obj = out.join("macos_metal.o");
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        _ => "x86_64",
    };
    run(Command::new("clang")
        .args(["-c", "shim/macos_metal.m", "-fobjc-arc", "-O2", "-Wall", "-Wextra"])
        .args(["-arch", arch, "-mmacosx-version-min=11.0", "-Ishim", "-o"])
        .arg(&obj));
    run(Command::new("ar").arg("crs").arg(out.join("liboeshim.a")).arg(&obj));
    for framework in ["Cocoa", "Metal", "QuartzCore", "CoreGraphics"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

/// PROTOTYPE: not built yet. Requires glslc, the Vulkan loader, and X11 headers.
fn build_linux(out: &Path) {
    let mut header = String::new();
    for (stage, name) in [("vert", "quad_vert_spv"), ("frag", "quad_frag_spv")] {
        let spv = out.join(format!("quad.{stage}.spv"));
        run(Command::new("glslc")
            .arg(format!("shim/shaders/quad.{stage}"))
            .arg("-o")
            .arg(&spv));
        let bytes = fs::read(&spv).unwrap();
        let words: Vec<String> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&word| format!("0x{:08x}", u32::from_le_bytes(word)))
            .collect();
        writeln!(header, "static const uint32_t {name}[] = {{{}}};", words.join(",")).unwrap();
    }
    fs::write(out.join("quad_spv.h"), header).unwrap();

    let obj = out.join("linux_vulkan.o");
    run(Command::new("cc")
        .args(["-c", "shim/linux_vulkan.c", "-std=c11", "-O2", "-Wall", "-Wextra", "-Ishim", "-I"])
        .arg(out)
        .arg("-o")
        .arg(&obj));
    run(Command::new("ar").arg("crs").arg(out.join("liboeshim.a")).arg(&obj));
    println!("cargo:rustc-link-lib=vulkan");
    println!("cargo:rustc-link-lib=X11");
}

/// PROTOTYPE: not built yet. Run from a Visual Studio developer prompt.
fn build_windows(out: &Path) {
    let obj = out.join("windows_d3d11.obj");
    run(Command::new("cl")
        .args(["/nologo", "/c", "/O2", "/W4", "/Ishim", "shim/windows_d3d11.c"])
        .arg(format!("/Fo{}", obj.display())));
    run(Command::new("lib")
        .arg("/nologo")
        .arg(format!("/OUT:{}", out.join("oeshim.lib").display()))
        .arg(&obj));
    for lib in ["user32", "d3d11", "dxgi", "d3dcompiler", "dxguid"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
