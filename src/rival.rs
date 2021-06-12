use bevy::prelude::*;
use easy_cast::*;

use crate::{
    player::PlayerStageLabels,
    racer::{make_racer, RacerAssets},
    road::{get_draw_params_on_road, RoadDynamic, RoadStageLabels, RoadStatic},
    util::SpriteGridDesc,
};

enum RivalPalette {
    Green,
    Red,
}

struct Rival {
    palette: RivalPalette,
    x_pos: f32,
    z_pos: f32,
}

const RIVAL_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
    rows: 6,
    columns: 8,
};

pub fn startup_rival(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    racer_assets: Res<RacerAssets>,
    asset_server: Res<AssetServer>,
) {
    let bike_tex = asset_server.load("textures/rival_atlas.png");
    let bike_atlas = RIVAL_SPRITE_DESC.make_atlas(bike_tex);

    let racer_ent = make_racer(
        &mut commands,
        racer_assets,
        texture_atlases.add(bike_atlas),
        0.0,
    );

    commands.entity(racer_ent).insert(Rival {
        palette: RivalPalette::Green,
        x_pos: 20.0,
        z_pos: 10.5,
    });
}

pub fn add_rival_update_systems(system_set: SystemSet) -> SystemSet {
    system_set.with_system(
        update_rival
            .system()
            .after(PlayerStageLabels::UpdatePlayerRoadPosition)
            .after(RoadStageLabels::UpdateRoadTables),
    )
}

fn update_rival(
    mut query: Query<(&Rival, &mut Visible, &mut Transform)>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    for (rival, mut visible, mut xform) in query.iter_mut() {
        let draw_params =
            get_draw_params_on_road(&road_static, &road_dyn, rival.x_pos, rival.z_pos);
        if let Some(draw_params) = draw_params {
            xform.translation.x = draw_params.draw_pos.0;
            xform.translation.y =
                draw_params.draw_pos.1 + (f32::conv(RIVAL_SPRITE_DESC.tile_size) * 0.5);
            visible.is_visible = true;
        } else {
            visible.is_visible = false;
        }
    }
}
