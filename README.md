# Introduction

Deft is a framework for building desktop and mobile applications with Rust and JavaScript.

[![crates.io](https://img.shields.io/crates/v/deft)](https://crates.io/crates/deft)


# Features

* Hybrid programming with Rust and JavaScript
* Non-webview core
* Unified JavaScript engine and rendering engine
* Themes support
* Support React/Vue/Solid or any framework that supports custom render

# Limits

* Not all CSS properties are supported, see [documentation](https://deft-ui.github.io/styles/properties/) for more details.
* Accessibility is not yet available.
* JavaScript debugger is not yet available.

# Component Gallery

[Live Demo(WASM)](https://deft-ui.github.io/gallery/)

<img width="360" src="https://github.com/deft-ui/deft/blob/main/snapshots/gallery.png?raw=true" />
<img width="360" src="https://github.com/deft-ui/deft/blob/main/snapshots/gallery-dark.png?raw=true" />


# Quick Start

```
npm create deft@latest hello-deft
cd hello-deft
npm install
npm run dev
```

[Documentation](https://deft-ui.github.io/guides/what-is-deft/)

[Demos](https://deft-ui.github.io/demos/)

# Platforms

| Platform    | Versions      | Supported      |
|-------------|---------------|----------------|
| Windows     | 10+           | ✅              |
| Linux       | X11 & Wayland | ✅              |
| MacOS       | 10.12+        | ✅              |
| HarmonyOS   | 5+            | ✅experimental  |
| Android     | 6+            | ✅experimental  |
| iOS         | -             | ✅experimental  |
| WebAssembly | -             | ✅experimental  |

# Develop

### Prerequisites

Make sure `Rust`, `Node.js` and `Clang14+` installed.

Some extra packages need to be installed on Linux.

```
apt install build-essential libssl-dev libclang-dev libc++-dev \
    xorg-dev libxcb-xfixes0-dev libxcb-shape0-dev libdbus-1-dev \
    libasound2-dev libegl-dev libgles-dev librust-wayland-egl-dev
```

### Run demo

Native:
```
cd examples/gallery
cargo run
```

Wasm:
> Make sure that [Emscripten SDK](https://emscripten.org/docs/getting_started/downloads.html) is installed.
```bash
cd examples/gallery
cargo build --target wasm32-unknown-emscripten --release
python3 -m http.server
```
Visit `http://localhost:8000/web/` to preview.

# License

MIT