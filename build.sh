#!/bin/bash

# Build script for Rust Stock Analyzer

echo "🚀 Building Rust Stock Analyzer..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build the project
echo "📦 Building project..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo ""
    echo "🎯 Binary location: target/release/rust-stock-analyzer"
    echo "📋 Usage: ./target/release/rust-stock-analyzer"
    echo ""
    echo "🌐 Web interface will be available at: http://localhost:8080"
    echo "📊 Batch analysis at: http://localhost:8080/batch"
    echo ""
    echo "🔧 Configuration options:"
    echo "   HOST=0.0.0.0 PORT=8080 ./target/release/rust-stock-analyzer"
    echo "   Or use config.json file"
    echo ""
    echo "📈 To use real data, start akshare service:"
    echo "   python3 ../akshare_service.py"
    echo "   Or run: ../setup_akshare_service.sh"
else
    echo "❌ Build failed. Please check the error messages above."
    exit 1
fi