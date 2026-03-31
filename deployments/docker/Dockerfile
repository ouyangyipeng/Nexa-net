# Nexa-net Dockerfile
# Multi-stage build for optimized image size

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn add(left: u64, right: u64) -> u64 { left + right }" > src/lib.rs

# Build dependencies
RUN cargo build --release && rm -rf src

# Copy actual source
COPY src ./src
COPY proto ./proto
COPY build.rs ./

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/nexa-proxy /usr/local/bin/
COPY --from=builder /app/target/release/libnexa_net.rlib /usr/local/lib/ 2>/dev/null || true

# Create non-root user
RUN useradd -r -s /bin/false nexa && \
    mkdir -p /var/lib/nexa /var/log/nexa /etc/nexa && \
    chown -R nexa:nexa /var/lib/nexa /var/log/nexa /etc/nexa

USER nexa

# Expose ports
# 7070: REST API
# 7071: gRPC API
# 9090: Metrics
EXPOSE 7070 7071 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:7070/api/v1/health || exit 1

# Default configuration
ENV NEXA_PROXY_ENDPOINT=0.0.0.0:7070
ENV NEXA_PROXY_GRPC_ENDPOINT=0.0.0.0:7071
ENV NEXA_PROXY_METRICS_ENDPOINT=0.0.0.0:9090
ENV RUST_LOG=info

# Entry point
ENTRYPOINT ["nexa-proxy"]
CMD ["--config", "/etc/nexa/config.yaml"]