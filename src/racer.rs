use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::TIME_STEP,
    util::{LocalVisible, SpriteGridDesc},
};

pub struct OverlayOffsets(pub [(i32, i32); NUM_TURN_LEVELS]);

const TIRE_OFFSETS: [OverlayOffsets; NUM_RACER_LODS * 2] = [
    // LOD level 0
    // Up cycle
    OverlayOffsets([(0, -16), (-1, -16), (-3, -17), (-10, -19)]),
    // Down cycle
    OverlayOffsets([(0, -19), (-2, -19), (-6, -21), (-12, -21)]),
    // LOD level 1
    // Up cycle
    OverlayOffsets([(0, -18), (0, -17), (-3, -17), (-8, -21)]),
    // Down cycle
    OverlayOffsets([(0, -21), (-2, -22), (-5, -22), (-12, -24)]),
    // LOD level 2
    // Up cycle
    OverlayOffsets([(0, -21), (-1, -21), (-2, -22), (-6, -22)]),
    // Down cycle
    OverlayOffsets([(0, -23), (-2, -23), (-3, -24), (-9, -25)]),
    // LOD level 3
    // Up cycle
    OverlayOffsets([(1, -23), (1, -23), (-4, -24), (-7, -24)]),
    OverlayOffsets([(1, -24), (0, -25), (-5, -26), (-9, -26)]),
];
fn make_tire_overlay(racer: Entity) -> RacerOverlay {
    RacerOverlay::new(racer, 2, 1, 4, true, true, &TIRE_SPRITE_DESC, &TIRE_OFFSETS)
}

struct Tire {}

const TIRE_Z_OFFSET: f32 = 0.1;
const TIRE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 16,
    rows: 4,
    columns: 4,
};

pub struct RacerOverlay {
    pub offset_cycle_pos: u8,
    pub sprite_cycle_pos: u8,

    // The overlay sets visibility based on LOD, so this sets whether it should
    // be drawn in the first place
    pub is_visible: bool,

    racer: Entity,

    offset_cycle_length: u8,
    sprite_cycle_length: u8,
    num_lod_levels: u8,
    turnable: bool,
    flippable: bool,
    sprite_desc: &'static SpriteGridDesc,

    // Laid out as [[OverlayOffsets; offset_cycle_length]; num_lod_levels;], except continuously
    offset_table: &'static [OverlayOffsets],
}

impl RacerOverlay {
    pub fn new(
        racer: Entity,
        offset_cycle_length: u8,
        sprite_cycle_length: u8,
        num_lod_levels: u8,
        turnable: bool,
        flippable: bool,
        sprite_desc: &'static SpriteGridDesc,
        offset_table: &'static [OverlayOffsets],
    ) -> Self {
        let expected_num_offsets = offset_cycle_length * num_lod_levels;
        assert!(
            offset_table.len() == expected_num_offsets as usize,
            "Offset table size mismatch: expected {}, was {}",
            expected_num_offsets,
            offset_table.len()
        );

        let expected_columns =
            (if turnable { NUM_TURN_LEVELS } else { 1 }) * sprite_cycle_length as usize;
        assert!(
            sprite_desc.columns as usize >= expected_columns,
            "Sprite grid not wide enough for all turn levels + sprite cycle"
        );
        assert!(
            sprite_desc.rows >= num_lod_levels as u32,
            "Sprite grid not tall enough for all LOD levels"
        );
        Self {
            racer,
            offset_cycle_pos: 0,
            sprite_cycle_pos: 0,
            is_visible: true,
            sprite_cycle_length,
            offset_cycle_length,
            num_lod_levels,
            turnable,
            flippable,
            sprite_desc,
            offset_table,
        }
    }

    pub fn get_sprite_cycle_length(&self) -> u8 {
        self.sprite_cycle_length
    }
}

const RACER_BASE_Z: f32 = 300.0;
pub const RACER_MAX_SPEED: f32 = 10.43;
pub const MAX_TURN_RATE: f32 = 400.0;
const NUM_RACER_LODS: usize = 4;
pub const NUM_TURN_LEVELS: usize = 4;
pub const RACER_ROAD_CURVE_SCALAR: f32 = 60.0;

pub struct RacerAssets {
    tire_atlas: Handle<TextureAtlas>,
}

pub struct Racer {
    pub turn_rate: f32,
    pub speed: f32,
    pub z_bias: f32,
    pub lod_level: u8,
}

pub fn startup_racer(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let tire_tex = asset_server.load("textures/tire_atlas.png");
    let tire_atlas = TIRE_SPRITE_DESC.make_atlas(tire_tex);

    commands.insert_resource(RacerAssets {
        tire_atlas: texture_atlases.add(tire_atlas),
    });
}

