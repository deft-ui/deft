#!/usr/bin/env bash

export LLVM_BIN_PATH=$OHOS_SDK_HOME/native/llvm/bin
export LIBCLANG_PATH=$OHOS_SDK_HOME/native/llvm/lib
export PATH=$LLVM_BIN_PATH:$PATH

export CLANG_PATH=$LLVM_BIN_PATH/clang++
export CXXSTDLIB_AARCH64_UNKNOWN_LINUX_OHOS=c++

export TARGET_CC=$LLVM_BIN_PATH/clang
export TARGET_CXX=$LLVM_BIN_PATH/clang++
export TARGET_AR=$LLVM_BIN_PATH/llvm-ar
export TARGET_OBJDUMP=$LLVM_BIN_PATH/llvm-objdump
export TARGET_OBJCOPY=$LLVM_BIN_PATH/llvm-objcopy
export TARGET_NM=$LLVM_BIN_PATH/llvm-nm
export TARGET_AS=$LLVM_BIN_PATH/llvm-as
export TARGET_LD=$LLVM_BIN_PATH/ld.lld
export TARGET_RANLIB=$LLVM_BIN_PATH/llvm-ranlib
export TARGET_STRIP=$LLVM_BIN_PATH/llvm-strip

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER=$TARGET_CC
export CARGO_ENCODED_RUSTFLAGS="-Clink-args=--target=aarch64-linux-ohos --sysroot=$OHOS_SDK_HOME/native/sysroot -D__MUSL__"

cargo build --target aarch64-unknown-linux-ohos --release --example mobile_demo
