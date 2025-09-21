#!/bin/bash

# Build and upload, then SSH to run the program
echo "Building for Linux..."
cross build --target x86_64-unknown-linux-musl || exit 1

echo "Uploading to server..."
scp target/x86_64-unknown-linux-musl/debug/zfs_space_visualizer cleteserver.home:/home/clete2/zfs_space_visualizer || exit 1

echo "Connecting to server and running program..."
echo "Press Ctrl+C to quit the program and close SSH connection"
echo "----------------------------------------"

# SSH with proper signal forwarding so Ctrl+C works correctly
ssh -t cleteserver.home './zfs_space_visualizer'