#!/bin/bash
set -e

echo "Building frontend..."
cd frontend
wasm-pack build --target web --out-dir dist --out-name frontend
cd ..

# echo "Building backend..."
# cargo build --bin backend --release

# echo "Build complete!"
# echo "Run 'cargo run --bin backend' to start the server"