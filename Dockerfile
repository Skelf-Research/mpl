# MPL Proxy Dockerfile
# Multi-stage build for minimal image size

FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binaries
RUN cargo build --release --package mpl-proxy --package mplx

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/mpl-proxy /usr/local/bin/
COPY --from=builder /app/target/release/mpl /usr/local/bin/mpl

# Copy registry
COPY registry /app/registry

# Copy default config
COPY mpl-config.yaml /app/mpl-config.yaml

# Expose ports
EXPOSE 9443 9100

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9443/health || exit 1

# Run proxy
ENTRYPOINT ["mpl-proxy"]
CMD ["--config", "/app/mpl-config.yaml"]
