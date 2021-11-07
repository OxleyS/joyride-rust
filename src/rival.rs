use bevy::prelude::*;
use easy_cast::*;

use crate::{
    debug::{spawn_collision_debug_box, DebugAssets},
    joyride::TIME_STEP,
    player::PLAYER_MAX_NORMAL_SPEED,
    racer::{get_turning_sprite_desc, make_racer, Racer, RacerAssets, NUM_TURN_LEVELS},
    road::{get_draw_params_on_road, RoadDynamic, RoadStatic},
    road_object::{Collider, CollisionAction, RoadObject},
    util::{LocalVisible, SpriteGridDesc},
};

pub enum RivalPalette {
    Green,
    Red,
}

pub struct Rival {
    palette: RivalPalette,
}

pub struct RivalAssets {
    bike_atlas: Handle<TextureAtlas>,
}

pub struct Systems {
    pub startup_rivals: SystemSet,
    pub update_rivals: SystemSet,
    pub update_rival_visuals: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_rivals: SystemSet::new().with_system(startup_rivals.system()),
            update_rivals: SystemSet::new().with_system(update_rivals.system()),
            update_rival_visuals: SystemSet::new().with_system(update_rival_visuals.system()),
        }
    }
}

const RIVAL_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
    rows: 8,
    columns: 8,
};

const LOD_SCALE_MAPPING: [f32; 7] = [0.83, 0.67, 0.55, 0.42, 0.30, 0.22, 0.16];

pub fn spawn_rival(
    commands: &mut Commands,
    x_pos: f32,
    z_pos: f32,
    speed: f32,
    palette: RivalPalette,
    rival_assets: &RivalAssets,
    racer_assets: &RacerAssets,
    debug_assets: &DebugAssets,
) {
    let racer_ent = make_racer(
        commands,
        racer_assets,
        rival_assets.bike_atlas.clone(),
        speed,
        Vec3::default(),
    );

    let coll_left = -15.0;
    let coll_right = 15.0;
    let debug_box = spawn_collision_debug_box(
        commands,
        &debug_assets,
        Vec2::new(0.0, -f32::conv(RIVAL_SPRITE_DESC.tile_size) * 0.5),
        Vec2::new(coll_right - coll_left, 1.0),
    );

    commands
        .entity(racer_ent)
        .insert(Rival { palette })
        .insert(RoadObject {
            x_pos,
            z_pos,
            collider1: Some(Collider {
                left: coll_left,
                right: coll_right,
            }),
            collider2: None,
            collision_action: CollisionAction::SlidePlayer,
        })
        .push_children(&[debug_box]);
}

fn startup_rivals(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let bike_tex = asset_server.load("textures/rival_atlas.png");
    let bike_atlas = RIVAL_SPRITE_DESC.make_atlas(bike_tex);
    let bike_atlas_handle = texture_atlases.add(bike_atlas);

    let rival_assets = RivalAssets {
        bike_atlas: bike_atlas_handle,
    };
    commands.insert_resource(rival_assets);
}

fn update_rivals(
    mut query: Query<(&mut RoadObject, &mut Racer, With<Rival>)>,
    road_dyn: Res<RoadDynamic>,
) {
    for (mut obj, mut racer, _) in query.iter_mut() {
        obj.z_pos += racer.speed * TIME_STEP;

        // Racers go significantly slower than the player, but we want their turn rates to be similar,
        // so we fudge their speed
        racer.turn_rate = road_dyn.get_road_x_pull(obj.z_pos, PLAYER_MAX_NORMAL_SPEED);
    }
}

fn update_rival_visuals(
    mut query: Query<(
        &Rival,
        &RoadObject,
        &mut Racer,
        &mut TextureAtlasSprite,
        &mut LocalVisible,
        &mut Transform,
    )>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    for (rival, obj, mut racer, mut sprite, mut visible, mut xform) in query.iter_mut() {
        let draw_params = get_draw_params_on_road(&road_static, &road_dyn, obj.x_pos, obj.z_pos);

        let mut is_visible = false;
        if let Some(draw_params) = draw_params {
            xform.translation.x = draw_params.draw_pos.x;
            xform.translation.y =
                draw_params.draw_pos.y + (f32::conv(RIVAL_SPRITE_DESC.tile_size) * 0.5);

            let lod_level: u8 = LOD_SCALE_MAPPING
                .binary_search_by(|x| draw_params.scale.partial_cmp(&x).unwrap())
                .unwrap_or_else(|x| x)
                .cast();
            racer.lod_level = lod_level;

            let sprite_params = get_turning_sprite_desc(racer.turn_rate);
            let sprite_x = match rival.palette {
                RivalPalette::Green => sprite_params.turn_idx,
                RivalPalette::Red => sprite_params.turn_idx + u32::conv(NUM_TURN_LEVELS),
            };
            sprite.flip_x = sprite_params.flip_x;
            sprite.index = RIVAL_SPRITE_DESC.get_sprite_index(sprite_x, lod_level.cast());

            is_visible = true;
        }

        if visible.is_visible != is_visible {
            visible.is_visible = is_visible;
        }
    }
}
