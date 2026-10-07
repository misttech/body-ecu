# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

# The Body ECU's MCU firmware in Rust, as a Zephyr application: the Cargo package in this
# directory, built by the Rust OpenBSW port's rust_app.cmake, linked with the one C file
# that owns the kernel objects and devices.
#
# BODY_ECU_FORKPOINT=ON builds the Rust with its `forkpoint` feature, which reports
# properties and events through Forkpoint's hostcall window; the window's address comes
# from the FPT_HOSTCALL_BASE environment variable the Forkpoint SDK reads at build time
# (forkpoint/Makefile sets it).

cmake_minimum_required(VERSION 3.20.0)

find_package(Zephyr REQUIRED HINTS $ENV{ZEPHYR_BASE})

project(body_ecu_rust LANGUAGES C)

include(${CMAKE_CURRENT_SOURCE_DIR}/../../../openbsw-mss/openbsw-zephyr/rust/cmake/rust_app.cmake)

option(BODY_ECU_FORKPOINT "Report properties and events to Forkpoint's virtual MCU" OFF)

set(RUST_CARGO_ARGS --locked)
set(rust_features "")
if(BODY_ECU_FORKPOINT)
    list(APPEND rust_features forkpoint)
endif()
if(CONFIG_ADC)
    list(APPEND rust_features adc)
endif()
if(CONFIG_CAN)
    list(APPEND rust_features can)
endif()
if(rust_features)
    list(JOIN rust_features "," rust_features)
    list(APPEND RUST_CARGO_ARGS --features ${rust_features})
endif()

# The only C file: it owns the kernel objects and devices and forwards plain-scalar calls
# to Zephyr. The C++ application's libraries are built at -O2; the shim is too.
target_sources(app PRIVATE src/zephyr_shim.c)
set_source_files_properties(src/zephyr_shim.c PROPERTIES COMPILE_OPTIONS -O2)

# The Forkpoint SDK's catalog section, which the Rust records its properties in, stays
# in the ELF for the host and out of the image.
zephyr_linker_sources(SECTIONS fpt_catalog.ld)

openbsw_rust_application()
