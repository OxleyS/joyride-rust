use bevy::input::InputSystem;
use bevy::prelude::*;
use bevy::render::RenderSystem;
use easy_cast::*;
use fixed_framerate::FixedFramerate;

#[cfg(target_arch = "wasm32")]
use bevy_webgl2;

use crate::joyride::TIME_STEP;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 960.0;

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum InputStageLabels {
    UpdateInput,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerStageLabels {
    UpdatePlayerDriving,
    UpdatePlayerRoadPosition,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum RoadStageLabels {
    UpdateRoadTables,
}

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

    let joyride_systems = joyride::Systems::new();
    let player_systems = player::Systems::new();
    let road_systems = road::Systems::new();
    let skybox_systems = skybox::Systems::new();
    let text_systems = text::Systems::new();
    let rival_systems = rival::Systems::new();
    let racer_systems = racer::Systems::new();

    app_builder.add_startup_stage_before(
        StartupStage::Startup,
        "racer startup",
        SystemStage::parallel().with_system(racer_systems.startup_racer),
    );
    app_builder.add_startup_system(joyride_systems.startup_joyride);
    app_builder.add_startup_system(player_systems.startup_player);
    app_builder.add_startup_system(road_systems.startup_road);
    app_builder.add_startup_system(skybox_systems.startup_skybox);
    app_builder.add_startup_system(text_systems.startup_text);
    app_builder.add_startup_system(rival_systems.startup_rivals);

    app_builder.add_system_set_to_stage(
        CoreStage::PreUpdate,
        joyride_systems
            .update_input
            .label(InputStageLabels::UpdateInput)
            .after(InputSystem),
    );

    app_builder.add_system_set(
        player_systems
            .update_player_driving
            .label(PlayerStageLabels::UpdatePlayerDriving),
    );
    app_builder.add_system_set(
        player_systems
            .update_player_road_position
            .label(PlayerStageLabels::UpdatePlayerRoadPosition)
            .after(PlayerStageLabels::UpdatePlayerDriving),
    );
    app_builder.add_system_set(
        player_systems
            .update_player_visuals
            .after(PlayerStageLabels::UpdatePlayerRoadPosition),
    );
    app_builder.add_system_set(
        road_systems
            .update_road
            .after(PlayerStageLabels::UpdatePlayerRoadPosition)
            .label(RoadStageLabels::UpdateRoadTables),
    );
    app_builder.add_system_set(
        road_systems
            .draw_road
            .after(RoadStageLabels::UpdateRoadTables),
    );
    app_builder.add_system_set(road_systems.test_curve_road);
    app_builder.add_system_set(
        skybox_systems
            .update_skybox
            .after(RoadStageLabels::UpdateRoadTables),
    );

    app_builder.add_system_set(
        rival_systems
            .update_rivals
            .after(RoadStageLabels::UpdateRoadTables),
    );

    app_builder.add_system_set(
        racer_systems
            .update_racers
            .after(RoadStageLabels::UpdateRoadTables),
    );

    app_builder.add_system_set(
        text_systems
            .update_texts
            .after(PlayerStageLabels::UpdatePlayerDriving),
    );

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

    app_builder.run();
}
