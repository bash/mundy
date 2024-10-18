use bevy::{log, prelude::*};
use bevy_mundy::mundy::AccentColor;
use bevy_mundy::{Pref, PreferencesChanged, PreferencesPlugin};

fn main() -> AppExit {
    App::new()
        .add_plugins((DefaultPlugins, PreferencesPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, (preferences_changed, update_materials))
        .run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    let circle = meshes.add(Circle { radius: 50.0 });
    let material = materials.add(Color::BLACK);
    commands.spawn((Mesh2d(circle), MeshMaterial2d(material), AccentColored));
}

#[derive(Component)]
struct AccentColored;

fn update_materials(
    mut materials: ResMut<Assets<ColorMaterial>>,
    entities: Query<&MeshMaterial2d<ColorMaterial>, With<AccentColored>>,
    accent_color: Pref<AccentColor>,
) {
    for entity in entities.iter() {
        if let Some(material) = materials.get_mut(&entity.0) {
            let color = accent_color.read().0.map(Color::from).unwrap_or_default();
            material.color = color;
        }
    }
}

fn preferences_changed(mut events: EventReader<PreferencesChanged>) {
    for event in events.read() {
        log::info!("{:#?}", &event.0);
    }
}
