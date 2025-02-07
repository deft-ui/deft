#!/bin/bash
set -ue

pushd packages/deft-build
cargo publish
popd

pushd packages/deft-macros
cargo publish
popd

cargo publish --features x11
