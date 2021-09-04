use bevy::prelude::*;
use easy_cast::*;

use crate::{
    debug::{spawn_collision_debug_box, DebugAssets},
    joyride::TIME_STEP,
    player::{Player, PlayerSlideDirection},
    racer::Racer,
    road::{get_draw_params_on_road, RoadDynamic, RoadStatic, SEGMENT_LENGTH},
    util::{LocalVisible, SpriteGridDesc},
};

pub const PLAYER_COLLISION_WIDTH: f32 = 30.0;

pub const ROAD_OBJ_BASE_Z: f32 = 300.0;

const ROAD_OBJ_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 128,
    rows: 10,
    columns: 3,
};

// TODO: Share this with Rival?
const LOD_SCALE_MAPPING: [f32; 9] = [0.83, 0.67, 0.55, 0.42, 0.30, 0.26, 0.16, 0.09, 0.06];

const ROAD_SIGN_Z_OFFSETS: [f32; 3] = [
    SEGMENT_LENGTH * 0.35,
    SEGMENT_LENGTH * 0.5,
    SEGMENT_LENGTH * 0.65,
];

#[derive(Debug, Clone)]
pub struct Collider {
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum CollisionAction {
    SlidePlayer,
    CrashPlayer,
}

#[derive(Debug, Clone, Copy)]
pub enum RoadSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum RoadSignType {
    Oxman,
    BeatDown,
    Turn(bool),
}

#[derive(Debug, Clone)]
pub enum RoadObjectType {
    RoadSigns(RoadSignType, RoadSide),
}

pub struct RoadObject {
    pub x_pos: f32,
    pub z_pos: f32,
    pub collider1: Option<Collider>,
    pub collider2: Option<Collider>,
    pub collision_action: CollisionAction,
}

struct RoadObjectAssets {
    sprite_atlas: Handle<TextureAtlas>,
}

#[derive(Debug, Clone)]
struct RoadObjectSpriteSelector {
    sprite_set_idx: u32,
    flip: bool,
}

struct Spawner {
    last_seg_idx: usize,
}

pub struct Systems {
    pub startup_road_objects: SystemSet,
    pub manage_road_objects: SystemSet,
    pub update_road_object_visuals: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_road_objects: SystemSet::new().with_system(startup_road_objects.system()),
            manage_road_objects: SystemSet::new()
                .with_system(check_passed_objects.system().label("check_passed_objects"))
                .with_system(spawn_segment_objects.system().after("check_passed_objects"))
                .with_system(update_road_object_z.system().after("check_passed_objects")),
            update_road_object_visuals: SystemSet::new()
                .with_system(update_road_object_visuals.system()),
        }
    }
}

fn startup_road_objects(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    debug_assets: Res<DebugAssets>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    let tex = asset_server.load("textures/road_object_atlas.png");
    let atlas = ROAD_OBJ_SPRITE_DESC.make_atlas(tex);

    let assets = RoadObjectAssets {
        sprite_atlas: texture_atlases.add(atlas),
    };

    let z_map = road_static.z_map();
    let far_z = z_map[z_map.len() - 1];
    let road_point = road_dyn.query_road_point(far_z);

    for seg_idx in 0..=road_point.seg_idx {
        let seg = road_dyn.get_bounded_seg(seg_idx);
        let seg_start_z = SEGMENT_LENGTH * f32::conv(seg_idx);
        if let Some(spawn_type) = &seg.spawn_object_type {
            spawn_objects(
                spawn_type,
                seg_start_z,
                &assets,
                &&debug_assets,
                &mut commands,
            );
        }
    }

    commands.insert_resource(assets);
    commands.insert_resource(Spawner {
        last_seg_idx: road_point.seg_idx,
    });
}

fn spawn_segment_objects(
    mut commands: Commands,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut spawner: ResMut<Spawner>,
    assets: Res<RoadObjectAssets>,
    debug_assets: Res<DebugAssets>,
) {
    let z_map = road_static.z_map();
    let far_z = z_map[z_map.len() - 1];
    let road_point = road_dyn.query_road_point(far_z);

    if road_point.seg_idx != spawner.last_seg_idx {
        let seg_start_z = far_z - road_point.seg_pos;

        if let Some(spawn_type) = &road_point.seg.spawn_object_type {
            spawn_objects(
                spawn_type,
                seg_start_z,
                &assets,
                &debug_assets,
                &mut commands,
            );
        }
        spawner.last_seg_idx = road_point.seg_idx;
    }
}

