#!/bin/bash
cargo build --target wasm32-unknown-emscripten --no-default-features $@
