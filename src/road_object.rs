use bevy::prelude::*;

use crate::{
    joyride::TIME_STEP,
    player::{Player, PlayerSlideDirection},
    racer::Racer,
    road::{RoadDynamic, RoadStatic},
};

const PLAYER_COLLISION_WIDTH: f32 = 30.0;

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

#[derive(Debug, Clone)]
pub enum RoadObjectType {
    RoadSigns(RoadSide),
}

pub struct RoadObject {
    pub x_pos: f32,
    pub z_pos: f32,
    pub collider1: Option<Collider>,
    pub collider2: Option<Collider>,
    pub collision_action: CollisionAction,
}

pub struct Systems {
    pub startup_road_objects: SystemSet,
    pub manage_road_objects: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_road_objects: SystemSet::new().with_system(startup_road_objects.system()),
            manage_road_objects: SystemSet::new()
                .with_system(spawn_segment_objects.system())
                .with_system(check_passed_objects.system()),
        }
    }
}

fn startup_road_objects(mut commands: Commands) {}

fn spawn_segment_objects() {}

fn check_passed_objects(
    mut commands: Commands,
    mut obj_query: Query<(&mut RoadObject, Entity)>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut player: ResMut<Player>,
    racer_query: Query<&Racer>,
) {
    let screen_bottom_z = road_static.z_map()[0];

    let player_speed = racer_query
        .get(player.get_racer_ent())
        .map_or(0.0, |r| r.speed);
    let player_x = -road_dyn.x_offset;

    for (mut obj, ent) in obj_query.iter_mut() {
        obj.z_pos -= player_speed * TIME_STEP;
        if obj.z_pos >= screen_bottom_z {
            continue;
        }

        if object_colliding_with_player(&obj, player_x) {
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

fn object_colliding_with_player(obj: &RoadObject, player_x: f32) -> bool {
    if let Some(coll) = &obj.collider1 {
        if collider_colliding_with_player(coll, obj.x_pos, player_x) {
            return true;
        }
    }
    if let Some(coll) = &obj.collider2 {
        if collider_colliding_with_player(coll, obj.x_pos, player_x) {
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
