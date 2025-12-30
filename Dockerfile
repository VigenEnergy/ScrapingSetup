# Build stage - using Alpine/musl for static linking
FROM rust:1.83-alpine AS builder

WORKDIR /app

# Install build dependencies for musl static compilation
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    git

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Configure git and build with token for private repos
# GitHub App tokens require x-access-token format
# Build with musl target for fully static binary
RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    git config --global url."https://x-access-token:$(cat /run/secrets/GIT_AUTH_TOKEN)@github.com/".insteadOf "https://github.com/" && \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    OPENSSL_STATIC=1 \
    cargo build --release

# Runtime stage - minimal Alpine image
FROM alpine:3.20

WORKDIR /app

# Install CA certificates for HTTPS
RUN apk add --no-cache ca-certificates

# Copy the binary from builder
COPY --from=builder /app/target/release/scraping_service /usr/local/bin/scraping_service

# Copy config file (can be overridden with volume mount)
COPY config.json ./config.json

# Create data and log directories
RUN mkdir -p /app/data /app/logs

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["scraping_service"]
