# Makefile for hexsift
# 전통적인 make 명령어 지원

.PHONY: all build build-release test test-all clean check fmt clippy dev-test pre-commit ci help \
	build-linux build-windows build-macos build-all-platforms \
	build-linux-musl build-windows-gnu build-arm64

# 기본 타겟
all: build test

# 빌드 관련
build:
	@echo "🔨 디버그 빌드 중..."
	cargo build

build-release:
	@echo "🚀 릴리즈 빌드 중..."
	cargo build --release

# 테스트 관련
test:
	@echo "🧪 테스트 실행 중..."
	cargo test

test-all:
	@echo "🧪 모든 테스트 실행 중..."
	cargo test
	cargo test -- --ignored

# 테스트 관련 (이어서)
test-ignored:
	@echo "🧪 무시된 테스트 실행 중..."
	cargo test -- --ignored

# 개발 도구
clean:
	@echo "🧹 정리 중..."
	cargo clean

check:
	@echo "🔍 문법 검사 중..."
	cargo check

fmt:
	@echo "🎨 코드 포맷팅 중..."
	cargo fmt

clippy:
	@echo "📎 린팅 중..."
	cargo clippy

# 통합 명령어
dev-test: test clippy
	@echo "✅ 개발자 테스트 완료!"

pre-commit: fmt clippy test
	@echo "✅ 커밋 전 검사 완료!"

ci: clean build test-all
	@echo "✅ CI 파이프라인 완료!"

# 크로스 플랫폼 빌드
build-linux:
	@echo "🐧 Linux x86_64 빌드 중..."
	cargo build --release --target x86_64-unknown-linux-gnu

build-linux-musl:
	@echo "🐧 Linux x86_64 (musl) 빌드 중..."
	cargo build --release --target x86_64-unknown-linux-musl

build-windows:
	@echo "🪟 Windows x86_64 빌드 중..."
	cargo build --release --target x86_64-pc-windows-msvc

build-windows-gnu:
	@echo "🪟 Windows x86_64 (GNU) 빌드 중..."
	cargo build --release --target x86_64-pc-windows-gnu

build-macos:
	@echo "🍎 macOS x86_64 빌드 중..."
	cargo build --release --target x86_64-apple-darwin

build-arm64:
	@echo "🦾 ARM64 빌드 중..."
	cargo build --release --target aarch64-unknown-linux-gnu
	cargo build --release --target aarch64-apple-darwin

build-all-platforms: build-linux build-linux-musl build-windows build-macos build-arm64
	@echo "🌍 모든 플랫폼 빌드 완료!"

# 타겟 추가 (필요시 사용)
add-targets:
	@echo "📦 크로스 컴파일 타겟 추가 중..."
	rustup target add x86_64-unknown-linux-gnu
	rustup target add x86_64-unknown-linux-musl
	rustup target add x86_64-pc-windows-msvc
	rustup target add x86_64-pc-windows-gnu
	rustup target add x86_64-apple-darwin
	rustup target add aarch64-unknown-linux-gnu
	rustup target add aarch64-apple-darwin
	@echo "✅ 타겟 추가 완료!"

# 도움말
help:
	@echo "📋 사용 가능한 명령어들:"
	@echo ""
	@echo "🔨 빌드:"
	@echo "  make build         - 디버그 빌드"
	@echo "  make build-release - 릴리즈 빌드"
	@echo ""
	@echo "🌍 크로스 플랫폼 빌드:"
	@echo "  make build-linux        - Linux x86_64"
	@echo "  make build-linux-musl   - Linux x86_64 (musl)"
	@echo "  make build-windows      - Windows x86_64 (MSVC)"
	@echo "  make build-windows-gnu  - Windows x86_64 (GNU)"
	@echo "  make build-macos        - macOS x86_64"
	@echo "  make build-arm64        - ARM64 (Linux/macOS)"
	@echo "  make build-all-platforms - 모든 플랫폼"
	@echo "  make add-targets        - 크로스 컴파일 타겟 추가"
	@echo ""
	@echo "🧪 테스트:"
	@echo "  make test          - 기본 테스트"
	@echo "  make test-all      - 모든 테스트 (무시된 것 포함)"
	@echo ""
	@echo "🛠  개발:"
	@echo "  make clean         - 빌드 정리"
	@echo "  make check         - 문법 검사"
	@echo "  make fmt           - 코드 포맷팅"
	@echo "  make clippy        - 린팅"
	@echo ""
	@echo "🔄 통합:"
	@echo "  make dev-test      - 개발자 테스트"
	@echo "  make pre-commit    - 커밋 전 검사"
	@echo "  make ci            - CI 파이프라인"