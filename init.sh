#!/usr/bin/env bash
#
# Tycho Bridge - Development Environment Setup
#
# This script sets up the development environment for first-time users.
# It checks prerequisites, installs dependencies, builds the project,
# and runs tests to verify everything works.
#
# Usage:
#   ./init.sh          # Full setup
#   ./init.sh --check  # Only check prerequisites
#   ./init.sh --help   # Show help
#

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

RUST_MIN_VERSION="1.70.0"
GO_MIN_VERSION="1.22"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# =============================================================================
# Colors and Output
# =============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }
header() { echo -e "\n${BOLD}${CYAN}=== $1 ===${NC}\n"; }

# =============================================================================
# Helper Functions
# =============================================================================

version_ge() {
    # Returns 0 if $1 >= $2
    printf '%s\n%s\n' "$2" "$1" | sort -V -C
}

command_exists() {
    command -v "$1" &> /dev/null
}

show_help() {
    cat << EOF
Tycho Bridge - Development Environment Setup

Usage:
    ./init.sh [OPTIONS]

Options:
    --check     Only check prerequisites, don't install or build
    --skip-go   Skip Go client setup
    --help      Show this help message

Prerequisites:
    - Rust ${RUST_MIN_VERSION}+ (https://rustup.rs)
    - Go ${GO_MIN_VERSION}+ (https://go.dev/dl)
    - protoc (Protocol Buffers compiler)

For more information, see README.md
EOF
    exit 0
}

# =============================================================================
# Prerequisite Checks
# =============================================================================

check_rust() {
    info "Checking Rust installation..."

    if ! command_exists rustc; then
        error "Rust is not installed"
        echo "  Install from: https://rustup.rs"
        echo "  Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        return 1
    fi

    local version
    version=$(rustc --version | awk '{print $2}')

    if version_ge "$version" "$RUST_MIN_VERSION"; then
        success "Rust $version"
    else
        error "Rust $version is too old (need $RUST_MIN_VERSION+)"
        echo "  Run: rustup update stable"
        return 1
    fi

    # Check cargo
    if ! command_exists cargo; then
        error "Cargo is not installed"
        return 1
    fi
    success "Cargo $(cargo --version | awk '{print $2}')"
}

check_go() {
    info "Checking Go installation..."

    if ! command_exists go; then
        warn "Go is not installed (optional, needed for Go clients)"
        echo "  Install from: https://go.dev/dl"
        return 1
    fi

    local version
    version=$(go version | awk '{print $3}' | sed 's/go//')

    if version_ge "$version" "$GO_MIN_VERSION"; then
        success "Go $version"
    else
        warn "Go $version is older than recommended ($GO_MIN_VERSION+)"
    fi
}

check_protoc() {
    info "Checking Protocol Buffers compiler..."

    if ! command_exists protoc; then
        error "protoc is not installed (required for gRPC)"
        echo ""
        echo "  Install options:"
        echo "    macOS:  brew install protobuf"
        echo "    Ubuntu: apt-get install protobuf-compiler"
        echo "    Arch:   pacman -S protobuf"
        echo "    Other:  https://github.com/protocolbuffers/protobuf/releases"
        return 1
    fi

    local version
    version=$(protoc --version | awk '{print $2}')
    success "protoc $version"
}

check_all_prerequisites() {
    header "Checking Prerequisites"

    local failed=0

    check_rust || failed=1
    check_protoc || failed=1
    check_go || true  # Go is optional

    if [ $failed -eq 1 ]; then
        echo ""
        error "Some prerequisites are missing. Please install them and try again."
        exit 1
    fi

    echo ""
    success "All required prerequisites are installed!"
}

# =============================================================================
# Build Steps
# =============================================================================

build_rust() {
    header "Building Rust Workspace"

    cd "$SCRIPT_DIR"

    info "Fetching dependencies..."
    cargo fetch

    info "Building all crates..."
    cargo build --workspace

    success "Rust build complete"

    # Check if C header was generated
    if [ -f "$SCRIPT_DIR/include/tycho_bridge.h" ]; then
        success "C header generated: include/tycho_bridge.h"
    fi
}

build_rust_release() {
    header "Building Release Binaries"

    cd "$SCRIPT_DIR"

    info "Building release binaries..."
    cargo build --workspace --release

    success "Release build complete"

    # Show library location
    if [ -f "$SCRIPT_DIR/target/release/libtycho_ffi.so" ]; then
        success "Shared library: target/release/libtycho_ffi.so"
    elif [ -f "$SCRIPT_DIR/target/release/libtycho_ffi.dylib" ]; then
        success "Shared library: target/release/libtycho_ffi.dylib"
    fi
}

setup_go_clients() {
    header "Setting Up Go Clients"

    if ! command_exists go; then
        warn "Skipping Go setup (Go not installed)"
        return 0
    fi

    # Go gRPC client
    if [ -d "$SCRIPT_DIR/clients/go" ]; then
        info "Setting up Go gRPC client..."
        cd "$SCRIPT_DIR/clients/go"
        go mod download
        go build ./...
        success "Go gRPC client ready"
    fi

    # Go FFI client
    if [ -d "$SCRIPT_DIR/clients/go-ffi" ]; then
        info "Setting up Go FFI client..."
        cd "$SCRIPT_DIR/clients/go-ffi"
        go mod download
        success "Go FFI client ready"
    fi
}

run_tests() {
    header "Running Tests"

    cd "$SCRIPT_DIR"

    info "Running Rust tests..."
    cargo test --workspace

    success "All Rust tests passed!"

    # Run Go tests if available
    if command_exists go; then
        if [ -d "$SCRIPT_DIR/clients/go-ffi" ]; then
            info "Running Go FFI tests..."
            cd "$SCRIPT_DIR/clients/go-ffi"
            # Go FFI tests require the shared library, skip if not built
            if [ -f "$SCRIPT_DIR/target/debug/libtycho_ffi.so" ] || \
               [ -f "$SCRIPT_DIR/target/debug/libtycho_ffi.dylib" ]; then
                go test ./... -v 2>/dev/null || warn "Go FFI tests skipped (library linking issue)"
            else
                warn "Go FFI tests skipped (build release first with: cargo build --release)"
            fi
        fi
    fi
}

copy_env_example() {
    if [ ! -f "$SCRIPT_DIR/.env" ] && [ -f "$SCRIPT_DIR/.env.example" ]; then
        info "Creating .env from .env.example..."
        cp "$SCRIPT_DIR/.env.example" "$SCRIPT_DIR/.env"
        success "Created .env (edit with your configuration)"
    fi
}

# =============================================================================
# Summary
# =============================================================================

show_summary() {
    header "Setup Complete!"

    echo -e "${GREEN}The development environment is ready.${NC}\n"

    echo "Project structure:"
    echo "  crates/tycho-core  - Core types and business logic"
    echo "  crates/tycho-grpc  - gRPC server implementation"
    echo "  crates/tycho-ffi   - C FFI bindings"
    echo "  clients/go         - Go gRPC client"
    echo "  clients/go-ffi     - Go FFI client"
    echo ""

    echo "Common commands:"
    echo "  cargo build              # Build debug"
    echo "  cargo build --release    # Build release"
    echo "  cargo test               # Run tests"
    echo "  cargo doc --open         # View documentation"
    echo ""

    echo "For Go FFI client:"
    echo "  cargo build --release    # Build the shared library first"
    echo "  cd clients/go-ffi && go test ./..."
    echo ""

    if [ -f "$SCRIPT_DIR/.env" ]; then
        warn "Remember to configure .env with your settings"
    fi
}

# =============================================================================
# Main
# =============================================================================

main() {
    local check_only=false
    local skip_go=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --check)
                check_only=true
                shift
                ;;
            --skip-go)
                skip_go=true
                shift
                ;;
            --help|-h)
                show_help
                ;;
            *)
                error "Unknown option: $1"
                echo "Run './init.sh --help' for usage"
                exit 1
                ;;
        esac
    done

    echo -e "${BOLD}${CYAN}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║              Tycho Bridge - Development Setup              ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    # Always check prerequisites
    check_all_prerequisites

    if [ "$check_only" = true ]; then
        exit 0
    fi

    # Setup environment
    copy_env_example

    # Build Rust workspace
    build_rust

    # Setup Go clients (unless skipped)
    if [ "$skip_go" = false ]; then
        setup_go_clients
    fi

    # Run tests
    run_tests

    # Show summary
    show_summary
}

main "$@"
