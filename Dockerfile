# ==============================================================================
# Tycho Bridge Service - Multi-Stage Dockerfile (Optimized)
# ==============================================================================
#
# This Dockerfile creates a minimal production image for the Tycho Bridge gRPC
# sidecar service. It follows best practices for containerized Rust applications:
#
# - Multi-stage build with `cargo-chef` for optimal layer caching
# - Distroless runtime for security (no shell, minimal attack surface)
# - SSL certificates included for secure WebSocket connections
# - Non-root user for defense in depth
#
# Build:
#   docker build -t tycho-bridge:latest .
#
# Run:
#   docker run -e TYCHO_API_KEY=<key> -p 50051:50051 -p 9000:9000 tycho-bridge:latest
#
# ==============================================================================

# ==============================================================================
# Stage 1: Chef - Toolchain Preparation
# ==============================================================================
# We use the official Rust image to install cargo-chef.
# cargo-chef is used to pre-calculate dependencies and cache them, 
# drastically reducing build times when only source code changes.
FROM rustlang/rust:nightly-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# ==============================================================================
# Stage 2: Planner - Generate Recipe
# ==============================================================================
# In this stage, we scan the source code to generate a "recipe.json".
# This file contains a lockfile-like description of our dependencies.
# We do this in a separate stage so that the subsequent build stage 
# only invalidates its cache if the *dependencies* change, not the source code.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# Stage 3: Builder - Compile the Rust binary
# ==============================================================================
FROM chef AS builder

# Install system dependencies required for compilation:
# - protobuf-compiler: Required by tonic-build for gRPC code generation
# - pkg-config: Build tool for linking C libraries
# - libssl-dev: OpenSSL development files for TLS support
# - cmake: Required by some Rust crates (e.g., aws-lc-sys)
# - clang: Required by blst and other crypto crates
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Copy the dependency recipe from the Planner stage
COPY --from=planner /app/recipe.json recipe.json

# Cook the dependencies!
# This builds all dependencies (crates.io index, download, compile) 
# and caches the result in a Docker layer. This layer will NOT be rebuilt 
# unless Cargo.toml or Cargo.lock changes.
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the actual source code
# This is done *after* dependency cooking so that source changes 
# do not invalidate the heavy dependency build layer above.
COPY . .

# Build the application in release mode
# Since dependencies are already compiled in the previous step, 
# this step only compiles our local source code (Tycho Bridge).
RUN cargo build --release

# Verify the binary was built successfully
RUN test -f target/release/tycho-bridge

# ==============================================================================
# Stage 4: Runtime - Minimal production image
# ==============================================================================
# Using distroless/cc image which includes:
# - glibc: Required for most compiled binaries
# - libgcc: Required for Rust binaries
# - ca-certificates: Required for TLS/SSL connections
#
# This image does NOT include:
# - Shell (no /bin/sh, /bin/bash)
# - Package manager
# - Any other unnecessary tools
FROM gcr.io/distroless/cc-debian12:nonroot

# Labels for container metadata (OCI spec)
LABEL org.opencontainers.image.title="Tycho Bridge Service"
LABEL org.opencontainers.image.description="gRPC sidecar for Tycho indexer - Remote State Stream / Local Execution"
LABEL org.opencontainers.image.version="0.1.0"
LABEL org.opencontainers.image.source="https://github.com/branched-services/tycho-bridge"
LABEL org.opencontainers.image.licenses="MIT"

# Copy the compiled binary from the builder stage
# The nonroot variant already has a non-root user (uid 65532)
COPY --from=builder /app/target/release/tycho-bridge /usr/local/bin/tycho-bridge

# Expose the gRPC and metrics ports
# - 50051: gRPC server (configurable via GRPC_PORT)
# - 9000: Prometheus metrics (configurable via METRICS_PORT)
EXPOSE 50051 9000

# Set environment variable defaults
# These can be overridden at runtime with -e flags
ENV GRPC_HOST=0.0.0.0 \
    GRPC_PORT=50051 \
    METRICS_PORT=9000 \
    RUST_LOG=info

# The distroless nonroot image runs as user 65532 by default
# This is already configured in the base image

# Run the application
# Using exec form to ensure signals are properly forwarded
ENTRYPOINT ["/usr/local/bin/tycho-bridge"]