#!/bin/bash
set -e

echo "Installing frontend dependencies..."
cd frontend
npm install

echo "Building Tailwind CSS..."
npm run build-css

echo "Copying HTML template..."
cp index.html dist/

echo "Building frontend WebAssembly..."
wasm-pack build --target web --out-dir dist --out-name frontend --no-opt #keep args

# echo "Building backend..."
# cd ..
# cargo build --bin backend --release

# echo "Build complete!"
# echo "Run 'cargo run --bin backend' to start the server"