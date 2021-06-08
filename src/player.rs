use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{JoyrideInput, JoyrideInputState, FIELD_WIDTH, TIME_STEP},
    road::{is_offroad, RoadDynamic, RoadStatic},
    util::SpriteGridDesc,
};

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerStageLabels {
    UpdatePlayerState,
}

#[derive(Clone, Copy)]
struct PlayerFrameTurn {
    left: bool,
    right: bool,
}

const TURN_BUFFER_SIZE: usize = 3;

const OFFROAD_SHAKE_OFFSETS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

pub struct Player {
    turn_buffer: [PlayerFrameTurn; TURN_BUFFER_SIZE],

    is_braking: bool,

    offroad_shake_index: usize,
    offroad_shake_timer: Timer,

    racer_ent: Entity,

    tire_ent: Entity,
    brake_light_ent: Entity,
    sand_blast_ent: Entity,
}

impl Player {
    pub fn get_racer_ent(&self) -> Entity {
        self.racer_ent
    }
}

struct Tire {}

pub struct Racer {
    pub turn_rate: f32,
    pub speed: f32,
    lod_level: u8,
}

struct RacerOverlay {
    pub offset_cycle_pos: u8,
    pub sprite_cycle_pos: u8,

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
            sprite_cycle_length,
            offset_cycle_length,
            num_lod_levels,
            turnable,
            flippable,
            sprite_desc,
            offset_table,
        }
    }
}

struct OverlayOffsets([(i32, i32); NUM_TURN_LEVELS]);

const TIRE_OFFSETS: [OverlayOffsets; NUM_RACER_LODS * 2] = [
    // LOD level 0
    // Up cycle
    OverlayOffsets([(0, -16), (1, -16), (3, -17), (10, -19)]),
    // Down cycle
    OverlayOffsets([(0, -19), (2, -19), (6, -21), (12, -21)]),
    // LOD level 1
    // Up cycle
    OverlayOffsets([(0, -18), (0, -17), (3, -17), (8, -21)]),
    // Down cycle
    OverlayOffsets([(0, -21), (2, -22), (5, -22), (12, -24)]),
    // LOD level 2
    // Up cycle
    OverlayOffsets([(1, -20), (1, -21), (2, -22), (6, -22)]),
    // Down cycle
    OverlayOffsets([(1, -22), (2, -23), (3, -24), (9, -25)]),
    // LOD level 3
    // Up cycle
    OverlayOffsets([(1, -23), (-1, -23), (4, -24), (7, -24)]),
    OverlayOffsets([(1, -24), (0, -25), (5, -26), (9, -26)]),
];
fn make_tire_overlay(racer: Entity) -> RacerOverlay {
    RacerOverlay::new(racer, 2, 1, 4, true, true, &TIRE_SPRITE_DESC, &TIRE_OFFSETS)
}

// No cycle or LOD to worry about, unlike tires
const BRAKE_LIGHT_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -1), (-2, -2), (-4, -5), (0, -8)])];
fn make_brake_light_overlay(racer: Entity) -> RacerOverlay {
    RacerOverlay::new(
        racer,
        1,
        1,
        1,
        true,
        true,
        &BRAKE_LIGHT_SPRITE_DESC,
        &BRAKE_LIGHT_OFFSETS,
    )
}

const SAND_BLAST_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -16), (8, -16), (14, -16), (22, -16)])];
fn make_sand_blast_overlay(racer: Entity) -> RacerOverlay {
    RacerOverlay::new(
        racer,
        1,
        2,
        1,
        false,
        false,
        &SAND_BLAST_SPRITE_DESC,
        &SAND_BLAST_OFFSETS,
    )
}

const NUM_RACER_LODS: usize = 4;

const NUM_TURN_LEVELS: usize = 4;

const MAX_TURN_RATE: f32 = 400.0;

const RACER_MIN_SPEED: f32 = 1.4;
pub const RACER_MAX_NORMAL_SPEED: f32 = 9.0;
const RACER_MAX_TURBO_SPEED: f32 = 10.43;

// TODO: Instead scale acceleration by how close we are to max speed.
// Makes stopping less punishing while forcing a commitment to unlock turbo
const PLAYER_SPEED_MIN_ACCEL: f32 = 0.4;
const PLAYER_SPEED_MAX_ACCEL: f32 = 3.0;

const PLAYER_COAST_DRAG: f32 = 0.75;
const PLAYER_BRAKE_DRAG: f32 = 3.6;
const PLAYER_OFFROAD_DRAG: f32 = 1.8;

const PLAYER_TURN_ACCEL: f32 = 1200.0;
const PLAYER_TURN_FALLOFF: f32 = 1800.0;
const PLAYER_ROAD_CURVE_SCALAR: f32 = 60.0;

const BIKE_SPRITE_Z: f32 = 100.0;
const TIRE_SPRITE_Z: f32 = 100.1;
const BRAKE_LIGHT_SPRITE_Z: f32 = 100.1;
const SAND_BLAST_SPRITE_Z: f32 = 100.2;

