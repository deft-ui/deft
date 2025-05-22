#!/bin/bash
set -ue

export LLVM_BIN_PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
export LIBCLANG_PATH=$ANDROID_NDK_HOME/toolchains/llvm//prebuilt/linux-x86_64/lib
export NDK_SYSROOT=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/sysroot

export CLANG_PATH=$LLVM_BIN_PATH/clang++
export CXXSTDLIB_AARCH64_LINUX_ANDROID=c++
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
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$LLVM_BIN_PATH/clang
export CARGO_ENCODED_RUSTFLAGS="-Clink-args=--target=aarch64-linux-android23 --sysroot=$NDK_SYSROOT"

cargo build --target aarch64-linux-android --release --example hello