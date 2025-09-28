use std::env;

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    // Also set CARGO_PKG_VERSION as an environment variable during build
    // This ensures the version is available at runtime
    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string()));
}