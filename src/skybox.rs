use bevy::prelude::*;
use easy_cast::*;

use crate::road::{RoadDynamic, RoadStageLabels};

// Used for layering with other sprites
const SKYBOX_SPRITE_Z: f32 = 0.0;

struct Skybox {
    tex: Handle<Texture>,
    ent: Entity,
}

pub fn startup_skybox(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let tex = asset_server.load("textures/sky_bg.png");
    let ent = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(tex.clone().into()),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, SKYBOX_SPRITE_Z)),
            ..Default::default()
        })
        .id();

    commands.insert_resource(Skybox { tex, ent });
}

pub fn add_skybox_update_systems(system_set: SystemSet) -> SystemSet {
    system_set.with_system(
        reposition_skybox
            .system()
            .after(RoadStageLabels::UpdateRoadTables),
    )
}

// TODO: Ordering with road update
fn reposition_skybox(
    skybox: Res<Skybox>,
    road_dyn: Option<Res<RoadDynamic>>,
    textures: Res<Assets<Texture>>,
    mut positioning: Query<(&mut Transform, &Sprite)>,
) {
    // Do nothing if the skybox texture has not loaded yet
    if textures.get(&skybox.tex).is_none() {
        return;
    }

    let road_draw_height = match road_dyn {
        Some(road_dyn) => road_dyn.draw_height,
        None => return, // No-op if no road
    };

    let (mut xform, sprite) = match positioning.get_mut(skybox.ent) {
        Ok(pos) => pos,
        Err(_) => return, // No-op if components are missing
    };

    // TODO: Hide skybox over horizon if going uphill
    // TODO: There seems to be some sort of mispositioning going on. The bug happens when hill intensity rapidly changes, and
    // places the skybox one frame earlier than the road drawing fills the gap. Something out-of-step with a Bevy system?

    let size = sprite.size;
    xform.translation.y = f32::conv(road_draw_height - 1) + (size.y * 0.5);
}
