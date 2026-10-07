# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
#
# Build the Body ECU's MCU firmware and run it on Forkpoint's deterministic virtual MCU.
# Run from this directory, inside the west workspace `west init -l body-ecu` lays out:
#
#   make build-cpp          the C++ firmware, instrumented (BODY_ECU_FORKPOINT=ON)
#   make run-cpp            one scripted run, logs under out/
#   make test-cpp           ./test.sh: milestones, determinism, replay, properties
#   make build-rust         the Rust firmware (rust/app), run-rust and test-rust likewise
#   make build-rust-rivet   the Rust firmware with rivet as its C library, run-rust-rivet
#                           and test-rust-rivet likewise
#   make equiv              fpt equiv of the C++ and Rust images; equiv-rivet the rivet one
#   make report             docs/forkpoint/set3-equivalence.md; report-rivet set4-rivet.md
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
# The Rust firmware with rivet as its C library (rust/third_party/rivet, built by Cargo for
# the board's Rust target with its allocator; its printf family's C entry points need
# clang, CLANG overrides it).
RIVET := $(REPO)/rust/third_party/rivet
RIVET_TARGET := thumbv7em-none-eabi
RIVET_TARGET_DIR := $(OUT)/rivet
RIVET_LIBRARY := $(RIVET_TARGET_DIR)/$(RIVET_TARGET)/release/librivet_libc.a
RIVET_BUILD := $(OUT)/build-rust-rivet
RIVET_ELF := $(OUT)/body-ecu-rust-rivet.elf
CARGO ?= cargo
# The Forkpoint SDK's Rust crate reads the hostcall window's address at build time.
export FPT_HOSTCALL_BASE ?= 0xA0000000

.PHONY: build-cpp run-cpp test-cpp build-rust run-rust test-rust build-rust-rivet run-rust-rivet \
	test-rust-rivet equiv equiv-rivet report report-rivet clean
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

# The Rust firmware on rivet: the same application, Zephyr built without a C library of
# its own (rivet.conf), rivet supplied by rivet_module/ in place of Zephyr's minimal one.
$(RIVET_LIBRARY): FORCE
	@command -v "$${CLANG:-clang}" >/dev/null || { \
		echo "clang is required to build rivet's printf family; install clang, or set CLANG" >&2; \
		exit 1; \
	}
	+cd "$(RIVET)/src" && CARGO_TARGET_DIR="$(RIVET_TARGET_DIR)" "$(CARGO)" rustc -p rivet-libc \
		--locked --release --target $(RIVET_TARGET) --features cmpctmalloc --crate-type staticlib

build-rust-rivet: $(RIVET_ELF)
$(RIVET_ELF): $(RIVET_LIBRARY) FORCE
	west build -b $(ZEPHYR_BOARD) -d "$(RIVET_BUILD)" --pristine=auto "$(REPO)/rust/app" -- \
		-DBODY_ECU_FORKPOINT=ON -DEXTRA_CONF_FILE="$(CURDIR)/forkpoint.conf;$(CURDIR)/rivet.conf" \
		-DZEPHYR_EXTRA_MODULES="$(CURDIR)/rivet_module" \
		-DRIVET_INCLUDE_DIR="$(RIVET)/include" -DRIVET_LIBRARY="$(RIVET_LIBRARY)"
	cp "$(RIVET_BUILD)/zephyr/zephyr.elf" "$@"

run-rust-rivet: $(RIVET_ELF)
	"$(FPT)" explore --boards-dir "$(FPT_BOARDS)" --board $(BOARD) --max-cycles $(CYCLES) \
		--i2c-script scripts/scenario.script --properties "$(OUT)/rust-rivet-properties.jsonl" \
		"$(RIVET_ELF)" >"$(OUT)/rust-rivet-run.txt" 2>"$(OUT)/rust-rivet-log.txt"; \
	cat "$(OUT)/rust-rivet-run.txt"

# newlib_free.sh reads the linker map: rivet, and no member of the toolchain's newlib.
test-rust-rivet: $(RIVET_ELF)
	MAP="$(RIVET_BUILD)/zephyr/zephyr.map" ./newlib_free.sh
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) ./test.sh rust-rivet "$(RIVET_ELF)"

# Both images through the same scenarios: fpt equiv, the consoles, the properties.
equiv: $(CPP_ELF) $(RUST_ELF)
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) ./equiv.sh

# The rivet image against the C++ baseline, and against the Rust image on Zephyr's
# minimal C library.
equiv-rivet: $(CPP_ELF) $(RUST_ELF) $(RIVET_ELF)
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) \
		RUST_ELF="$(RIVET_ELF)" RUST_NAME=rust-rivet PREFIX=rivet ./equiv.sh
	FPT="$(FPT)" FPT_BOARDS="$(FPT_BOARDS)" BOARD=$(BOARD) CYCLES=$(CYCLES) \
		CPP_ELF="$(RUST_ELF)" CPP_NAME=rust RUST_ELF="$(RIVET_ELF)" RUST_NAME=rust-rivet PREFIX=rivet-vs-rust ./equiv.sh

# The equivalence report, from what test-cpp, test-rust, and equiv left under out/.
report:
	python3 report.py "$(OUT)" "$(REPO)/docs/forkpoint/set3-equivalence.md"

# The rivet report, from what the three builds, test-rust-rivet, run-rust-rivet, and
# equiv-rivet left under out/.
report-rivet:
	python3 report.py --rivet "$(OUT)" "$(REPO)/docs/forkpoint/set4-rivet.md"

clean:
	rm -rf -- "$(OUT)"
