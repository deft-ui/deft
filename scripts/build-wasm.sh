#!/bin/bash
source $EMSDK/emsdk_env.sh
clang -v
export EMCC_CFLAGS="-s MAX_WEBGL_VERSION=2 -s MODULARIZE=1 -s EXPORT_NAME=loadDeftApp -s EXPORTED_RUNTIME_METHODS=GL,cwrap"
cargo build --target wasm32-unknown-emscripten --no-default-features --example gallery $@
