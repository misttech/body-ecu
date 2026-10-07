// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! For a bare-metal target, compiles the C entry points of `stdio.h`,
//! `stdio/variadic.c`, with clang into a static library that Cargo bundles
//! into rivet's archive. Stable Rust cannot define them (see that file). A
//! host build needs none of this: the functions export only on bare metal.
//!
//! Environment: `CLANG` and `AR` override the compiler and archiver.

use std::env;
use std::path::PathBuf;
use std::process::Command;

include!("../build/clang.rs");

fn main() {
    println!("cargo::rerun-if-changed=stdio/variadic.c");
    println!("cargo::rerun-if-changed=stdio/variadic.h");
    println!("cargo::rerun-if-changed=../build/clang.rs");
    println!("cargo::rerun-if-changed=../include");
    println!("cargo::rerun-if-env-changed=CLANG");
    println!("cargo::rerun-if-env-changed=AR");
    println!("cargo::rerun-if-env-changed=RIVET_CPU");
    println!("cargo::rerun-if-env-changed=RIVET_FRAME_POINTERS");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("none") {
        return;
    }
    let target = env::var("TARGET").expect("cargo sets TARGET");
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));

    let object = out.join("variadic.o");
    let clang = env::var("CLANG").unwrap_or_else(|_| "clang".into());
    let mut command = Command::new(&clang);
    command.args(clang_target_flags(&target)).args([
        "-O2",
        "-g",
        "-std=c11",
        "-ffreestanding",
        "-fno-builtin",
        "-nostdlibinc",
        "-ffunction-sections",
        "-fdata-sections",
        "-Wall",
        "-Wextra",
        "-Wmissing-prototypes",
        "-Werror",
    ]);
    command.arg("-I").arg(manifest.join("../include"));
    command.arg("-c").arg(manifest.join("stdio/variadic.c")).arg("-o").arg(&object);
    run_tool(&mut command, &clang);

    let archive = out.join("librivet_variadic.a");
    let _ = std::fs::remove_file(&archive);
    let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
    run_tool(Command::new(&ar).arg("crs").arg(&archive).arg(&object), &ar);
    println!("cargo::rustc-link-search=native={}", out.display());
    println!("cargo::rustc-link-lib=static=rivet_variadic");
}