const BIKE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
    rows: 4,
    columns: 6,
};
const TIRE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 16,
    rows: 4,
    columns: 4,
};
const BRAKE_LIGHT_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 16,
    rows: 1,
    columns: 4,
};
const SAND_BLAST_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 2,
};

const PLAYER_NOT_INIT: &str = "Player was not initialized";

pub fn startup_player(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let bike_tex = asset_server.load("textures/player_atlas.png");
    let bike_atlas = BIKE_SPRITE_DESC.make_atlas(bike_tex);
    let tire_tex = asset_server.load("textures/tire_atlas.png");
    let tire_atlas = TIRE_SPRITE_DESC.make_atlas(tire_tex);
    let brake_light_tex = asset_server.load("textures/brake_light_atlas.png");
    let brake_light_atlas = BRAKE_LIGHT_SPRITE_DESC.make_atlas(brake_light_tex);
    let sand_blast_tex = asset_server.load("textures/sand_blast_atlas.png");
    let sand_blast_atlas = SAND_BLAST_SPRITE_DESC.make_atlas(sand_blast_tex);

    let bike_xform = Transform::from_translation(Vec3::new(
        f32::conv(FIELD_WIDTH) * 0.5,
        f32::conv(BIKE_SPRITE_DESC.tile_size) * 0.5,
        BIKE_SPRITE_Z,
    ));

    let racer_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(bike_atlas),
            transform: bike_xform,
            ..Default::default()
        })
        .insert(Racer {
            lod_level: 0,
            turn_rate: 0.0,
            speed: RACER_MAX_NORMAL_SPEED,
        })
        .id();

    let tire_xform = Transform::from_translation(Vec3::new(0.0, 0.0, TIRE_SPRITE_Z));

    let tire_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(tire_atlas),
            transform: tire_xform,
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, false))
        .insert(make_tire_overlay(racer_ent))
        .insert(Tire {})
        .id();

    let brake_light_xform = Transform::from_translation(Vec3::new(0.0, 0.0, BRAKE_LIGHT_SPRITE_Z));
    let brake_light_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(brake_light_atlas),
            transform: brake_light_xform,
            ..Default::default()
        })
        .insert(make_brake_light_overlay(racer_ent))
        .id();

    let sand_blast_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(sand_blast_atlas),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, SAND_BLAST_SPRITE_Z)),
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(make_sand_blast_overlay(racer_ent))
        .id();

    commands
        .entity(racer_ent)
        .push_children(&[tire_ent, brake_light_ent, sand_blast_ent]);

    commands.insert_resource(Player {
        turn_buffer: [PlayerFrameTurn {
            left: false,
            right: false,
        }; TURN_BUFFER_SIZE],
        offroad_shake_timer: Timer::from_seconds(1.0 / 15.0, true),
        offroad_shake_index: 0,
        is_braking: false,
        racer_ent,
        tire_ent,
        brake_light_ent,
        sand_blast_ent,
    })
}

