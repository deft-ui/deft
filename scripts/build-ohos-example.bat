@echo off
set LLVM_BIN_PATH=%OHOS_SDK_HOME%/native/llvm/bin
set LIBCLANG_PATH=%OHOS_SDK_HOME%/native/llvm/lib
set PATH=%LLVM_BIN_PATH%;%PATH%

set CLANG_PATH=%LLVM_BIN_PATH%/clang++.exe
set CXXSTDLIB_X86_64_UNKNOWN_LINUX_OHOS=c++

set TARGET_CC=%LLVM_BIN_PATH%/clang.exe
set TARGET_CXX=%LLVM_BIN_PATH%/clang++.exe
set TARGET_AR=%LLVM_BIN_PATH%/llvm-ar.exe
set TARGET_OBJDUMP=%LLVM_BIN_PATH%/llvm-objdump.exe
set TARGET_OBJCOPY=%LLVM_BIN_PATH%/llvm-objcopy.exe
set TARGET_NM=%LLVM_BIN_PATH%/llvm-nm.exe
set TARGET_AS=%LLVM_BIN_PATH%/llvm-as.exe
set TARGET_LD=%LLVM_BIN_PATH%/ld.lld.exe
set TARGET_RANLIB=%LLVM_BIN_PATH%/llvm-ranlib.exe
set TARGET_STRIP=%LLVM_BIN_PATH%/llvm-strip.exe

set CARGO_TARGET_X86_64_UNKNOWN_LINUX_OHOS_LINKER=%TARGET_CC%
set CARGO_ENCODED_RUSTFLAGS=-Clink-args=--target=x86_64-linux-ohos --sysroot=%OHOS_SDK_HOME%/native/sysroot -D__MUSL__

cargo build --target x86_64-unknown-linux-ohos --release --example mobile_demo
