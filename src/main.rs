/*
 * App entrypoint
 */
mod app;
pub mod ext;

fn main() {
    env_logger::init();

    // TODO: load config
    // TODO: load arguments

    // build window event loop
    let (win_thread, event_loop) = app::WindowThread::build();

    // start 'app' process
    let app_state = app::start_main(event_loop.create_proxy());

    // start window event loop
    win_thread.run(event_loop, app_state);
}