pub fn add_player_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        // .with_system(
        //     test_modify_player
        //         .system()
        //         .label(PlayerStageLabels::UpdatePlayerState),
        // )
        .with_system(
            update_player_state
                .system()
                .label(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_bike_sprites
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_tires
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_brake_lights
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_racer_offsets
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_sand_blasts
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
}

fn update_player_state(
    mut player: ResMut<Player>,
    input: Res<JoyrideInput>,
    mut racers: Query<(&mut Racer, &mut Transform)>,
    road_static: Res<RoadStatic>,
    mut road_dyn: ResMut<RoadDynamic>,
) {
    let (mut racer, mut xform) = racers.get_mut(player.racer_ent).expect(PLAYER_NOT_INIT);

    let next_turn = player.turn_buffer[0];
    player.turn_buffer.copy_within(1.., 0);
    player.turn_buffer[TURN_BUFFER_SIZE - 1] = PlayerFrameTurn {
        left: input.left.is_pressed(),
        right: input.right.is_pressed(),
    };

    let turn_accel = PLAYER_TURN_ACCEL * TIME_STEP;
    let turn_falloff = PLAYER_TURN_FALLOFF * TIME_STEP;

    // Increase steering to the left if the button is held, otherwise undo any left steering
    if next_turn.left {
        racer.turn_rate = f32::max(-MAX_TURN_RATE, racer.turn_rate - turn_accel);
    } else if racer.turn_rate < 0.0 {
        racer.turn_rate = f32::min(0.0, racer.turn_rate + turn_falloff)
    }

    // Same for the right
    if next_turn.right {
        racer.turn_rate = f32::min(MAX_TURN_RATE, racer.turn_rate + turn_accel);
    } else if racer.turn_rate > 0.0 {
        racer.turn_rate = f32::max(0.0, racer.turn_rate - turn_falloff);
    }

    let mut speed_change = 0.0;

    player.is_braking = input.brake.is_pressed();
    let is_accelerating = input.accel.is_pressed();
    if player.is_braking {
        speed_change -= PLAYER_BRAKE_DRAG;
    } else if is_accelerating {
        let accel_scale = f32::max(1.0 - (racer.speed / RACER_MAX_NORMAL_SPEED), 0.0);
        let accel = PLAYER_SPEED_MIN_ACCEL
            + ((PLAYER_SPEED_MAX_ACCEL - PLAYER_SPEED_MIN_ACCEL) * accel_scale);
        speed_change += accel;
    } else {
        speed_change -= PLAYER_COAST_DRAG;
    }

    let is_offroad = is_offroad(&road_static, &road_dyn);
    if is_offroad {
        speed_change -= PLAYER_OFFROAD_DRAG;
    }

    racer.speed = f32::clamp(
        // TODO: Applying delta time here may be a problem for boost end clamping
        racer.speed + (speed_change * TIME_STEP),
        RACER_MIN_SPEED,
        RACER_MAX_NORMAL_SPEED,
    );

    road_dyn.advance_z(racer.speed * TIME_STEP);

    let mut road_x = road_dyn.x_offset;

    road_x -= racer.turn_rate * TIME_STEP;

    // Apply the road's curvature against the player
    road_x += road_dyn.get_seg_curvature() * TIME_STEP * PLAYER_ROAD_CURVE_SCALAR * racer.speed;

    road_dyn.x_offset = f32::clamp(road_x, -500.0, 500.0);

    let xform_offset = if is_offroad {
        player
            .offroad_shake_timer
            .tick(Duration::from_secs_f32(TIME_STEP));
        if player.offroad_shake_timer.just_finished() {
            player.offroad_shake_index =
                (player.offroad_shake_index + 1) % OFFROAD_SHAKE_OFFSETS.len();
        }

        let offset = OFFROAD_SHAKE_OFFSETS[player.offroad_shake_index];
        (offset.0, offset.1)
    } else {
        (0.0, 0.0)
    };

    xform.translation.x = (f32::conv(FIELD_WIDTH) * 0.5) + xform_offset.0;
    xform.translation.y = (f32::conv(BIKE_SPRITE_DESC.tile_size) * 0.5) + xform_offset.1;
}

fn update_bike_sprites(
    player: Res<Player>,
    mut racer_query: Query<(&mut TextureAtlasSprite, &Racer)>,
) {
    let (mut sprite, racer) = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    let RacerSpriteParams {
        turn_idx: sprite_x,
        flip_x,
    } = get_turning_sprite_desc(racer.turn_rate);
    let sprite_y = if flip_x { 1 } else { 0 }; // TODO: Actually flip the sprite instead?
    sprite.index = BIKE_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
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

fn update_brake_lights(player: Res<Player>, mut query: Query<&mut Visible>) {
    let mut visible = query
        .get_mut(player.brake_light_ent)
        .expect(PLAYER_NOT_INIT);

    visible.is_visible = player.is_braking;
}

fn update_sand_blasts(
    player: Res<Player>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut query: Query<(&mut Visible, &mut Timer, &mut RacerOverlay)>,
) {
    let (mut visible, mut timer, mut overlay) =
        query.get_mut(player.sand_blast_ent).expect(PLAYER_NOT_INIT);

    let is_offroad = is_offroad(&road_static, &road_dyn);
    if is_offroad {
        timer.tick(Duration::from_secs_f32(TIME_STEP));
        if timer.just_finished() {
            overlay.sprite_cycle_pos = (overlay.sprite_cycle_pos + 1) % overlay.sprite_cycle_length
        }
    }

    visible.is_visible = is_offroad;
}

fn update_racer_offsets(
    mut overlay_query: Query<(&RacerOverlay, &mut TextureAtlasSprite, &mut Transform)>,
    racer_query: Query<&Racer>,
) {
    for (overlay, mut sprite, mut xform) in overlay_query.iter_mut() {
        let (turn_rate, lod_level) = racer_query
            .get(overlay.racer)
            .map_or((0.0, 0), |r| (r.turn_rate, r.lod_level));

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

fn test_modify_player(
    input: Res<JoyrideInput>,
    player: Res<Player>,
    mut racer_query: Query<&mut Racer>,
) {
    let mut racer = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    if input.left == JoyrideInputState::JustPressed {
        racer.turn_rate = f32::max(racer.turn_rate - MAX_TURN_RATE / 4.0, -MAX_TURN_RATE);
    }
    if input.right == JoyrideInputState::JustPressed {
        racer.turn_rate = f32::min(racer.turn_rate + MAX_TURN_RATE / 4.0, MAX_TURN_RATE);
    }
}

struct RacerSpriteParams {
    turn_idx: u32,
    flip_x: bool,
}

fn get_turning_sprite_desc(turn_rate: f32) -> RacerSpriteParams {
    let turn_div = turn_rate / (MAX_TURN_RATE / f32::conv(NUM_TURN_LEVELS));
    let turn_div_trunc = i32::conv_trunc(turn_div);
    let turn_idx = u32::min(3, u32::conv(turn_div_trunc.abs()));

    RacerSpriteParams {
        turn_idx,
        flip_x: turn_div_trunc >= 0,
    }
}

fn get_tire_cycle_seconds(speed: f32) -> f32 {
    f32::clamp((RACER_MAX_TURBO_SPEED / speed) / 16.0, 0.02, 4.0)
}
