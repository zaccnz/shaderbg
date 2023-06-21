/*
 * App entrypoint
 */
use clap::Parser;
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod app;
pub mod egui_tao;
mod io;
mod ui;

fn main() {
    env_logger::init();

    //let scene_path = "scenes/shadertoy-ltcGDl";
    //let scene_path = "scenes/shadertoy-mdBSRt";
    //let scene_path = "scenes/waves";

    let args = io::Args::parse();

    let config = match io::Config::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("failed to load config file.");
            eprintln!("error: {:?}", e);
            io::Config::default()
        }
    };

    // check if shaderbg is already running
    // it is? -> tell 'window' to open and quit this process
    // https://gist.github.com/andelf/8668088 could be used for IPC

    let (win_thread, event_loop) = app::WindowThread::build();

    let (app_state, handle) = app::start_main(args, config, event_loop.create_proxy());

    win_thread.run(event_loop, app_state, handle);
}
