use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::TIME_STEP,
    player::Player,
    racer::{get_turning_sprite_desc, make_racer, Racer, RacerAssets, NUM_TURN_LEVELS},
    road::{get_draw_params_on_road, RoadDynamic, RoadStatic},
    util::{LocalVisible, SpriteGridDesc},
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

pub struct Systems {
    pub startup_rivals: SystemSet,
    pub update_rivals: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_rivals: SystemSet::new().with_system(startup_rival.system()),
            update_rivals: SystemSet::new().with_system(update_rival.system()),
        }
    }
}

const RIVAL_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
    rows: 6,
    columns: 8,
};

const LOD_SCALE_MAPPING: [f32; 5] = [0.83, 0.67, 0.59, 0.34, 0.16];

fn startup_rival(
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
        palette: RivalPalette::Red,
        x_pos: 50.0,
        z_pos: 1.5,
    });
}

fn update_rival(
    mut query: QuerySet<(
        Query<(
            &mut Rival,
            &mut Racer,
            &mut TextureAtlasSprite,
            &mut LocalVisible,
            &mut Transform,
        )>,
        Query<&Racer>,
    )>,
    player: Res<Player>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    let player_speed = query
        .q1()
        .get(player.get_racer_ent())
        .map_or(0.0, |r| r.speed);

    for (mut rival, mut racer, mut sprite, mut visible, mut xform) in query.q0_mut().iter_mut() {
        racer.speed = 2.0; // TODO: Temporary
        rival.z_pos += (racer.speed - player_speed) * TIME_STEP;

        let draw_params =
            get_draw_params_on_road(&road_static, &road_dyn, rival.x_pos, rival.z_pos);
        if let Some(draw_params) = draw_params {
            xform.translation.x = draw_params.draw_pos.0;
            xform.translation.y =
                draw_params.draw_pos.1 + (f32::conv(RIVAL_SPRITE_DESC.tile_size) * 0.5);

            let lod_level: u8 = LOD_SCALE_MAPPING
                .binary_search_by(|x| draw_params.scale.partial_cmp(&x).unwrap())
                .unwrap_or_else(|x| x)
                .cast();
            racer.lod_level = lod_level;

            // TODO: Lerp here for smooth turning
            racer.turn_rate = road_dyn.get_road_x_pull(rival.z_pos, racer.speed);

            let sprite_params = get_turning_sprite_desc(racer.turn_rate);
            let sprite_x = match rival.palette {
                RivalPalette::Green => sprite_params.turn_idx,
                RivalPalette::Red => sprite_params.turn_idx + u32::conv(NUM_TURN_LEVELS),
            };
            sprite.flip_x = sprite_params.flip_x;
            sprite.index = RIVAL_SPRITE_DESC.get_sprite_index(sprite_x, lod_level.cast());

            visible.is_visible = true;
        } else {
            visible.is_visible = false;
        }
    }
}
