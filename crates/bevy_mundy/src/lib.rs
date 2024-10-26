use async_channel::{unbounded, Receiver, Sender};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt as _;
use bevy::tasks::{IoTaskPool, Task};
use bevy::utils::synccell::SyncCell;
use bevy::winit::{EventLoopProxy, EventLoopProxyWrapper, WakeUp, WinitPlugin};
use mundy::{AccentColor, Interest, Preferences, PreferencesStream};
use std::marker::PhantomData;

pub use mundy;

#[derive(Debug)]
#[non_exhaustive]
pub struct PreferencesPlugin<E: 'static = WakeUp> {
    /// Interests used when subscribing to the preferences.
    /// They indicate which preferences should be retrieved and monitored.
    /// *Default:* [`Interest::All`]
    pub interest: Interest,
    /// The event sent to the [`EventLoopProxy`] in order to wake up
    /// the window loop when running in reactive mode.
    /// *Default:* `|| WakeUp`
    pub wakeup: fn() -> E,
}

impl Default for PreferencesPlugin {
    fn default() -> Self {
        PreferencesPlugin {
            interest: Interest::All,
            wakeup: || WakeUp,
        }
    }
}

impl<E: 'static + Send> Plugin for PreferencesPlugin<E> {
    fn build(&self, app: &mut App) {
        // If we create our stream *before* winit is initialized,
        // we'll get a panic on macOS: https://github.com/rust-windowing/winit/issues/3772
        if !app.is_plugin_added::<WinitPlugin>() {
            panic!("WinitPlugin needs to be added before PreferencesPlugin")
        }

        let stream = Preferences::stream(self.interest);
        app.insert_resource(PreferencesStreamRes(SyncCell::new(Some(stream))));
        app.init_resource::<PreferencesRes>();
        app.insert_resource(WakeupEvent(self.wakeup));
        app.add_event::<PreferencesChanged>();
        app.add_systems(PreStartup, spawn_task::<E>);
        app.add_systems(
            PreUpdate,
            (poll_receiver, update_preferences_resource).chain(),
        );
    }
}

impl<E: 'static> PreferencesPlugin<E> {
    pub fn with_custom_event<F>(self, wakeup: fn() -> F) -> PreferencesPlugin<F> {
        PreferencesPlugin {
            interest: self.interest,
            wakeup,
        }
    }

    pub fn with_interest(mut self, interest: Interest) -> Self {
        self.interest = interest;
        self
    }
}

fn spawn_task<E: 'static + Send>(
    mut commands: Commands,
    mut stream: ResMut<PreferencesStreamRes>,
    event_loop_proxy: Res<EventLoopProxyWrapper<E>>,
    wakeup: Res<WakeupEvent<E>>,
) {
    let stream = (stream.0)
        .get()
        .take()
        .expect("plugin ensures that pref stream exists");
    let (sender, receiver) = unbounded();
    let task = forward_preferences(stream, sender, event_loop_proxy.clone(), wakeup.0);
    commands.insert_resource(PreferencesSubscription { receiver, task });
    commands.remove_resource::<PreferencesStreamRes>();
    commands.remove_resource::<WakeupEvent<E>>();
}

fn poll_receiver(
    mut writer: EventWriter<PreferencesChanged>,
    subscription: ResMut<PreferencesSubscription>,
) {
    if let Ok(preferences) = subscription.receiver.try_recv() {
        writer.send(PreferencesChanged(preferences));
    }
}

fn update_preferences_resource(
    mut res: ResMut<PreferencesRes>,
    mut events: EventReader<PreferencesChanged>,
) {
    if let Some(event) = events.read().last() {
        res.0 = event.0;
    }
}

fn forward_preferences<E: 'static + Send>(
    mut stream: PreferencesStream,
    sender: Sender<Preferences>,
    event_loop_proxy: EventLoopProxy<E>,
    wakeup_event: fn() -> E,
) -> Task<()> {
    IoTaskPool::get().spawn(async move {
        while let Some(preferences) = stream.next().await {
            _ = sender.send(preferences).await;
            _ = event_loop_proxy.send_event(wakeup_event());
        }
    })
}

#[derive(Resource, Default, Debug, Clone)]
pub struct PreferencesRes(pub Preferences);

#[derive(Event, Debug, Clone)]
pub struct PreferencesChanged(pub Preferences);

#[derive(Resource)]
struct PreferencesSubscription {
    #[expect(dead_code)]
    task: Task<()>,
    receiver: Receiver<Preferences>,
}

#[derive(Resource)]
struct PreferencesStreamRes(SyncCell<Option<PreferencesStream>>);

#[derive(SystemParam)]
#[allow(private_bounds)]
pub struct Pref<'w, P: Preference + 'static> {
    preferences: Res<'w, PreferencesRes>,
    marker: PhantomData<P>,
}

#[allow(private_bounds)]
impl<P: Preference + 'static> Pref<'_, P> {
    pub fn read(&self) -> P {
        P::from_preferences(&self.preferences.0)
    }
}

trait Preference {
    fn from_preferences(preferences: &Preferences) -> Self;
}

impl Preference for AccentColor {
    fn from_preferences(preferences: &Preferences) -> Self {
        preferences.accent_color
    }
}

#[derive(Resource)]
struct WakeupEvent<E>(fn() -> E);
