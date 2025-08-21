use std::time::Duration;

use mundy::{Interest, Preferences};
use winit::application::ApplicationHandler;
use winit::event_loop::EventLoop;
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

fn main() {
    env_logger::init();

    let once = Preferences::once_blocking(Interest::All, Duration::from_millis(200));
    eprintln!("preferences from once_blocking: {once:#?}");

    let mut builder = EventLoop::builder();

    // This hides the app icon from the Dock.
    #[cfg(target_os = "macos")]
    builder.with_activation_policy(ActivationPolicy::Prohibited);

    let event_loop = builder.build().unwrap();
    let _subscription = Preferences::subscribe(Interest::All, |preferences| {
        dbg!(preferences);
    });
    event_loop.run_app(&mut App).unwrap();
}

#[derive(Default)]
struct App;

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }
}
