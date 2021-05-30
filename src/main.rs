use bevy::core::FixedTimestep;
use bevy::prelude::*;
use easy_cast::*;
use road::add_road_render_systems;
use road::add_road_update_systems;
use skybox::add_skybox_update_systems;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

mod joyride;
mod road;
mod skybox;
mod util;

fn main() {
    let mut app_builder = App::build();

    let mut ingame_update_set =
        SystemSet::new().with_run_criteria(FixedTimestep::step(joyride::TIME_STEP.cast()));
    ingame_update_set = add_road_update_systems(ingame_update_set);
    ingame_update_set = add_skybox_update_systems(ingame_update_set);

    // We add road rendering to a non-fixed timestep. If we use a fixed timestep, the updated road
    // texture is sometimes used one (non-fixed) frame too late, leaving a gap of black pixels.
    // Not quite sure why
    let mut ingame_render_set = SystemSet::new();
    ingame_render_set = add_road_render_systems(ingame_render_set);

    app_builder
        .insert_resource(WindowDescriptor {
            title: "Joyride".to_string(),
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            vsync: true,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(joyride::startup_joyride.system())
        .add_startup_system(skybox::startup_skybox.system())
        .add_startup_system(road::startup_road.system())
        .add_system_set(ingame_update_set)
        .add_system_set_to_stage(CoreStage::PostUpdate, ingame_render_set);

    #[cfg(target_arch = "wasm32")]
    app_builder.add_plugin(bevy_webgl2::WebGL2Plugin);

    app_builder.run();
}
