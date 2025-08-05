# 🚀 Development Setup with Template Watching

This project now includes build.rs support for template file watching and automatic server restart.

## 📋 Quick Start

### Basic Commands
```bash
# Standard build and run
cargo run

# Development mode with file watching (recommended)
./dev.sh

# Manual file watching
cargo install cargo-watch
cargo watch -x run

# Build only
cargo build
```

## 🔧 Template Watching

The build.rs script automatically monitors:
- `templates/*.html` - HTML template files
- `src/*.rs` - Rust source files  
- `Cargo.toml` - Dependencies
- `static/**/*` - Static assets

## 📁 Auto-created Directories

On first build, these directories are created:
- `templates/` - HTML templates
- `static/css/` - CSS files
- `static/js/` - JavaScript files
- `logs/` - Log files

## ⚙️ Development Configuration

The `.cargo/config.toml` provides:
- Optimized build settings
- Development aliases
- Relaxed warning levels for development

## 💡 Tips

1. **Use `./dev.sh`** for the best development experience
2. **Template changes** auto-restart the server
3. **Clear browser cache** (Ctrl+F5) after template changes
4. **Check logs** in the `logs/` directory
5. **Use debug logging** with `RUST_LOG=debug cargo run`

## 🐛 Troubleshooting

If you encounter the "alias watch" warning:
- The conflicting alias has been removed
- Use `cargo watch -x run` directly
- Or use `./dev.sh` for the best experience

## 🔍 File Monitoring

The system uses cargo-watch for efficient file monitoring:
- Ignores `target/` and temporary files
- Monitors relevant source files
- Fast restart on changes
- Minimal resource usage