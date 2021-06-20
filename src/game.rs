use crate::{joyride, player, racer, rival, road, skybox, text};
use bevy::prelude::*;

#[derive(StageLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum StartupStageLabels {
    StartupRacerSystems,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum StartupSystemLabels {}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum GameSystemLabels {
    UpdateInput,
    UpdatePlayerDriving,
    UpdatePlayerRoadPosition,
    UpdateRoad,
}

pub fn setup_game(app: &mut AppBuilder) {
    let joyride_systems = joyride::Systems::new();
    let player_systems = player::Systems::new();
    let road_systems = road::Systems::new();
    let skybox_systems = skybox::Systems::new();
    let text_systems = text::Systems::new();
    let rival_systems = rival::Systems::new();
    let racer_systems = racer::Systems::new();

    app.add_startup_stage_before(
        StartupStage::Startup,
        StartupStageLabels::StartupRacerSystems,
        SystemStage::parallel(),
    );

    add_startup_systems(
        app,
        StartupStageLabels::StartupRacerSystems,
        None,
        vec![racer_systems.startup_racer],
    );

    add_startup_systems(
        app,
        StartupStage::Startup,
        None,
        vec![
            joyride_systems.startup_joyride,
            player_systems.startup_player,
            road_systems.startup_road,
            rival_systems.startup_rivals,
            text_systems.startup_text,
            skybox_systems.startup_skybox,
        ],
    );

    //app.add_system_set(road_systems.test_curve_road);

    add_systems_after(
        app,
        None,
        vec![joyride_systems
            .update_input
            .label(GameSystemLabels::UpdateInput)],
    );

    add_systems_after(
        app,
        Some(GameSystemLabels::UpdateInput),
        vec![player_systems
            .update_player_driving
            .label(GameSystemLabels::UpdatePlayerDriving)],
    );

    add_systems_after(
        app,
        Some(GameSystemLabels::UpdatePlayerDriving),
        vec![
            text_systems.update_texts,
            player_systems
                .update_player_road_position
                .label(GameSystemLabels::UpdatePlayerRoadPosition),
        ],
    );

    add_systems_after(
        app,
        Some(GameSystemLabels::UpdatePlayerRoadPosition),
        vec![
            player_systems.update_player_visuals,
            road_systems.update_road.label(GameSystemLabels::UpdateRoad),
        ],
    );

    add_systems_after(
        app,
        Some(GameSystemLabels::UpdateRoad),
        vec![
            skybox_systems.update_skybox,
            rival_systems.update_rivals,
            racer_systems.update_racers,
            road_systems.draw_road,
        ],
    );
}

fn add_startup_systems<S: StageLabel + Clone>(
    app: &mut AppBuilder,
    stage_label: S,
    after: Option<StartupSystemLabels>,
    mut sets: Vec<SystemSet>,
) {
    for set in sets.drain(..) {
        let with_after = if let Some(after) = after {
            set.after(after)
        } else {
            set
        };

        app.stage(CoreStage::Startup, |schedule: &mut Schedule| {
            schedule.add_system_set_to_stage(stage_label.clone(), with_after)
        });
    }
}

fn add_systems_after(
    app: &mut AppBuilder,
    after: Option<GameSystemLabels>,
    mut sets: Vec<SystemSet>,
) {
    for set in sets.drain(..) {
        let with_after = if let Some(after) = after {
            set.after(after)
        } else {
            set
        };

        app.add_system_set(with_after);
    }
}
