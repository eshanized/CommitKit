# Makefile for ck - Intelligent Git Commit Assistant
# Supports: Windows (Git Bash/WSL), MacOS, Arch Linux, Debian/Ubuntu, Redhat/Fedora

# Variables
CARGO := cargo
BIN_NAME := ck
INSTALL_PATH ?= /usr/local/bin
TARGET_DIR := target/release

# Windows-specific overrides
ifeq ($(OS),Windows_NT)
    BIN_EXT := .exe
    INSTALL_PATH := $(HOMEDRIVE)$(HOMEPATH)/.cargo/bin
else
    BIN_EXT :=
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Linux)
        DISTRO_ID := $(shell grep -oP '(?<=^ID=).+' /etc/os-release 2>/dev/null | tr -d '"')
    endif
endif

.PHONY: all build test clean install lint fmt help install-deps release-build

# Default target
all: build

# Help
help:
	@echo "Available targets:"
	@echo "  build         - Build release binary"
	@echo "  test          - Run all tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  lint          - Run clippy linting"
	@echo "  fmt           - Format code"
	@echo "  install       - Install binary to system path"
	@echo "  install-deps  - Install system dependencies (Linux only)"

# Build
build:
	$(CARGO) build --release

# Test
test:
	$(CARGO) test --all-features

# Clean
clean:
	$(CARGO) clean

# Lint
lint:
	$(CARGO) clippy --all-features -- -D warnings

# Format
fmt:
	$(CARGO) fmt

# Install
install: build
	@echo "Installing $(BIN_NAME) to $(INSTALL_PATH)..."
ifeq ($(OS),Windows_NT)
	@if not exist "$(INSTALL_PATH)" mkdir "$(INSTALL_PATH)"
	@copy "$(TARGET_DIR)/$(BIN_NAME)$(BIN_EXT)" "$(INSTALL_PATH)/$(BIN_NAME)$(BIN_EXT)"
else
	@mkdir -p $(INSTALL_PATH)
	@install -m 755 $(TARGET_DIR)/$(BIN_NAME)$(BIN_EXT) $(INSTALL_PATH)/$(BIN_NAME)$(BIN_EXT)
endif
	@echo "Installation complete!"

# Dependencies (Linux specific)
install-deps:
ifeq ($(UNAME_S),Linux)
	@echo "Detected Linux distribution: $(DISTRO_ID)"
	@if [ "$(DISTRO_ID)" = "arch" ] || [ "$(DISTRO_ID)" = "manjaro" ]; then \
		sudo pacman -S --needed base-devel openssl zlib cmake pkgconf; \
	elif [ "$(DISTRO_ID)" = "debian" ] || [ "$(DISTRO_ID)" = "ubuntu" ] || [ "$(DISTRO_ID)" = "pop" ]; then \
		sudo apt-get update && sudo apt-get install -y build-essential libssl-dev pkg-config cmake zlib1g-dev; \
	elif [ "$(DISTRO_ID)" = "fedora" ] || [ "$(DISTRO_ID)" = "rhel" ] || [ "$(DISTRO_ID)" = "centos" ]; then \
		sudo dnf install -y openssl-devel cmake gcc gcc-c++ pkgconfig zlib-devel; \
	else \
		echo "Unsupported or unknown distribution: $(DISTRO_ID). Please install dependencies manually."; \
	fi
else ifeq ($(UNAME_S),Darwin)
	@echo "Detected MacOS. Checking for Homebrew..."
	@if which brew >/dev/null; then \
		brew install openssl cmake pkg-config; \
	else \
		echo "Homebrew not found. Please install dependencies manually."; \
	fi
else
	@echo "Dependency installation is better handled via Cargo or manual setup on this platform."
endif
