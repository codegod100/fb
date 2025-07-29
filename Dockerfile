# Multi-stage build for Rust full-stack app
FROM rust:1.75 as builder

# Install wasm-pack for frontend build
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app
COPY . .

# Build frontend (WebAssembly)
RUN wasm-pack build frontend --target web --out-dir dist --out-name frontend

# Build backend (release mode)
RUN cargo build --release -p backend

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy built artifacts
COPY --from=builder /app/target/release/backend /app/backend
COPY --from=builder /app/frontend/dist /app/frontend/dist

# Set environment variables
ENV RUST_LOG=info
ENV REDIS_URL=redis://redis:6379

# Expose port
EXPOSE 3000

# Run the backend server
CMD ["./backend"]