# Introduction

Deft is a framework for building desktop and mobile applications with Rust and JavaScript.

[![crates.io](https://img.shields.io/crates/v/deft)](https://crates.io/crates/deft)


# Features

* Hybrid programming with Rust and JavaScript
* Non-webview core
* Unified JavaScript engine and rendering engine
* Support React/Vue/Solid or any framework that supports custom render

# Quick Start

```
npm create deft@latest
```

[Documentation](https://deft-ui.github.io/guides/what-is-deft/)

[Demos](https://deft-ui.github.io/demos/)

# Platforms

| Platform | Versions      | Supported |
|----------|---------------|-----------|
| Windows  | 10+           | ✅         |
| Linux    | X11 & Wayland | ✅         |
| MacOS    | -             | ✅         |
| Android  | -             | ✅         |
| iOS      | -             | ❔         |
| Web      | -             | ❌         |

# Building

**On Debian**

```
apt install build-essential libssl-dev libclang-dev libc++-dev xorg-dev libxcb-xfixes0-dev libxcb-shape0-dev libdbus-1-dev libasound2-dev libegl-dev libgles-dev librust-wayland-egl-dev
```

```
cargo build --features x11,wayland
```

**On Windows/MacOS**

Make sure Clang14+ installed.

```
cargo build
```

# License

MIT