#!/bin/bash

cross build --target x86_64-unknown-linux-musl || exit 1
scp target/x86_64-unknown-linux-musl/debug/zfs_space_visualizer cleteserver.home:/home/clete2/zfs_space_visualizer