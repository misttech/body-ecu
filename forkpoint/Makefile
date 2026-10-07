# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
#
# Build the Body ECU's MCU firmware and run it on Forkpoint's deterministic virtual MCU.
# Run from this directory, inside the west workspace `west init -l body-ecu` lays out:
#
#   make build-cpp          the C++ firmware, instrumented (BODY_ECU_FORKPOINT=ON)
#   make run-cpp            one scripted run, logs under out/
#   make test-cpp           ./test.sh: milestones, determinism, replay, properties
#
# FPT names the Forkpoint binary (fpt on PATH), FPT_BOARDS its boards directory (the
# `boards/` of the Forkpoint checkout fpt was built from, which holds the board's Device
# Tree), and GNUARMEMB_TOOLCHAIN_PATH an Arm GNU toolchain root with libstdc++, such as
# Forkpoint's prebuilt/toolchains/arm-none-eabi.

FPT ?= fpt
FPT_BOARDS ?= $(error set FPT_BOARDS to the boards directory of the Forkpoint checkout)
BOARD ?= nucleo-h743zi
ZEPHYR_BOARD ?= nucleo_h753zi
REPO := $(abspath $(CURDIR)/..)
WS := $(abspath $(REPO)/..)
OUT := $(CURDIR)/out
export ZEPHYR_BASE ?= $(WS)/zephyr
export ZEPHYR_TOOLCHAIN_VARIANT ?= gnuarmemb
# 5 virtual seconds at the board's 480 MHz: through the scenario's last request.
CYCLES ?= 2400000000

CPP_BUILD := $(OUT)/build-cpp
CPP_ELF := $(OUT)/body-ecu-cpp.elf
RUST_BUILD := $(OUT)/build-rust
RUST_ELF := $(OUT)/body-ecu-rust.elf
# The Forkpoint SDK's Rust crate reads the hostcall window's address at build time.
export FPT_HOSTCALL_BASE ?= 0xA0000000

.PHONY: build-cpp run-cpp test-cpp build-rust run-rust test-rust clean
build-cpp: $(CPP_ELF)
$(CPP_ELF): FORCE
	west build -b $(ZEPHYR_BOARD) -d "$(CPP_BUILD)" --pristine=auto "$(REPO)/app" -- \
		-DBODY_ECU_FORKPOINT=ON -DEXTRA_CONF_FILE="$(CURDIR)/forkpoint.conf"
	cp "$(CPP_BUILD)/zephyr/zephyr.elf" "$@"
FORCE:

run-cpp: $(CPP_ELF)
	"$(FPT)" explore --boards-dir "$(FPT_BOARDS)" --board $(BOARD) --max-cycles $(CYCLES) \
		--i2c-script scripts/scenario.script --properties "$(OUT)/cpp-properties.jsonl" \
		"$(CPP_ELF)" >"$(OUT)/cpp-run.txt" 2>"$(OUT)/cpp-log.txt"; \
	cat "$(OUT)/cpp-run.txt"

test-cpp: $(CPP_ELF)
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) ./test.sh cpp "$(CPP_ELF)"

# The Rust firmware (rust/app), on the Rust OpenBSW port west checks out beside the
# upstream trees; `rustup target add thumbv7em-none-eabi` installs its target.
build-rust: $(RUST_ELF)
$(RUST_ELF): FORCE
	west build -b $(ZEPHYR_BOARD) -d "$(RUST_BUILD)" --pristine=auto "$(REPO)/rust/app" -- \
		-DBODY_ECU_FORKPOINT=ON -DEXTRA_CONF_FILE="$(CURDIR)/forkpoint.conf"
	cp "$(RUST_BUILD)/zephyr/zephyr.elf" "$@"

run-rust: $(RUST_ELF)
	"$(FPT)" explore --boards-dir "$(FPT_BOARDS)" --board $(BOARD) --max-cycles $(CYCLES) \
		--i2c-script scripts/scenario.script --properties "$(OUT)/rust-properties.jsonl" \
		"$(RUST_ELF)" >"$(OUT)/rust-run.txt" 2>"$(OUT)/rust-log.txt"; \
	cat "$(OUT)/rust-run.txt"

test-rust: $(RUST_ELF)
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) ./test.sh rust "$(RUST_ELF)"

clean:
	rm -rf -- "$(OUT)"
