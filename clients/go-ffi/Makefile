.PHONY: all build test test-short clean lib help

# Paths
RUST_TARGET = ../../target/release
RUST_LIB = $(RUST_TARGET)/libtycho_bridge.so
INCLUDE_DIR = ../../include

# Go environment
export CGO_ENABLED = 1
export CGO_CFLAGS = -I$(INCLUDE_DIR)
export CGO_LDFLAGS = -L$(RUST_TARGET) -ltycho_bridge -ldl -lm -lpthread
export LD_LIBRARY_PATH := $(RUST_TARGET):$(LD_LIBRARY_PATH)

# Default target
all: lib build

help:
	@echo "Tycho Bridge Go FFI Client"
	@echo ""
	@echo "Usage:"
	@echo "  make all        - Build Rust library and Go package"
	@echo "  make lib        - Build Rust FFI library"
	@echo "  make build      - Build Go package"
	@echo "  make test       - Run all tests (requires Rust lib)"
	@echo "  make test-short - Run unit tests only (no FFI required)"
	@echo "  make clean      - Clean build artifacts"
	@echo ""
	@echo "Requirements:"
	@echo "  - Rust toolchain (cargo)"
	@echo "  - Go 1.22+"
	@echo "  - CGO enabled"

# Build Rust FFI library
lib: $(RUST_LIB)

$(RUST_LIB):
	cd ../.. && cargo build -p tycho-ffi --release

# Build Go package
build: $(RUST_LIB)
	go build ./...

# Run all tests
test: $(RUST_LIB)
	go test -v ./...

# Run unit tests only (no FFI required)
test-short:
	go test -v -short ./...

# Run tests with race detector
test-race: $(RUST_LIB)
	go test -v -race ./...

# Run benchmarks
bench: $(RUST_LIB)
	go test -v -bench=. -benchmem ./...

# Generate coverage report
coverage: $(RUST_LIB)
	go test -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html

# Clean build artifacts
clean:
	go clean
	rm -f coverage.out coverage.html

# Verify the library is properly linked
verify: $(RUST_LIB)
	@echo "Checking library symbols..."
	nm -D $(RUST_LIB) | grep -E "^[0-9a-f]+ T tycho_" | head -20
	@echo ""
	@echo "Library verified successfully!"

# Format code
fmt:
	go fmt ./...

# Run linter
lint:
	golangci-lint run ./...

# Install development tools
tools:
	go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Create release archive
release: lib build test
	@mkdir -p dist
	@cp $(RUST_LIB) dist/
	@cp $(INCLUDE_DIR)/tycho_bridge.h dist/
	@echo "Release files in dist/"
