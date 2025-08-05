use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Print build information
    println!("📦 Building Stock Scanner with template watching support");
    
    // Tell Cargo to rerun build.rs if these files change
    println!("cargo:rerun-if-changed=templates/");
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    
    // Create necessary directories
    create_directories();
    
    // Create development helper files
    create_dev_helpers();
    
    // Print development information
    print_dev_info();
}

fn create_directories() {
    let dirs = ["templates", "static/css", "static/js", "logs"];
    
    for dir in dirs.iter() {
        if !Path::new(dir).exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("Warning: Failed to create directory {}: {}", dir, e);
            } else {
                println!("✅ Created directory: {}", dir);
            }
        }
    }
}

fn create_dev_helpers() {
    // Create .cargo/config.toml for better development experience
    let cargo_dir = ".cargo";
    if !Path::new(cargo_dir).exists() {
        if let Err(e) = fs::create_dir_all(cargo_dir) {
            eprintln!("Warning: Failed to create .cargo directory: {}", e);
            return;
        }
    }
    
    let config_content = r#"[build]
rustflags = ["-D", "warnings"]

[target.'cfg(target_os = "linux")']
runner = "sudo -E"

[alias]
dev = "run --bin dev"
watch = "watch --exec run"
"#;
    
    let config_path = ".cargo/config.toml";
    if !Path::new(config_path).exists() {
        if let Err(e) = fs::write(config_path, config_content) {
            eprintln!("Warning: Failed to create cargo config: {}", e);
        } else {
            println!("✅ Created .cargo/config.toml");
        }
    }
}

fn print_dev_info() {
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    
    if profile == "debug" {
        println!("🔧 Development Build");
        println!("==================");
        println!("");
        println!("📁 Template watching is enabled!");
        println!("");
        println!("🚀 Quick start commands:");
        println!("  cargo run                    # Build and run");
        println!("  ./dev.sh                     # Development mode with watching");
        println!("  cargo install cargo-watch    # Install cargo-watch");
        println!("  cargo watch -x run           # Auto-restart on changes");
        println!("");
        println!("📂 Watched files and directories:");
        println!("  • templates/*.html           # HTML templates");
        println!("  • src/*.rs                   # Rust source files");
        println!("  • Cargo.toml                 # Dependencies");
        println!("  • static/**/*               # Static assets");
        println!("");
        println!("💡 Development tips:");
        println!("  • Use ./dev.sh for the best development experience");
        println!("  • Template changes will auto-restart the server");
        println!("  • Check logs/ directory for server logs");
        println!("  • Use RUST_LOG=debug for verbose logging");
        println!("");
    }
}

// Helper function to check if a command exists
#[allow(dead_code)]
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// Function to create a system-specific watcher
#[allow(dead_code)]
fn create_system_watcher() {
    #[cfg(target_os = "linux")]
    {
        if command_exists("inotifywait") {
            create_inotify_watcher();
        } else {
            println!("💡 Install inotify-tools for better file watching:");
            println!("   sudo apt-get install inotify-tools");
            println!("   sudo pacman -S inotify-tools");
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if command_exists("fswatch") {
            create_fswatch_watcher();
        } else {
            println!("💡 Install fswatch for better file watching:");
            println!("   brew install fswatch");
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        println!("💡 On Windows, consider using:");
        println!("   cargo install cargo-watch");
        println!("   cargo watch -x run");
    }
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
fn create_inotify_watcher() {
    let script_content = r#"#!/bin/bash
echo "👁️  Starting inotify template watcher..."
echo "Press Ctrl+C to stop"

while true; do
    inotifywait -r -e modify,create,delete,move \
        --include '\.(html|css|js|toml)$' \
        templates/ src/ Cargo.toml 2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "🔄 Files changed, restarting server..."
        pkill -f "target/debug/rust-stock-analyzer" 2>/dev/null
        sleep 1
        cargo run &
    fi
done
"#;
    
    if let Err(e) = fs::write("watch_inotify.sh", script_content) {
        eprintln!("Warning: Failed to create inotify watcher: {}", e);
    } else {
        Command::new("chmod").args(&["+x", "watch_inotify.sh"]).output().ok();
        println!("✅ Created watch_inotify.sh");
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn create_fswatch_watcher() {
    let script_content = r#"#!/bin/bash
echo "👁️  Starting fswatch template watcher..."
echo "Press Ctrl+C to stop"

fswatch -o -r -e "\.git" -e "target" -e "\.swp$" -e "\.tmp$" \
    templates/ src/ Cargo.toml | while read f; do
    echo "🔄 Files changed, restarting server..."
    pkill -f "target/debug/rust-stock-analyzer" 2>/dev/null
    sleep 1
    cargo run &
done
"#;
    
    if let Err(e) = fs::write("watch_fswatch.sh", script_content) {
        eprintln!("Warning: Failed to create fswatch watcher: {}", e);
    } else {
        Command::new("chmod").args(&["+x", "watch_fswatch.sh"]).output().ok();
        println!("✅ Created watch_fswatch.sh");
    }
}