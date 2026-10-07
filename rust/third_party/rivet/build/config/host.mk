# Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

# Select the host target and its output directory.
# Tags are <os>-<cpu>, os in {linux, mac}, cpu in {x64, arm64}.
# Linux and macOS are the supported hosts. The output directory is
# out/$(TARGET_PLATFORM)/, and out/default links to that directory.

_uname_s := $(shell uname -s)
_uname_m := $(shell uname -m)

ifeq ($(_uname_s),Linux)
include $(ROOT)/build/config/linux.mk
else ifeq ($(_uname_s),Darwin)
include $(ROOT)/build/config/mac.mk
else
$(error Unsupported host operating system '$(_uname_s)'. Supported hosts: Linux, macOS)
endif

ifeq ($(_uname_m),x86_64)
include $(ROOT)/build/config/x64.mk
else ifeq ($(_uname_m),aarch64)
include $(ROOT)/build/config/arm64.mk
else ifeq ($(_uname_m),arm64)
include $(ROOT)/build/config/arm64.mk
else
$(error Unsupported host architecture '$(_uname_m)'. Supported: x64, arm64)
endif

HOST_PLATFORM := $(HOST_OS)-$(HOST_CPU)

# Shown by `make help` only. Do not pass this to Cargo as --target:
# that nests another directory under out/$(TARGET_PLATFORM)/, where host builds belong.
ifeq ($(HOST_PLATFORM),linux-x64)
RUST_HOST_TARGET := x86_64-unknown-linux-gnu
else ifeq ($(HOST_PLATFORM),linux-arm64)
RUST_HOST_TARGET := aarch64-unknown-linux-gnu
else ifeq ($(HOST_PLATFORM),mac-x64)
RUST_HOST_TARGET := x86_64-apple-darwin
else ifeq ($(HOST_PLATFORM),mac-arm64)
RUST_HOST_TARGET := aarch64-apple-darwin
else
$(error Unsupported host platform '$(HOST_PLATFORM)')
endif

SUPPORTED_TARGET_PLATFORMS := linux-x64 linux-arm64 mac-x64 mac-arm64

# Optional untracked override. Until cross-compilation exists it can only
# set TARGET_PLATFORM to this host; any other value hits the mismatch error.
-include $(ROOT)/build/config/local/target.mk

TARGET_PLATFORM ?= $(HOST_PLATFORM)

ifeq ($(filter $(TARGET_PLATFORM),$(SUPPORTED_TARGET_PLATFORMS)),)
$(error Unsupported target '$(TARGET_PLATFORM)'. Supported: $(SUPPORTED_TARGET_PLATFORMS))
endif

ifneq ($(TARGET_PLATFORM),$(HOST_PLATFORM))
$(error Target '$(TARGET_PLATFORM)' does not match this host '$(HOST_PLATFORM)'. Build on that OS)
endif

OUT_DIR := $(ROOT)/out/$(TARGET_PLATFORM)
export CARGO_TARGET_DIR := $(OUT_DIR)