fn spawn_objects(
    obj_type: &RoadObjectType,
    seg_start_z: f32,
    assets: &RoadObjectAssets,
    debug_assets: &DebugAssets,
    commands: &mut Commands,
) {
    match obj_type {
        &RoadObjectType::RoadSigns(sign_type, road_side) => {
            let selector: RoadObjectSpriteSelector = match sign_type {
                RoadSignType::Oxman => RoadObjectSpriteSelector {
                    sprite_set_idx: 0,
                    flip: false,
                },
                RoadSignType::BeatDown => RoadObjectSpriteSelector {
                    sprite_set_idx: 1,
                    flip: false,
                },
                RoadSignType::Turn(flip) => RoadObjectSpriteSelector {
                    sprite_set_idx: 2,
                    flip,
                },
            };

            let x_pos = match road_side {
                RoadSide::Left => -204.0,
                RoadSide::Right => 204.0,
            };

            for z_pos in ROAD_SIGN_Z_OFFSETS.iter() {
                let coll_left = -43.0;
                let coll_right = 43.0;
                let debug_box = spawn_collision_debug_box(
                    commands,
                    debug_assets,
                    Vec2::new(0.0, -f32::conv(ROAD_OBJ_SPRITE_DESC.tile_size) * 0.5),
                    Vec2::new(coll_right - coll_left, 1.0),
                );

                let road_obj = RoadObject {
                    x_pos,
                    z_pos: *z_pos + seg_start_z,
                    collider1: Some(Collider {
                        left: coll_left,
                        right: coll_right,
                    }),
                    collider2: None,
                    collision_action: CollisionAction::CrashPlayer,
                };

                commands
                    .spawn_bundle(SpriteSheetBundle {
                        texture_atlas: assets.sprite_atlas.clone(),
                        ..Default::default()
                    })
                    .insert(road_obj)
                    .insert(selector.clone())
                    .insert(LocalVisible::default())
                    .push_children(&[debug_box]);
            }
        }
    }
}

fn check_passed_objects(
    mut commands: Commands,
    mut obj_query: Query<(&mut RoadObject, Entity)>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut player: ResMut<Player>,
    racer_query: Query<&Racer>,
) {
    let screen_bottom_z = road_static.z_map()[0];
    let screen_bottom_scale = road_static.scale_map()[0];

    let player_speed = racer_query
        .get(player.get_racer_ent())
        .map_or(0.0, |r| r.speed);
    let player_x = -road_dyn.x_offset;

    for (mut obj, ent) in obj_query.iter_mut() {
        obj.z_pos -= player_speed * TIME_STEP;
        if obj.z_pos >= screen_bottom_z {
            continue;
        }

        if object_colliding_with_player(&obj, player_x, screen_bottom_scale) {
            match obj.collision_action {
                CollisionAction::CrashPlayer => {
                    player.crash();
                }
                CollisionAction::SlidePlayer => {
                    let direction = if obj.x_pos > player_x {
                        PlayerSlideDirection::Left
                    } else {
                        PlayerSlideDirection::Right
                    };
                    player.slide(direction);
                }
            }
        }

        commands.entity(ent).despawn_recursive();
    }
}

fn object_colliding_with_player(obj: &RoadObject, player_x: f32, scale: f32) -> bool {
    if let Some(coll) = &obj.collider1 {
        if collider_colliding_with_player(coll, obj.x_pos * scale, player_x) {
            return true;
        }
    }
    if let Some(coll) = &obj.collider2 {
        if collider_colliding_with_player(coll, obj.x_pos * scale, player_x) {
            return true;
        }
    }

    return false;
}

fn collider_colliding_with_player(collider: &Collider, x_pos: f32, player_x: f32) -> bool {
    let coll_left = collider.left + x_pos;
    let coll_right = collider.right + x_pos;
    let player_left = player_x - (PLAYER_COLLISION_WIDTH * 0.5);
    let player_right = player_x + (PLAYER_COLLISION_WIDTH * 0.5);

    coll_left <= player_right && player_left <= coll_right
}

fn update_road_object_z(mut query: Query<(&mut Transform, With<RoadObject>)>) {
    for (mut xform, _) in query.iter_mut() {
        xform.translation.z = ROAD_OBJ_BASE_Z - xform.translation.y;
    }
}

fn update_road_object_visuals(
    query: Query<(
        &RoadObjectSpriteSelector,
        &RoadObject,
        &mut TextureAtlasSprite,
        &mut LocalVisible,
        &mut Transform,
    )>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    query.for_each_mut(|(selector, object, mut sprite, mut visible, mut xform)| {
        let draw_params =
            get_draw_params_on_road(&road_static, &road_dyn, object.x_pos, object.z_pos);
        let mut is_visible = false;

        if let Some(draw_params) = draw_params {
            xform.translation.x = draw_params.draw_pos.x;
            xform.translation.y =
                draw_params.draw_pos.y + (f32::conv(ROAD_OBJ_SPRITE_DESC.tile_size) * 0.5);

            let lod_level: u32 = LOD_SCALE_MAPPING
                .binary_search_by(|x| draw_params.scale.partial_cmp(&x).unwrap())
                .unwrap_or_else(|x| x)
                .cast();

            let sprite_x: u32 = selector.sprite_set_idx;
            let sprite_y: u32 = lod_level;
            sprite.index = ROAD_OBJ_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
            sprite.flip_x = selector.flip;

            is_visible = true;
        }

        if visible.is_visible != is_visible {
            visible.is_visible = is_visible;
        }
    });
}
