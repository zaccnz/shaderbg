<h1 align="center">
    shaderbg
</h1>
<p align="center">
    <img src="repo/demo.gif" alt="A demo of shaderbg, showing the Waves scene"><br>
    waves demo scene <em>(ported from <a href="https://github.com/tengbao/vanta/blob/master/src/vanta.waves.js">Vanta.js</a>)</em>
</p>
<p align="center">
  try it online <a href="https://zaccnz.github.io/shaderbg">here</a><br>
  <em>note that WebGPU is required, and not supported by all browsers.</em>
</p>

lightweight animated backgrounds.  
built to be cross-platform (Windows, macOS, Linux) using Rust.  
  
### note: still in development!

### features
- not a web browser!
- cross platform
  - only macOS at the moment, Windows eta July
- lightweight
- configurable
- portable
- free and open source

### packages
| name               | use                               |
|--------------------|-----------------------------------|
| `shaderbg-render`  | library to load and render scenes |
| `shaderbg`         | shaderbg application              |
| `shaderbg-web`     | web demo build                    |

### dependencies

- **render**
  - [wgpu](https://crates.io/crates/wgpu/) = "0.15.0"
  - [egui](https://crates.io/crates/egui) = "0.22.0"
  - [egui-wgpu](https://crates.io/crates/egui-wgpu) = "0.22.0"
  - [serde](https://crates.io/crates/serde) = "1.0"
  - [toml](https://crates.io/crates/toml) = "0.7.3"
  - [cgmath](https://crates.io/crates/cgmath) = "0.18", [clap](https://crates.io/crates/clap) = "4.2.4, [chrono](https://crates.io/crates/chrono) = "0.4.24", [bytemuck](https://crates.io/crates/bytemuck) = "1.12", [rand](https://crates.io/crates/rand) = "0.8.5", [hex_color](https://crates.io/crates/hex_color) = "2.0.0", [naga](https://crates.io/crates/naga) = "0.11.0", [raw-window-handle](https://crates.io/crates/raw-window-handle) = "0.5", [log](https://crates.io/crates/log) = "0.4
- **desktop**
  - [tao](https://crates.io/crates/tao/) = "0.19.0"
  - [env_logger](https://crates.io/crates/env_logger/) = "0.10.0"
  - [image](https://crates.io/crates/image/) = "0.24.6"
  - [pollster](https://crates.io/crates/pollster/) = "0.3.0"
  - [webbrowser](https://crates.io/crates/webbrowser) = "0.8.10"
  - **macOS only:** [cocoa](https://crates.io/crates/cocoa) = "0.24", [objc](https://crates.io/crates/objc) = "0.2.2"
- **web**
  - [winit](https://crates.io/crates/winit) = "0.28.6"
  - [egui-winit](https://crates.io/crates/egui-winit) = "0.22.0"
  - [wasm-bindgen](https://crates.io/crates/wasm-bindgen) = "0.2.86"
  - [wasm-bindgen-futures](https://crates.io/crates/wasm-bindgen-futures) = "0.4.36"
  - [web-sys](https://crates.io/crates/web-sys) = "0.3.22"
  - [getrandom](https://crates.io/crates/getrandom) = "0.2", [web-time](https://crates.io/crates/web-time) = "0.2.0", [console_log](https://crates.io/crates/console_log) = "1", [console_error_panic_hook](https://crates.io/crates/console_error_panic_hook) = "0.1.7"

### build instructions

to build and run the desktop application
``` sh
cargo run -p shaderbg
```

to build the web demo
```sh
rustup target add wasm32-unknown-unknown
cargo install -f wasm-bindgen-cli
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --target wasm32-unknown-unknown -p shaderbg-web
wasm-bindgen --out-dir generated --web target/wasm32-unknown-unknown/debug/shaderbg-web.wasm
```
then serve the `generated` folder.  
example, using npm's [http-server](https://www.npmjs.com/package/http-server): `http-server -p80 ./generated`  

### platform notes
**macOS**  
the background can not render behind the menu bar.  set a black desktop background for the best experience.

### resources
- [Learn WGPU](https://sotrh.github.io/learn-wgpu/) by Ben Hansen
- [wgpu-life](https://github.com/blakej11/wgpu-life) by Blake Jones
- [wgpu-demo](https://github.com/0xc0dec/wgpu-demo) by Aleksey Fedotov
- [Vanta.js](https://github.com/tengbao/vanta/blob/master/src/vanta.waves.js) by Teng Bao  
- [cefrust](https://github.com/maketechnology/cefrust/blob/6404c4dc0c984b3ca92fff7d42d7599cd432f088/cefrustlib/src/lib.rs#LL154C24-L154C24)'s swizzleSendEvent
- [wgpu-rs on the web](https://gfx-rs.github.io/2020/04/21/wgpu-web.html) by gfx-rs nuts and bolts
- [BlueEngineEGUI](https://github.com/AryanpurTech/BlueEngineEGUI) by Elham Aryanpur
