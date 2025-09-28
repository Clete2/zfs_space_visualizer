#!/bin/bash

echo "##### Building local (dev) #####"
cargo build

echo "##### Building local (release) #####"
cargo build --release # Build dev for current arch

targets="aarch64-unknown-linux-musl x86_64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin"
# glibc issue: https://github.com/cross-rs/cross/issues/724#issuecomment-1638593579
for target in $targets; do
  echo "##### Building $target (release) #####"
  cross build --release --target $target || exit 1
done