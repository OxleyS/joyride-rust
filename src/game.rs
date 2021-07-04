use crate::{joyride, player, racer, rival, road, road_object, skybox, text};
use bevy::prelude::*;

#[derive(StageLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum StartupStageLabels {
    StartupRacerSystems,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum StartupSystemLabels {
    StartupRoad,
}

#[derive(StageLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum GameStageLabels {}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum GameSystemLabels {
    UpdateInput,
    UpdatePlayerDriving,
    UpdatePlayerRoadPosition,
    UpdateRoad,
    UpdateRivals,
    UpdateRoadObjects,
}

struct StageBuilder<'a, S: StageLabel + Clone> {
    app: &'a mut AppBuilder,
    stage_label: S,
}

impl<'a, S: StageLabel + Clone> StageBuilder<'a, S> {
    pub fn new(stage_label: S, app: &'a mut AppBuilder) -> Self {
        Self { app, stage_label }
    }

    pub fn add_systems_after(&mut self, after: Option<GameSystemLabels>, mut sets: Vec<SystemSet>) {
        for set in sets.drain(..) {
            let with_after = if let Some(after) = after {
                set.after(after)
            } else {
                set
            };

            self.app
                .stage(self.stage_label.clone(), |stage: &mut SystemStage| {
                    stage.add_system_set(with_after)
                });
        }
    }

    pub fn add_startup_systems_after(
        &mut self,
        after: Option<StartupSystemLabels>,
        mut sets: Vec<SystemSet>,
    ) {
        for set in sets.drain(..) {
            let with_after = if let Some(after) = after {
                set.after(after)
            } else {
                set
            };

            let stage_label = self.stage_label.clone();
            self.app
                .stage(CoreStage::Startup, |schedule: &mut Schedule| {
                    schedule.add_system_set_to_stage(stage_label, with_after)
                });
        }
    }
}

pub fn setup_game(app: &mut AppBuilder) {
    let joyride_systems = joyride::Systems::new();
    let player_systems = player::Systems::new();
    let road_systems = road::Systems::new();
    let skybox_systems = skybox::Systems::new();
    let text_systems = text::Systems::new();
    let rival_systems = rival::Systems::new();
    let racer_systems = racer::Systems::new();
    let road_object_systems = road_object::Systems::new();

    app.add_startup_stage_before(
        StartupStage::Startup,
        StartupStageLabels::StartupRacerSystems,
        SystemStage::parallel(),
    );

    StageBuilder::new(StartupStageLabels::StartupRacerSystems, app)
        .add_startup_systems_after(None, vec![racer_systems.startup_racer]);

    let mut startup_builder = StageBuilder::new(StartupStage::Startup, app);

    startup_builder.add_startup_systems_after(
        None,
        vec![
            joyride_systems.startup_joyride,
            player_systems.startup_player,
            road_systems
                .startup_road
                .label(StartupSystemLabels::StartupRoad),
            rival_systems.startup_rivals,
            text_systems.startup_text,
            skybox_systems.startup_skybox,
        ],
    );

    startup_builder.add_startup_systems_after(
        Some(StartupSystemLabels::StartupRoad),
        vec![road_object_systems.startup_road_objects],
    );

    // TODO: Enforce that systems are labeled and added in game loop order sequence
    let mut builder = StageBuilder::new(CoreStage::Update, app);

    builder.add_systems_after(None, vec![road_systems.test_curve_road]);

    builder.add_systems_after(
        None,
        vec![joyride_systems
            .update_input
            .label(GameSystemLabels::UpdateInput)],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdateInput),
        vec![player_systems
            .update_player_driving
            .label(GameSystemLabels::UpdatePlayerDriving)],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdatePlayerDriving),
        vec![
            text_systems.update_texts,
            player_systems
                .update_player_road_position
                .label(GameSystemLabels::UpdatePlayerRoadPosition),
        ],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdatePlayerRoadPosition),
        vec![road_systems.update_road.label(GameSystemLabels::UpdateRoad)],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdateRoad),
        vec![rival_systems
            .update_rivals
            .label(GameSystemLabels::UpdateRivals)],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdateRivals),
        vec![road_object_systems
            .manage_road_objects
            .label(GameSystemLabels::UpdateRoadObjects)],
    );

    builder.add_systems_after(
        Some(GameSystemLabels::UpdateRoadObjects),
        vec![
            skybox_systems.update_skybox,
            racer_systems.update_racers,
            player_systems.update_player_visuals,
            rival_systems.update_rival_visuals,
            road_systems.draw_road,
        ],
    );
}
