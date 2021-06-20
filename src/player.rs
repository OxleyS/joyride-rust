use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{JoyrideInput, JoyrideInputState, FIELD_WIDTH, TIME_STEP},
    racer::{
        get_turning_sprite_desc, make_racer, OverlayOffsets, Racer, RacerAssets, RacerOverlay,
        RacerSpriteParams, MAX_TURN_RATE, RACER_MAX_SPEED, RACER_ROAD_CURVE_SCALAR,
    },
    road::{is_offroad, RoadDynamic, RoadStatic},
    util::{LocalVisible, SpriteGridDesc},
};

#[derive(Clone, Copy)]
struct PlayerFrameTurn {
    left: bool,
    right: bool,
}

const TURN_BUFFER_SIZE: usize = 3;

const OFFROAD_SHAKE_OFFSETS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

pub struct Player {
    turn_buffer: [PlayerFrameTurn; TURN_BUFFER_SIZE],

    offroad_shake_index: usize,
    offroad_shake_timer: Timer,

    racer_ent: Entity,

    brake_light_ent: Entity,
    sand_blast_ent: Entity,
}

impl Player {
    pub fn get_racer_ent(&self) -> Entity {
        self.racer_ent
    }
}

// No cycle or LOD to worry about, unlike tires
const BRAKE_LIGHT_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -1), (2, -2), (4, -5), (0, -8)])];
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

const SAND_BLAST_OFFSETS: [OverlayOffsets; 1] = [OverlayOffsets([
    (0, -16),
    (-8, -16),
    (-14, -16),
    (-22, -16),
])];
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

const PLAYER_MIN_SPEED: f32 = 1.4;
pub const PLAYER_MAX_NORMAL_SPEED: f32 = 9.0;
const PLAYER_MAX_TURBO_SPEED: f32 = RACER_MAX_SPEED;

// TODO: Instead scale acceleration by how close we are to max speed.
// Makes stopping less punishing while forcing a commitment to unlock turbo
const PLAYER_SPEED_MIN_ACCEL: f32 = 0.4;
const PLAYER_SPEED_MAX_ACCEL: f32 = 3.0;

const PLAYER_COAST_DRAG: f32 = 0.75;
const PLAYER_BRAKE_DRAG: f32 = 3.6;
const PLAYER_OFFROAD_DRAG: f32 = 1.8;

const PLAYER_TURN_ACCEL: f32 = 1200.0;
const PLAYER_TURN_FALLOFF: f32 = 1800.0;

const BRAKE_LIGHT_OFFSET_Z: f32 = 0.1;
const SAND_BLAST_OFFSET_Z: f32 = 0.2;

const PLAYER_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
    rows: 3,
    columns: 6,
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

pub struct Systems {
    pub startup_player: SystemSet,
    pub update_player_driving: SystemSet,
    pub update_player_road_position: SystemSet,
    pub update_player_visuals: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_player: SystemSet::new().with_system(startup_player.system()),
            update_player_driving: SystemSet::new()
                .with_system(update_player_turning.system())
                .with_system(update_player_speed.system()),
            update_player_road_position: SystemSet::new()
                .with_system(update_player_road_position.system()),
            update_player_visuals: SystemSet::new()
                .with_system(update_player_shake.system())
                .with_system(update_player_bike_sprites.system())
                .with_system(update_brake_lights.system())
                .with_system(update_sand_blasts.system()),
        }
    }
}

fn startup_player(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    racer_assets: Res<RacerAssets>,
    asset_server: Res<AssetServer>,
) {
    let bike_tex = asset_server.load("textures/player_atlas.png");
    let bike_atlas = PLAYER_SPRITE_DESC.make_atlas(bike_tex);
    let brake_light_tex = asset_server.load("textures/brake_light_atlas.png");
    let brake_light_atlas = BRAKE_LIGHT_SPRITE_DESC.make_atlas(brake_light_tex);
    let sand_blast_tex = asset_server.load("textures/sand_blast_atlas.png");
    let sand_blast_atlas = SAND_BLAST_SPRITE_DESC.make_atlas(sand_blast_tex);

    let racer_ent = make_racer(
        &mut commands,
        racer_assets,
        texture_atlases.add(bike_atlas),
        0.5,
    );

    let brake_light_xform = Transform::from_translation(Vec3::new(0.0, 0.0, BRAKE_LIGHT_OFFSET_Z));
    let brake_light_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(brake_light_atlas),
            transform: brake_light_xform,
            ..Default::default()
        })
        .insert(make_brake_light_overlay(racer_ent))
        .insert(LocalVisible::default())
        .id();

    let sand_blast_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(sand_blast_atlas),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, SAND_BLAST_OFFSET_Z)),
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(make_sand_blast_overlay(racer_ent))
        .insert(LocalVisible::default())
        .id();

    commands
        .entity(racer_ent)
        .push_children(&[brake_light_ent, sand_blast_ent]);

    commands.insert_resource(Player {
        turn_buffer: [PlayerFrameTurn {
            left: false,
            right: false,
        }; TURN_BUFFER_SIZE],
        offroad_shake_timer: Timer::from_seconds(1.0 / 15.0, true),
        offroad_shake_index: 0,
        racer_ent,
        brake_light_ent,
        sand_blast_ent,
    })
}

