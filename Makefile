BIN := apple-notes-mcp
CARGO ?= cargo
NPX ?= npx
INSPECTOR_PKG ?= @modelcontextprotocol/inspector

TARGET_X64 := x86_64-apple-darwin
TARGET_ARM64 := aarch64-apple-darwin

.PHONY: build build-debug build-release \
	build-debug-x64 build-debug-arm64 build-release-x64 build-release-arm64 \
	inspector-debug inspector-release clean

build: build-debug build-release

build-debug: build-debug-x64 build-debug-arm64

build-release: build-release-x64 build-release-arm64

build-debug-x64:
	$(CARGO) build --target $(TARGET_X64)

build-debug-arm64:
	$(CARGO) build --target $(TARGET_ARM64)

build-release-x64:
	$(CARGO) build --release --target $(TARGET_X64)

build-release-arm64:
	$(CARGO) build --release --target $(TARGET_ARM64)

inspector-debug:
	$(CARGO) build --target $(TARGET_ARM64)
	$(NPX) -y $(INSPECTOR_PKG) $(abspath target/$(TARGET_ARM64)/debug/$(BIN))

inspector-release:
	$(CARGO) build --release --target $(TARGET_ARM64)
	$(NPX) -y $(INSPECTOR_PKG) $(abspath target/$(TARGET_ARM64)/release/$(BIN))

clean:
	$(CARGO) clean
