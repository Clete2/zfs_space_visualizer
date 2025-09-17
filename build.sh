#!/bin/bash

# Build script for zfs_space_visualizer
# Usage: ./build.sh [target]
# Example: ./build.sh x86_64-unknown-linux-gnu

TARGET=${1:-"x86_64-unknown-linux-gnu"}

echo "Building for target: $TARGET"

# Add the target if it's not already installed
rustup target add "$TARGET"

# For cross-compilation to Linux, try using cross if available
if [ "$TARGET" = "x86_64-unknown-linux-gnu" ] && command -v cross &> /dev/null; then
    echo "Using cross for cross-compilation..."
    cross build --release --target "$TARGET"
else
    # Standard cargo build
    cargo build --release --target "$TARGET"
fi

if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo "Binary location: target/$TARGET/release/zfs_space_visualizer"

    # If this is the x86_64 GNU Linux target, deploy to server
    if [ "$TARGET" = "x86_64-unknown-linux-gnu" ]; then
        echo "Deploying to cleteserver.home..."
        scp "target/$TARGET/release/zfs_space_visualizer" cleteserver.home:/home/clete2/
        if [ $? -eq 0 ]; then
            echo "Deployment successful!"
        else
            echo "Deployment failed!"
            exit 1
        fi
    fi
else
    echo "Build failed!"
    echo "Note: For cross-compilation to Linux, consider installing 'cross':"
    echo "  cargo install cross"
    exit 1
fi