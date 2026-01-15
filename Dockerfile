# Build stage
FROM rust:1.92-slim AS builder

# Add build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential

WORKDIR /app
COPY . .

# Build the application
RUN cargo build --release

# Run stage
FROM debian:bookworm-slim

# Set Time Zone to IST
ENV TZ="Asia/Kolkata"

# Set logging level
ENV RUST_LOG="info"

# Add required runtime packages
RUN apt-get update && \
    apt-get install --yes --no-install-recommends \
    curl ca-certificates openssl && \
    rm -rf /var/lib/apt/lists/* /tmp/*

# Set Working Directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/netviz /app/netviz

# Copy templates
COPY templates /app/templates

# Add user
RUN groupadd --system netvizgroup && useradd --system --gid netvizgroup netvizuser --create-home

# Create data directory and set permissions
RUN mkdir -p /app/data/peeringdb && chown -R netvizuser:netvizgroup /app

# Switch to the non-root user
USER netvizuser

# Expose Port
EXPOSE 8201

CMD ["/app/netviz"]