fn update_player_turning(
    mut player: ResMut<Player>,
    input: Res<JoyrideInput>,
    mut racers: Query<&mut Racer>,
) {
    let mut racer = racers.get_mut(player.racer_ent).expect(PLAYER_NOT_INIT);

    // TODO: This buffering algorithm will change turn mechanics based on framerate. Use a time-based buffer instead
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
}

fn update_player_speed(
    input: Res<JoyrideInput>,
    player: Res<Player>,
    mut racers: Query<&mut Racer>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    let mut racer = racers.get_mut(player.racer_ent).expect(PLAYER_NOT_INIT);
    let mut speed_change = 0.0;

    let is_braking = input.brake.is_pressed();
    let is_accelerating = input.accel.is_pressed();

    if is_braking {
        speed_change -= PLAYER_BRAKE_DRAG;
    } else if is_accelerating {
        let accel_scale = f32::max(1.0 - (racer.speed / PLAYER_MAX_NORMAL_SPEED), 0.0);
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
        PLAYER_MIN_SPEED,
        PLAYER_MAX_NORMAL_SPEED,
    );
}

fn update_player_road_position(
    player: Res<Player>,
    racers: Query<&Racer>,
    mut road_dyn: ResMut<RoadDynamic>,
) {
    let racer = racers.get(player.racer_ent).expect(PLAYER_NOT_INIT);
    road_dyn.advance_z(racer.speed * TIME_STEP);

    let mut road_x = road_dyn.x_offset;
    road_x -= racer.turn_rate * TIME_STEP;

    // Apply the road's curvature against the player
    road_x += road_dyn.get_seg_curvature(0.0) * TIME_STEP * RACER_ROAD_CURVE_SCALAR * racer.speed;
    road_dyn.x_offset = f32::clamp(road_x, -500.0, 500.0);
}

fn update_player_shake(
    mut player: ResMut<Player>,
    mut xforms: Query<&mut Transform>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    let mut xform = xforms.get_mut(player.racer_ent).expect(PLAYER_NOT_INIT);

    let xform_offset = if is_offroad(&road_static, &road_dyn) {
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
    xform.translation.y = (f32::conv(PLAYER_SPRITE_DESC.tile_size) * 0.5) + xform_offset.1;
}

fn update_player_bike_sprites(
    player: Res<Player>,
    mut racer_query: Query<(&mut TextureAtlasSprite, &Racer)>,
) {
    let (mut sprite, racer) = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    // The player's sprite sheet is laid out differently than other racers, missing a lot
    if racer.lod_level == 0 {
        let RacerSpriteParams {
            turn_idx: sprite_x,
            flip_x,
        } = get_turning_sprite_desc(racer.turn_rate);

        let sprite_y = 0;
        sprite.index = PLAYER_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
        sprite.flip_x = flip_x;
    } else {
        let sprite_x = racer.lod_level.cast();
        let sprite_y = 1;
        sprite.index = PLAYER_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
        sprite.flip_x = false;
    }
}

fn update_brake_lights(
    player: Res<Player>,
    input: Res<JoyrideInput>,
    mut query: Query<&mut RacerOverlay>,
) {
    let mut overlay = query
        .get_mut(player.brake_light_ent)
        .expect(PLAYER_NOT_INIT);

    overlay.is_visible = input.brake.is_pressed();
}

fn update_sand_blasts(
    player: Res<Player>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut query: Query<(&mut Timer, &mut RacerOverlay)>,
) {
    let (mut timer, mut overlay) = query.get_mut(player.sand_blast_ent).expect(PLAYER_NOT_INIT);

    let is_offroad = is_offroad(&road_static, &road_dyn);
    if is_offroad {
        timer.tick(Duration::from_secs_f32(TIME_STEP));
        if timer.just_finished() {
            overlay.sprite_cycle_pos =
                (overlay.sprite_cycle_pos + 1) % overlay.get_sprite_cycle_length()
        }
    }

    overlay.is_visible = is_offroad;
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
