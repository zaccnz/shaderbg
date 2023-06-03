# 🖼️ shaderbg

<p style="text-align: center;">
    <img src="repo/demo.gif" alt="A demo of shaderbg, showing the Waves scene">
    <em>waves demo scene</em> (ported from <a href="https://github.com/tengbao/vanta/blob/master/src/vanta.waves.js">Vanta.js</a>)
</p>

lightweight animated backgrounds.  
built to be cross-platform (Windows, macOS, Linux) using Rust.  
<!-- check it out at [zaccnz.github.io/shaderbg](https://zaccnz.github.io/shaderbg) -->

### note: still in development!

### features
- not a web browser!
- cross platform
  - only macOS at the moment, Windows eta July
- lightweight
- portable
- free and open source

### dependencies

- [tao](https://crates.io/crates/tao/) = "0.19.0"
- [wgpu](https://crates.io/crates/wgpu/) = "0.15.0"
- [imgui](https://crates.io/crates/imgui/) = "0.10"
- [imgui-wgpu](https://crates.io/crates/imgui-wgpu/) = "0.22.0"
- [cgmath](https://crates.io/crates/cgmath) = "0.18"
- [env_logger](https://crates.io/crates/env_logger/) = "0.10.0", [image](https://crates.io/crates/image/) = "0.24.6", [pollster](https://crates.io/crates/pollster/) = "0.3.0", [clap](https://crates.io/crates/clap) = "4.2.4, [chrono](https://crates.io/crates/chrono) = "0.4.24", [serde](https://crates.io/crates/serde) = "1.0", [toml](https://crates.io/crates/toml) = "0.7.3", [bytemuck](https://crates.io/crates/bytemuck) = "1.12", [rand](https://crates.io/crates/rand) = "0.8.5", [hex_color](https://crates.io/crates/hex_color) = "2.0.0", [naga](https://crates.io/crates/naga) = "0.11.0"
- **macOS dependencies** [cocoa](https://crates.io/crates/cocoa) = "0.24", [objc](https://crates.io/crates/objc) = "0.2.2"

### build instructions

``` sh
cargo run
```

### resources

- [Learn WGPU](https://sotrh.github.io/learn-wgpu/) by Ben Hansen
- [wgpu-life](https://github.com/blakej11/wgpu-life) by Blake Jones
- [wgpu-demo](https://github.com/0xc0dec/wgpu-demo) by Aleksey Fedotov
- [Vanta.js](https://github.com/tengbao/vanta/blob/master/src/vanta.waves.js) by Teng Bao  
- [cefrust](https://github.com/maketechnology/cefrust/blob/6404c4dc0c984b3ca92fff7d42d7599cd432f088/cefrustlib/src/lib.rs#LL154C24-L154C24)'s swizzleSendEvent
