#!/bin/bash

# Build script for zfs_space_visualizer
# Usage: ./build.sh [target]
# Example: ./build.sh x86_64-unknown-linux-gnu

TARGET=${1:-"x86_64-apple-darwin"}

echo "Building for target: $TARGET"

# Add the target if it's not already installed
rustup target add "$TARGET"

# For cross-compilation to Linux, try different approaches
if [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
    if command -v cross &> /dev/null; then
        echo "Attempting cross-compilation with cross..."
        cross build --release --target "$TARGET"
        BUILD_RESULT=$?
    else
        BUILD_RESULT=1
    fi

    # If cross failed, try standard cargo build
    if [ $BUILD_RESULT -ne 0 ]; then
        echo "Cross-compilation failed, trying standard cargo build..."
        cargo build --release --target "$TARGET"
        BUILD_RESULT=$?
    fi
else
    # Standard cargo build for other targets
    cargo build --release --target "$TARGET"
    BUILD_RESULT=$?
fi

if [ $BUILD_RESULT -eq 0 ]; then
    echo "Build successful!"
    echo "Binary location: target/$TARGET/release/zfs_space_visualizer"

    # Show deployment command for manual execution
    if [[ "$TARGET" == *"linux"* ]] || [ "$TARGET" = "x86_64-apple-darwin" ]; then
        echo "To deploy to server, run:"
        echo "  scp target/$TARGET/release/zfs_space_visualizer cleteserver.home:/home/clete2/"
    fi
else
    echo "Build failed!"
    echo "Note: For cross-compilation to Linux, consider installing 'cross':"
    echo "  cargo install cross"
    exit 1
fi