use bevy::prelude::*;
use road::add_road_update_systems;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

mod joyride;
mod road;
mod util;

fn main() {
    let mut app_builder = App::build();

    let mut ingame_set = SystemSet::new();
    ingame_set = add_road_update_systems(ingame_set);

    app_builder
        .insert_resource(WindowDescriptor {
            title: "Joyride".to_string(),
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            vsync: false,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(joyride::startup_joyride.system())
        .add_startup_system(road::startup_road.system())
        .add_system_set(ingame_set);

    #[cfg(target_arch = "wasm32")]
    app_builder.add_plugin(bevy_webgl2::WebGL2Plugin);

    app_builder.run();
}
