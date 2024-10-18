use async_channel::{unbounded, Receiver, Sender};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt as _;
use bevy::tasks::{IoTaskPool, Task};
use bevy::winit::WinitPlugin;
use mundy::{AccentColor, Interest, Preferences, PreferencesStream};
use std::marker::PhantomData;

pub use mundy;

#[derive(Debug)]
#[non_exhaustive]
pub struct PreferencesPlugin {
    pub interest: Interest,
}

impl Default for PreferencesPlugin {
    fn default() -> Self {
        PreferencesPlugin {
            interest: Interest::All,
        }
    }
}

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        // If we create our stream *before* winit is initialized,
        // we'll get a panic on macOS: https://github.com/rust-windowing/winit/issues/3772
        if !app.is_plugin_added::<WinitPlugin>() {
            panic!("WinitPlugin needs to be added before PreferencesPlugin")
        }

        let stream = Preferences::stream(self.interest);
        let (sender, receiver) = unbounded();
        let task = forward_preferences(stream, sender);
        app.insert_resource(PreferencesSubscription { receiver, task });
        app.insert_resource(PreferencesRes::default());
        app.add_event::<PreferencesChanged>();
        app.add_systems(
            PreUpdate,
            (poll_receiver, update_preferences_resource).chain(),
        );
    }
}

fn poll_receiver(mut commands: Commands, subscription: ResMut<PreferencesSubscription>) {
    if let Ok(preferences) = subscription.receiver.try_recv() {
        commands.send_event(PreferencesChanged(preferences));
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

fn forward_preferences(mut stream: PreferencesStream, sender: Sender<Preferences>) -> Task<()> {
    IoTaskPool::get().spawn(async move {
        while let Some(preferences) = stream.next().await {
            _ = sender.send(preferences).await;
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

// impl FromWorld
