# =============================================================================
# Drone Convoy Tracking Server - Multi-stage Dockerfile
# =============================================================================
# Stage 1: Build OpenCV and Rust application
# Stage 2: Runtime with minimal dependencies
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM rust:1.83-bookworm AS builder

# Install build dependencies for OpenCV
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    pkg-config \
    libclang-dev \
    clang \
    # OpenCV dependencies
    libopencv-dev \
    libopencv-contrib-dev \
    # SSL
    libssl-dev \
    # Other
    && rm -rf /var/lib/apt/lists/*

# Set environment for OpenCV
ENV OPENCV_LINK_LIBS=opencv_core,opencv_imgproc,opencv_highgui,opencv_imgcodecs,opencv_videoio,opencv_video,opencv_features2d,opencv_calib3d,opencv_objdetect
ENV OPENCV_LINK_PATHS=/usr/lib/x86_64-linux-gnu
ENV OPENCV_INCLUDE_PATHS=/usr/include/opencv4

# Create app directory
WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock* ./
COPY crates/drone-core/Cargo.toml ./crates/drone-core/
COPY crates/drone-cv/Cargo.toml ./crates/drone-cv/
COPY crates/drone-db/Cargo.toml ./crates/drone-db/
COPY crates/drone-api/Cargo.toml ./crates/drone-api/
COPY crates/drone-websocket/Cargo.toml ./crates/drone-websocket/
COPY crates/drone-p2p/Cargo.toml ./crates/drone-p2p/
COPY crates/drone-telemetry/Cargo.toml ./crates/drone-telemetry/
COPY crates/drone-tracker/Cargo.toml ./crates/drone-tracker/

# Create dummy source files for dependency caching
RUN mkdir -p crates/drone-core/src && echo "pub fn dummy() {}" > crates/drone-core/src/lib.rs
RUN mkdir -p crates/drone-cv/src && echo "pub fn dummy() {}" > crates/drone-cv/src/lib.rs
RUN mkdir -p crates/drone-db/src && echo "pub fn dummy() {}" > crates/drone-db/src/lib.rs
RUN mkdir -p crates/drone-api/src && echo "fn main() {}" > crates/drone-api/src/main.rs
RUN mkdir -p crates/drone-websocket/src && echo "pub fn dummy() {}" > crates/drone-websocket/src/lib.rs
RUN mkdir -p crates/drone-p2p/src && echo "pub fn dummy() {}" > crates/drone-p2p/src/lib.rs
RUN mkdir -p crates/drone-telemetry/src && echo "pub fn dummy() {}" > crates/drone-telemetry/src/lib.rs
RUN mkdir -p crates/drone-tracker/src && echo "pub fn dummy() {}" > crates/drone-tracker/src/lib.rs

# Build dependencies (this layer is cached)
RUN cargo build --release --workspace 2>/dev/null || true

# Copy actual source code
COPY crates/ ./crates/

# Touch source files to ensure rebuild
RUN find crates -name "*.rs" -exec touch {} \;

# Build the application
RUN cargo build --release --workspace

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libopencv-core406 \
    libopencv-imgproc406 \
    libopencv-highgui406 \
    libopencv-imgcodecs406 \
    libopencv-videoio406 \
    libopencv-video406 \
    libopencv-features2d406 \
    libopencv-calib3d406 \
    libopencv-objdetect406 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 drone
USER drone

# Set working directory
WORKDIR /app

# Copy built binaries
COPY --from=builder /app/target/release/drone-api /app/drone-api

# Set environment
ENV RUST_LOG=info,drone_api=debug,drone_cv=debug
ENV API_PORT=3000
ENV WS_PORT=9090

# Expose ports
EXPOSE 3000 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the API server
CMD ["/app/drone-api"]

# -----------------------------------------------------------------------------
# Stage: Development (optional)
# -----------------------------------------------------------------------------
FROM builder AS development

# Install additional dev tools
RUN cargo install cargo-watch

WORKDIR /app

# Keep source mounted
VOLUME ["/app"]

CMD ["cargo", "watch", "-x", "run --bin drone-api"]
