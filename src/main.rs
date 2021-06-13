use bevy::core::FixedTimestep;
use bevy::prelude::*;
use bevy::render::RenderSystem;
use easy_cast::*;
use player::add_player_update_systems;
use racer::add_racer_update_systems;
use rival::add_rival_update_systems;
use road::add_road_render_systems;
use road::add_road_update_systems;
use skybox::add_skybox_update_systems;
use text::add_text_update_systems;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

use crate::joyride::TIME_STEP;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 960.0;

mod fixed_framerate;
mod joyride;
mod player;
mod racer;
mod rival;
mod road;
mod skybox;
mod text;
mod util;

fn main() {
    let mut app_builder = App::build();

    // TODO: Refactor all this stuff to work like in joyride.rs
    let mut ingame_update_set = SystemSet::new(); //.with_run_criteria(FixedTimestep::step(joyride::TIME_STEP.cast()));
    ingame_update_set = add_road_update_systems(ingame_update_set);
    ingame_update_set = add_skybox_update_systems(ingame_update_set);
    ingame_update_set = add_player_update_systems(ingame_update_set);
    ingame_update_set = add_text_update_systems(ingame_update_set);
    ingame_update_set = add_racer_update_systems(ingame_update_set);
    ingame_update_set = add_rival_update_systems(ingame_update_set);

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
        .add_startup_stage_before(
            StartupStage::Startup,
            "racer startup",
            SystemStage::parallel().with_system(racer::startup_racer.system()),
        )
        .add_startup_system(skybox::startup_skybox.system())
        .add_startup_system(road::startup_road.system())
        .add_startup_system(player::startup_player.system())
        .add_startup_system(rival::startup_rival.system())
        .add_startup_system(text::startup_text.system())
        .add_system_to_stage(
            CoreStage::PostUpdate,
            util::propagate_visibility_system
                .system()
                .before(RenderSystem::VisibleEntities),
        )
        .add_system_set(ingame_update_set)
        .add_system_set_to_stage(CoreStage::PostUpdate, ingame_render_set);

    joyride::build_app(&mut app_builder);

    #[cfg(target_arch = "wasm32")]
    app_builder.add_plugin(bevy_webgl2::WebGL2Plugin);

    app_builder.app.schedule.set_run_criteria(
        fixed_framerate::create_fixed_framerate_run_criteria(TIME_STEP.cast()).system(),
    );

    app_builder.run();
}
