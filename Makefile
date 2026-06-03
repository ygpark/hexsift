# Makefile for hexsift
# Traditional make command support

.PHONY: all build release install test test-all test-ignored clean check fmt clippy dev-test pre-commit ci help \
	build-linux build-windows build-macos build-all-platforms \
	build-linux-musl build-windows-gnu build-arm64 add-targets

BINARY_NAME := hexsift

ifeq ($(OS),Windows_NT)
	EXE_EXT := .exe
	INSTALL_DIR ?= C:/bin
else
	EXE_EXT :=
	INSTALL_DIR ?= /usr/local/bin
endif

RELEASE_BIN := target/release/$(BINARY_NAME)$(EXE_EXT)
INSTALLED_BIN := $(INSTALL_DIR)/$(BINARY_NAME)$(EXE_EXT)

# Default target
all: build test

# Build
build:
	@echo "Building debug..."
	cargo build

release:
	@echo "Building release..."
	cargo build --release

install: release
	@echo "Installing $(INSTALLED_BIN)..."
ifeq ($(OS),Windows_NT)
	powershell -NoProfile -Command "New-Item -ItemType Directory -Force '$(INSTALL_DIR)' | Out-Null; Copy-Item -Force '$(RELEASE_BIN)' '$(INSTALLED_BIN)'"
else
	install -d "$(DESTDIR)$(INSTALL_DIR)"
	install -m 755 "$(RELEASE_BIN)" "$(DESTDIR)$(INSTALLED_BIN)"
endif
	@echo "Installed: $(INSTALLED_BIN)"

# Test
test:
	@echo "Running tests..."
	cargo test

test-all:
	@echo "Running all tests..."
	cargo test
	cargo test -- --ignored

# Test ignored cases
test-ignored:
	@echo "Running ignored tests..."
	cargo test -- --ignored

# Development tools
clean:
	@echo "Cleaning..."
	cargo clean

check:
	@echo "Checking..."
	cargo check

fmt:
	@echo "Formatting..."
	cargo fmt

clippy:
	@echo "Linting..."
	cargo clippy

# Combined commands
dev-test: test clippy
	@echo "Developer checks complete."

pre-commit: fmt clippy test
	@echo "Pre-commit checks complete."

ci: clean build test-all
	@echo "CI pipeline complete."

# Cross-platform builds
build-linux:
	@echo "Building Linux x86_64..."
	cargo build --release --target x86_64-unknown-linux-gnu

build-linux-musl:
	@echo "Building Linux x86_64 (musl)..."
	cargo build --release --target x86_64-unknown-linux-musl

build-windows:
	@echo "Building Windows x86_64..."
	cargo build --release --target x86_64-pc-windows-msvc

build-windows-gnu:
	@echo "Building Windows x86_64 (GNU)..."
	cargo build --release --target x86_64-pc-windows-gnu

build-macos:
	@echo "Building macOS x86_64..."
	cargo build --release --target x86_64-apple-darwin

build-arm64:
	@echo "Building ARM64..."
	cargo build --release --target aarch64-unknown-linux-gnu
	cargo build --release --target aarch64-apple-darwin

build-all-platforms: build-linux build-linux-musl build-windows build-macos build-arm64
	@echo "All platform builds complete."

# Add targets when needed
add-targets:
	@echo "Adding cross-compile targets..."
	rustup target add x86_64-unknown-linux-gnu
	rustup target add x86_64-unknown-linux-musl
	rustup target add x86_64-pc-windows-msvc
	rustup target add x86_64-pc-windows-gnu
	rustup target add x86_64-apple-darwin
	rustup target add aarch64-unknown-linux-gnu
	rustup target add aarch64-apple-darwin
	@echo "Targets added."

# Help
help:
	@echo "Available commands:"
	@echo ""
	@echo "Build:"
	@echo "  make build   - Debug build"
	@echo "  make release - Release build"
	@echo "  make install - Build release and install the binary"
	@echo "                 Override path with INSTALL_DIR=/path/to/bin"
	@echo ""
	@echo "Cross-platform builds:"
	@echo "  make build-linux        - Linux x86_64"
	@echo "  make build-linux-musl   - Linux x86_64 (musl)"
	@echo "  make build-windows      - Windows x86_64 (MSVC)"
	@echo "  make build-windows-gnu  - Windows x86_64 (GNU)"
	@echo "  make build-macos        - macOS x86_64"
	@echo "  make build-arm64        - ARM64 (Linux/macOS)"
	@echo "  make build-all-platforms - All platforms"
	@echo "  make add-targets        - Add cross-compile targets"
	@echo ""
	@echo "Tests:"
	@echo "  make test          - Default tests"
	@echo "  make test-all      - All tests, including ignored tests"
	@echo ""
	@echo "Development:"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make check         - Check syntax"
	@echo "  make fmt           - Format code"
	@echo "  make clippy        - Lint"
	@echo ""
	@echo "Combined:"
	@echo "  make dev-test      - Developer checks"
	@echo "  make pre-commit    - Pre-commit checks"
	@echo "  make ci            - CI pipeline"
