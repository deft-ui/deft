# Introduction

Deft is a framework for building desktop and mobile applications with Rust and JavaScript.

# Features

* Hybrid programming with Rust and JavaScript
* Non-Webview core
* Unified JavaScript engine and rendering engine
* Similar api to web

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
apt install build-essential libssl-dev libclang-dev libc++-dev
apt install xorg-dev libxcb-xfixes0-dev libxcb-shape0-dev
apt install libasound2-dev
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