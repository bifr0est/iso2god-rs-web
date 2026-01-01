# Stage 1: Build the application
FROM rust:latest as builder

WORKDIR /usr/src/iso2god-rs

# Copy the source code
COPY . .

# Build the release binary
RUN cargo build --release --bin iso2god-web

# Stage 2: Create the final image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/iso2god-rs/target/release/iso2god-web .

# Copy the templates and public directories
COPY templates ./templates
COPY public ./public

# Create directories for mounted volumes
RUN mkdir -p /data/input /data/output && \
    chmod -R 777 /data

# Expose the port the application will run on
EXPOSE 8000

# Set environment variables for Rocket
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

# Set configurable paths (can be overridden at runtime)
ENV ISO2GOD_INPUT_DIR=/data/input
ENV ISO2GOD_OUTPUT_DIR=/data/output

# Set the entrypoint
CMD ["./iso2god-web"]
