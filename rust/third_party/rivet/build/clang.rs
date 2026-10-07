// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

// Shared by the build scripts that compile C with clang for a Rust target:
// src/build.rs for the library's C entry points, and test/build.rs for the C
// tests. Each includes this file with `include!`, so it holds items only.

/// The clang flags that select `target` and its C ABI: the instruction set
/// and float ABI, the core `RIVET_CPU` names if any, and for bare-metal Arm
/// the enum size its ABI uses.
fn clang_target_flags(target: &str) -> Vec<String> {
    let mut flags = clang_isa_flags(target);
    let arm = target.starts_with("arm") || target.starts_with("thumb");
    // The firmware's core, as `make CPU=` passes it, which the Rust code is
    // also built for. On Arm it replaces the CPU and FPU the target's flags name; on
    // RISC-V the ISA string already says what the core has, so the core only
    // tunes the schedule.
    let cpu = std::env::var("RIVET_CPU").unwrap_or_default();
    if !cpu.is_empty() {
        if arm {
            // The core brings its own FPU, which the Rust code uses too.
            flags.retain(|flag| !flag.starts_with("-mcpu=") && !flag.starts_with("-mfpu="));
            flags.push(format!("-mcpu={cpu}"));
        } else {
            flags.push(format!("-mtune={cpu}"));
        }
    }
    // Frame records in every function, as `make FRAME_POINTERS=1` asks, so a
    // backtrace walks through rivet's own frames.
    if std::env::var("RIVET_FRAME_POINTERS").as_deref() == Ok("1") {
        flags.push("-fno-omit-frame-pointer".to_owned());
    }
    // Bare-metal Arm's ABI sizes an enum to its values, as GCC's arm-none-eabi
    // objects and the compiler runtime's tag theirs; clang sizes it as an int
    // unless told, and the linker warns when the two meet.
    if arm && target.contains("-none-") {
        flags.push("-fshort-enums".to_owned());
    }
    flags
}

/// The clang flags that select `target`'s instruction set and float ABI. The
/// two platforms the test suite runs on name their CPU and FPU; any other
/// target gets the generic flags clang derives from the triple, with the
/// RISC-V ISA string and ABI taken from the triple's architecture.
fn clang_isa_flags(target: &str) -> Vec<String> {
    let flag = |s: &str| s.to_owned();
    match target {
        "thumbv7em-none-eabihf" => vec![
            flag("--target=thumbv7em-none-eabihf"),
            flag("-mcpu=cortex-m4"),
            flag("-mfpu=fpv4-sp-d16"),
            flag("-mfloat-abi=hard"),
        ],
        "riscv32imafc-unknown-none-elf" => {
            vec![flag("--target=riscv32-unknown-elf"), flag("-march=rv32imafc"), flag("-mabi=ilp32f")]
        }
        _ if target.starts_with("riscv") => {
            let arch = target.split('-').next().unwrap_or(target);
            let (bits, extensions) = if let Some(rest) = arch.strip_prefix("riscv64") {
                ("64", rest)
            } else {
                ("32", arch.strip_prefix("riscv32").unwrap_or(""))
            };
            let base = extensions.split('_').next().unwrap_or("");
            // `g` is `imafd`, and `q` is the quad-precision ABI.
            let float = if base.contains('q') {
                "q"
            } else if base.contains('d') || base.contains('g') {
                "d"
            } else if base.contains('f') {
                "f"
            } else {
                ""
            };
            let abi = if bits == "64" { "lp64" } else { "ilp32" };
            vec![
                format!("--target=riscv{bits}-unknown-elf"),
                format!("-march=rv{bits}{extensions}"),
                format!("-mabi={abi}{float}"),
            ]
        }
        _ => {
            let mut flags = vec![format!("--target={target}")];
            if target.ends_with("hf") {
                flags.push(flag("-mfloat-abi=hard"));
            }
            flags
        }
    }
}

/// Runs `command`, naming `tool` if it cannot run or fails.
fn run_tool(command: &mut std::process::Command, tool: &str) {
    let status = command.status().unwrap_or_else(|error| {
        panic!("cannot run {tool}: {error}; install it or set its variable")
    });
    assert!(status.success(), "{command:?} failed with {status}");
}