pub fn add_racer_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        .with_system(update_tires.system())
        .with_system(update_racer_overlays.system())
        .with_system(update_racer_z.system())
}

pub fn make_racer(
    commands: &mut Commands,
    racer_assets: Res<RacerAssets>,
    bike_atlas: Handle<TextureAtlas>,
    z_bias: f32,
) -> Entity {
    let racer_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: bike_atlas.clone(),
            ..Default::default()
        })
        .insert(Racer {
            lod_level: 0,
            turn_rate: 0.0,
            speed: 0.0,
            z_bias,
        })
        .insert(LocalVisible::default())
        .id();

    let tire_xform = Transform::from_translation(Vec3::new(0.0, 0.0, TIRE_Z_OFFSET));

    let tire_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: racer_assets.tire_atlas.clone(),
            transform: tire_xform,
            ..Default::default()
        })
        .insert(LocalVisible::default())
        .insert(Timer::from_seconds(0.1, false))
        .insert(make_tire_overlay(racer_ent))
        .insert(Tire {})
        .id();

    commands.entity(racer_ent).push_children(&[tire_ent]);
    racer_ent
}

fn update_tires(
    mut overlay_query: Query<(&mut RacerOverlay, &mut Timer), With<Tire>>,
    racer_query: Query<&Racer>,
) {
    for (mut overlay, mut timer) in overlay_query.iter_mut() {
        let speed = racer_query.get(overlay.racer).map_or(0.0, |r| r.speed);

        timer.tick(Duration::from_secs_f32(TIME_STEP));
        if timer.finished() {
            overlay.offset_cycle_pos = (overlay.offset_cycle_pos + 1) % overlay.offset_cycle_length;

            let new_secs = get_tire_cycle_seconds(speed);
            timer.set_duration(Duration::from_secs_f32(new_secs));
            timer.reset();
        }
    }
}

// TODO: This needs to run after the player and all rivals have updated, or we get off-by-one-frames
fn update_racer_overlays(
    mut overlay_query: Query<(
        &RacerOverlay,
        &mut LocalVisible,
        &mut TextureAtlasSprite,
        &mut Transform,
    )>,
    racer_query: Query<&Racer>,
) {
    for (overlay, mut visible, mut sprite, mut xform) in overlay_query.iter_mut() {
        let (turn_rate, lod_level) = racer_query
            .get(overlay.racer)
            .map_or((0.0, 0), |r| (r.turn_rate, r.lod_level));

        if lod_level >= overlay.num_lod_levels {
            visible.is_visible = false;
            continue;
        }
        visible.is_visible = overlay.is_visible;

        let RacerSpriteParams { turn_idx, flip_x } = get_turning_sprite_desc(turn_rate);

        let lod_idx = u8::min(lod_level, overlay.num_lod_levels - 1);
        let offsets_idx = (overlay.offset_cycle_length * lod_idx) + overlay.offset_cycle_pos;

        let offsets = &overlay.offset_table[offsets_idx as usize];
        let mut turn_level_offset = offsets.0[turn_idx as usize];

        if flip_x {
            turn_level_offset.0 = -turn_level_offset.0;
        }
        sprite.flip_x = if overlay.flippable { flip_x } else { false };

        let sprite_x: u32 = if overlay.turnable {
            (u32::conv(overlay.sprite_cycle_pos) * u32::conv(NUM_TURN_LEVELS)) + turn_idx
        } else {
            overlay.sprite_cycle_pos.cast()
        };

        // One row per LOD level, highest resolution first.
        // Each LOD level has four columns, one for each distinct sprite based on how hard the racer is turning
        sprite.index = overlay
            .sprite_desc
            .get_sprite_index(sprite_x, lod_idx as u32);

        xform.translation.x = f32::conv(turn_level_offset.0);
        xform.translation.y = f32::conv(turn_level_offset.1);
    }
}

fn update_racer_z(mut query: Query<(&Racer, &mut Transform)>) {
    for (racer, mut xform) in query.iter_mut() {
        xform.translation.z = RACER_BASE_Z - xform.translation.y + racer.z_bias;
    }
}

pub struct RacerSpriteParams {
    pub turn_idx: u32,
    pub flip_x: bool,
}

pub fn get_turning_sprite_desc(turn_rate: f32) -> RacerSpriteParams {
    let turn_div = turn_rate / (MAX_TURN_RATE / f32::conv(NUM_TURN_LEVELS));
    let turn_div_trunc = i32::conv_trunc(turn_div);
    let turn_idx = u32::min(3, u32::conv(turn_div_trunc.abs()));

    RacerSpriteParams {
        turn_idx,
        flip_x: turn_div_trunc < 0,
    }
}

fn get_tire_cycle_seconds(speed: f32) -> f32 {
    f32::clamp((RACER_MAX_SPEED / speed) / 16.0, 0.02, 4.0)
}
