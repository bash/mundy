use log::info;
use mundy::{Interest, Preferences};
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event_loop::{EventLoop, EventLoopBuilder};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

pub fn main(configure: impl for<'a> FnOnce(&'a mut EventLoopBuilder<()>)) {
    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,winit_example=trace,mundy=trace"),
    )
    .init();
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter(None, log::LevelFilter::Warn)
                    .filter_module("mundy", log::LevelFilter::Trace)
                    .filter_module("winit_example", log::LevelFilter::Trace)
                    .build(),
            )
            .with_max_level(log::LevelFilter::Trace),
    );

    let once = Preferences::once_blocking(Interest::All, Duration::from_millis(200));
    info!("preferences from once_blocking: {once:#?}");

    let mut builder = EventLoop::builder();
    configure(&mut builder);

    // This hides the app icon from the Dock.
    #[cfg(target_os = "macos")]
    builder.with_activation_policy(ActivationPolicy::Prohibited);

    let event_loop = builder.build().unwrap();
    let _subscription = Preferences::subscribe(Interest::All, |preferences| {
        info!("preferences: {preferences:#?}");
    });
    event_loop.run_app(&mut App).unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid as _;
    main(move |b| {
        b.with_android_app(app);
    });
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
        // Sadly we can't get direct access to the underlying
        // `ConfigChangedEvent` on Android (this is an open issue¹ in winit),
        // but luckily winit emits a `ScaleFactorChanged` event (I think this is a bug).
        // ¹: https://github.com/rust-windowing/winit/issues/2120
        #[cfg(target_os = "android")]
        if let winit::event::WindowEvent::ScaleFactorChanged { .. } = _event {
            mundy::platform::android::on_configuration_changed();
        }
    }
}
