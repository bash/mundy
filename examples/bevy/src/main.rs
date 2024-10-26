use bevy::prelude::*;
use bevy::winit::WinitSettings;
use bevy_mundy::mundy::Interest;
use bevy_mundy::{PreferencesChanged, PreferencesPlugin};

fn main() -> AppExit {
    App::new()
        .insert_resource(WinitSettings::desktop_app())
        .add_plugins(DefaultPlugins)
        .add_plugins(PreferencesPlugin::default().with_interest(Interest::AccentColor))
        .add_systems(Startup, setup)
        .add_systems(Update, update_materials)
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
    mut reader: EventReader<PreferencesChanged>,
) {
    if let Some(preferences) = reader.read().last() {
        let accent_color = preferences.0.accent_color;

        for entity in entities.iter() {
            if let Some(material) = materials.get_mut(&entity.0) {
                let color = accent_color.0.map(Color::from).unwrap_or_default();
                material.color = color;
            }
        }
    }
}
