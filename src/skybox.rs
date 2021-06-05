use bevy::prelude::*;
use easy_cast::*;

use crate::road::{RoadDynamic, RoadStageLabels, ROAD_DISTANCE};

// Used for layering with other sprites
const SKYBOX_SPRITE_Z: f32 = 0.0;

// How quickly the skybox scrolls downward when the road goes uphill
const SKYBOX_UPHILL_SCROLL_SCALAR: f32 = 0.3;

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

fn reposition_skybox(
    skybox: Res<Skybox>,
    road_dyn: Option<Res<RoadDynamic>>,
    textures: Res<Assets<Texture>>,
    mut positioning: Query<(&mut Transform, &Sprite)>,
) {
    // Do nothing if the skybox texture has not loaded yet (we won't know its size)
    if textures.get(&skybox.tex).is_none() {
        return;
    }

    let road_draw_height = match road_dyn {
        Some(road_dyn) => road_dyn.get_draw_height_pixels(),
        None => return, // No-op if no road
    };

    let (mut xform, sprite) = match positioning.get_mut(skybox.ent) {
        Ok(pos) => pos,
        Err(_) => return, // No-op if components are missing
    };

    // Hide skybox over horizon if going uphill
    let y_offset = if road_draw_height < ROAD_DISTANCE {
        let uphill_height: f32 = -f32::conv(ROAD_DISTANCE - road_draw_height);
        uphill_height * SKYBOX_UPHILL_SCROLL_SCALAR
    } else {
        0.0
    };

    // TODO: Horizontal scrolling as well

    // Fit the skybox to match the height of the road
    let size = sprite.size;
    xform.translation.y = f32::conv(road_draw_height - 1) + (size.y * 0.5) + y_offset;
}
