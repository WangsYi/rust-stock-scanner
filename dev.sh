#!/bin/bash

# Development script with template watching
echo "🚀 Starting Stock Scanner Development Server"
echo "=========================================="

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    echo "⚠️  cargo-watch not found. Installing..."
    cargo install cargo-watch
fi

echo "📁 Watching for changes in:"
echo "   • templates/ (HTML files)"
echo "   • src/ (Rust source files)"
echo "   • Cargo.toml (Dependencies)"
echo ""
echo "🔧 Server will restart automatically when files change"
echo "Press Ctrl+C to stop"
echo ""

# Start watching and running
cargo watch -x run