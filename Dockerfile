# Multi-stage build for Rust full-stack app
FROM node:18-alpine AS frontend-builder

# Install Tailwind CSS
WORKDIR /app/frontend
COPY frontend/package.json frontend/tailwind.config.js ./
COPY frontend/src/input.css ./src/
RUN npm install

# Copy Rust source files and HTML for Tailwind content scanning
COPY frontend/src/ ./src/
COPY frontend/index.html ./

# Create dist directory and copy HTML
RUN mkdir -p ./dist
RUN cp index.html ./dist/

# Build Tailwind CSS
RUN npm run build-css

FROM rust:1.75 AS rust-builder

# Install wasm-pack for frontend build
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app
COPY . .

# Copy pre-built CSS and HTML from frontend-builder
COPY --from=frontend-builder /app/frontend/dist/styles.css ./frontend/dist/
COPY --from=frontend-builder /app/frontend/dist/index.html ./frontend/dist/

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
COPY --from=rust-builder /app/target/release/backend /app/backend
COPY --from=rust-builder /app/frontend/dist /app/frontend/dist

# Set environment variables
ENV RUST_LOG=info
ENV REDIS_URL=redis://redis:6379

# Expose port
EXPOSE 3000

# Run the backend server
CMD ["./backend"]