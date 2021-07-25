use bevy::prelude::*;
use bevy::render::RenderSystem;
use debug::LoopSectionTimer;
use easy_cast::*;
use fixed_framerate::FixedFramerate;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

use crate::joyride::TIME_STEP;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 960.0;

mod debug;
mod fixed_framerate;
mod game;
mod joyride;
mod player;
mod racer;
mod rival;
mod road;
mod road_object;
mod skybox;
mod text;
mod util;

fn main() {
    let mut app_builder = App::build();

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
        .insert_resource(LoopSectionTimer::new())
        .add_plugins(DefaultPlugins)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            util::propagate_visibility_system
                .system()
                .before(RenderSystem::VisibleEntities),
        );

    #[cfg(target_arch = "wasm32")]
    app_builder.add_plugin(bevy_webgl2::WebGL2Plugin);

    app_builder.app.schedule.set_run_criteria(
        fixed_framerate::create_fixed_framerate_run_criteria(FixedFramerate {
            fixed_step: TIME_STEP.cast(),

            // We don't need to bother trying to catch up if we fall behind
            drop_time_after_max_runs: true,

            // If we don't cap at one run for the top-level scheduler, event readers that are
            // part of the app runner will sometimes fail to receive events (notably,
            // the AppExit event reader of the Winit runner)
            max_runs_per_step: Some(1),
        })
        .system(),
    );

    game::setup_game(&mut app_builder);

    app_builder.run();
}
