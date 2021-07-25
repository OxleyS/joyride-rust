use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::TIME_STEP,
    util::{LocalVisible, SpriteGridDesc},
};

pub struct OverlayOffsets(pub [(i32, i32); NUM_TURN_LEVELS]);

const NUM_TIRE_LODS: u8 = 5;
const TIRE_OFFSETS: [OverlayOffsets; NUM_TIRE_LODS as usize * 2] = [
    // LOD level 0
    // Up cycle
    OverlayOffsets([(0, -16), (-1, -16), (-3, -17), (-10, -19)]),
    // Down cycle
    OverlayOffsets([(0, -19), (-2, -19), (-6, -21), (-12, -21)]),
    // LOD level 1
    // Up cycle
    OverlayOffsets([(1, -19), (0, -17), (-3, -17), (-8, -21)]),
    // Down cycle
    OverlayOffsets([(1, -22), (-2, -22), (-5, -22), (-12, -24)]),
    // LOD level 2
    // Up cycle
    OverlayOffsets([(0, -21), (-1, -21), (-2, -22), (-6, -22)]),
    // Down cycle
    OverlayOffsets([(0, -23), (-2, -23), (-3, -24), (-9, -25)]),
    // LOD level 3
    // Up cycle
    OverlayOffsets([(1, -23), (1, -23), (-4, -24), (-7, -24)]),
    OverlayOffsets([(1, -24), (0, -25), (-5, -26), (-9, -26)]),
    // LOD level 4
    // Up cycle
    OverlayOffsets([(1, -25), (2, -23), (-3, -24), (-6, -24)]),
    OverlayOffsets([(1, -26), (1, -25), (-4, -26), (-8, -26)]),
];
fn make_tire_overlay() -> RacerOverlay {
    RacerOverlay::new(
        2,
        1,
        NUM_TIRE_LODS,
        true,
        true,
        &TIRE_SPRITE_DESC,
        &TIRE_OFFSETS,
    )
}

pub struct Tire {}

const TIRE_Z_OFFSET: f32 = 0.1;
const TIRE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 16,
    rows: 5,
    columns: 4,
};

pub struct RacerOverlay {
    pub offset_cycle_pos: u8,
    pub sprite_cycle_pos: u8,

    // The overlay sets visibility based on LOD, so this sets whether it should
    // be drawn in the first place
    pub is_visible: bool,

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

pub const RACER_MAX_SPEED: f32 = 10.43;
pub const MAX_TURN_RATE: f32 = 400.0;
pub const NUM_TURN_LEVELS: usize = 4;

pub struct RacerAssets {
    tire_atlas: Handle<TextureAtlas>,
}

pub struct Racer {
    pub turn_rate: f32,
    pub speed: f32,
    pub lod_level: u8,
    pub tire_ent: Entity,
}

pub struct Systems {
    pub startup_racer: SystemSet,
    pub update_racers: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_racer: SystemSet::new().with_system(startup_racer.system()),
            update_racers: SystemSet::new()
                .with_system(update_tires.system())
                .with_system(update_racer_overlays.system()),
        }
    }
}

fn startup_racer(
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

pub fn make_racer(
    commands: &mut Commands,
    racer_assets: Res<RacerAssets>,
    bike_atlas: Handle<TextureAtlas>,
    speed: f32,
    translation: Vec3,
) -> Entity {
    let tire_xform = Transform::from_translation(Vec3::new(0.0, 0.0, TIRE_Z_OFFSET));
    let tire_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: racer_assets.tire_atlas.clone(),
            transform: tire_xform,
            ..Default::default()
        })
        .insert(LocalVisible::default())
        .insert(Timer::from_seconds(0.1, false))
        .insert(make_tire_overlay())
        .insert(Tire {})
        .id();

    let racer_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: bike_atlas.clone(),
            transform: Transform::from_translation(translation),
            ..Default::default()
        })
        .insert(Racer {
            lod_level: 0,
            turn_rate: 0.0,
            speed,
            tire_ent,
        })
        .insert(LocalVisible::default())
        .push_children(&[tire_ent])
        .id();

    racer_ent
}

fn update_tires(
    mut overlay_query: Query<(&mut RacerOverlay, &mut Timer, &Parent), With<Tire>>,
    racer_query: Query<&Racer>,
) {
    for (mut overlay, mut timer, parent) in overlay_query.iter_mut() {
        let speed = racer_query.get(parent.0).map_or(0.0, |r| r.speed);

        timer.tick(Duration::from_secs_f32(TIME_STEP));
        if timer.finished() {
            overlay.offset_cycle_pos = (overlay.offset_cycle_pos + 1) % overlay.offset_cycle_length;

            let new_secs = get_tire_cycle_seconds(speed);
            timer.set_duration(Duration::from_secs_f32(new_secs));
            timer.reset();
        }
    }
}

fn update_racer_overlays(
    mut overlay_query: Query<(
        &RacerOverlay,
        &mut LocalVisible,
        &mut TextureAtlasSprite,
        &mut Transform,
        &Parent,
    )>,
    racer_query: Query<&Racer>,
) {
    for (overlay, mut visible, mut sprite, mut xform, parent) in overlay_query.iter_mut() {
        let (turn_rate, lod_level) = racer_query
            .get(parent.0)
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
    f32::clamp((RACER_MAX_SPEED / speed) / 16.0, 0.02, 0.5)
}
