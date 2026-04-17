# Build stage
FROM rust:1.95-alpine AS builder

# Add build dependencies
RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY . .

# Build the application
RUN cargo build --release

# Run stage
FROM alpine:3.23

# Set Time Zone to IST
ENV TZ="Asia/Kolkata"

# Add required runtime packages
RUN apk add --no-cache curl ca-certificates tzdata

# Set Working Directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/netviz /app/netviz

# Copy templates
COPY templates /app/templates

# Add user
RUN addgroup -S netvizgroup && adduser -S netvizuser -G netvizgroup

# Create data directory and set permissions
RUN mkdir -p /app/data/peeringdb && chown -R netvizuser:netvizgroup /app

# Switch to the non-root user
USER netvizuser

# Expose Port
EXPOSE 8201

CMD ["/app/netviz"]
