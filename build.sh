#!/bin/bash
set -ue
cargo build --features x11,wayland
cargo ndk -t arm64-v8a -p 30  build --features x11
echo Done!!