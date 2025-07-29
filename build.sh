#!/bin/bash
set -e

echo "Installing frontend dependencies..."
cd frontend
npm install

echo "Building Tailwind CSS..."
npm run build-css

echo "Building frontend WebAssembly..."
wasm-pack build --target web --out-dir dist --out-name frontend --no-opt

# echo "Building backend..."
# cd ..
# cargo build --bin backend --release

# echo "Build complete!"
# echo "Run 'cargo run --bin backend' to start the server"