# Multi-stage build for Rust stock analyzer
FROM rust:1.75-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY templates ./templates

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 stockuser

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/rust-stock-analyzer .

# Copy templates
COPY --from=builder /app/templates ./templates

# Create necessary directories
RUN mkdir -p /app/data && \
    chown -R stockuser:stockuser /app

# Switch to non-root user
USER stockuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Set environment variables
ENV RUST_LOG=info \
    HOST=0.0.0.0 \
    PORT=8080 \
    DATABASE_URL=sqlite:/app/data/stock_analyzer.db \
    DATABASE_ENABLE_MIGRATIONS=true

# Run the application
CMD ["./rust-stock-analyzer"]