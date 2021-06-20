use bevy::{ecs::system::BoxedSystem, prelude::*};
use easy_cast::*;

use crate::{
    player::Player,
    racer::Racer,
    road::{RoadDynamic, ROAD_DISTANCE},
    util::spawn_empty_parent,
};

// Used for layering with other sprites
const SKYBOX_SPRITE_Z: f32 = 0.0;

// How quickly the skybox scrolls left/right in response to road curvature
const SKYBOX_HORIZONTAL_SCROLL_SCALAR: f32 = 1.5;

// How quickly the skybox scrolls downward when the road goes uphill
const SKYBOX_UPHILL_SCROLL_SCALAR: f32 = 0.5;

const SKYBOX_SIZE: (f32, f32) = (640.0, 240.0);

struct Skybox {}

pub struct Systems {
    pub startup_skybox: BoxedSystem<(), ()>,
    pub update_skybox: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_skybox: Box::new(startup_skybox.system()),
            update_skybox: SystemSet::new().with_system(reposition_skybox.system()),
        }
    }
}

fn startup_skybox(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let tex = asset_server.load("textures/sky_bg.png");
    spawn_empty_parent(&mut commands, Vec3::new(0.0, 0.0, SKYBOX_SPRITE_Z))
        .insert(Skybox {})
        .with_children(|cmd| {
            let x_positions: [f32; 3] = [-SKYBOX_SIZE.0, 0.0, SKYBOX_SIZE.0];
            for x in x_positions.iter() {
                cmd.spawn_bundle(SpriteBundle {
                    material: materials.add(tex.clone().into()),
                    transform: Transform::from_translation(Vec3::new(*x, 0.0, 0.0)),
                    ..Default::default()
                });
            }
        });
}

fn reposition_skybox(
    mut skyboxes: Query<&mut Transform, With<Skybox>>,
    racers: Query<&Racer>,
    player: Option<Res<Player>>,
    road_dyn: Option<Res<RoadDynamic>>,
) {
    let (road_draw_height, road_curvature) = match road_dyn {
        Some(road_dyn) => (
            road_dyn.get_draw_height_pixels(),
            road_dyn.get_seg_curvature(0.0),
        ),
        None => return, // No-op if no road
    };

    for mut xform in skyboxes.iter_mut() {
        // Hide skybox over horizon if going uphill
        let y_offset = if road_draw_height < ROAD_DISTANCE {
            let uphill_height: f32 = -f32::conv(ROAD_DISTANCE - road_draw_height);
            uphill_height * SKYBOX_UPHILL_SCROLL_SCALAR
        } else {
            0.0
        };

        let horizontal_scroll_speed = {
            let player_speed = player
                .as_ref()
                .and_then(|p| racers.get(p.get_racer_ent()).ok())
                .map_or(0.0, |r| r.speed);
            -road_curvature * player_speed * SKYBOX_HORIZONTAL_SCROLL_SCALAR
        };

        xform.translation.x =
            (xform.translation.x + horizontal_scroll_speed) % f32::conv(SKYBOX_SIZE.0);

        // Fit the skybox to match the height of the road
        xform.translation.y = f32::conv(road_draw_height - 1) + (SKYBOX_SIZE.1 * 0.5) + y_offset;
    }
}
