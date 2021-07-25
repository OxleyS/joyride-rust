use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    debug::{spawn_collision_debug_box, DebugAssets},
    joyride::{JoyrideInput, JoyrideInputState, FIELD_WIDTH, TIME_STEP},
    racer::{
        get_turning_sprite_desc, make_racer, OverlayOffsets, Racer, RacerAssets, RacerOverlay,
        RacerSpriteParams, Tire, MAX_TURN_RATE, RACER_MAX_SPEED,
    },
    road::{is_offroad, RoadDynamic, RoadStatic},
    road_object::{PLAYER_COLLISION_WIDTH, ROAD_OBJ_BASE_Z},
    util::{LocalVisible, SpriteGridDesc},
};

#[derive(Clone, Copy)]
struct PlayerFrameTurn {
    left: bool,
    right: bool,
}

struct PlayerSlide {
    direction: PlayerSlideDirection,
    timer: Timer,
}

struct PlayerCrash {
    sprite_cycle_timer: Option<Timer>,
    sprite_cycle_idx: u32,

    resetting: bool,
    pre_reset_timer: Timer,
}

impl PlayerCrash {
    fn next_sprite_cycle_time(speed: f32) -> f32 {
        if speed > 3.0 {
            1.0 / 30.0
        } else if speed > 1.2 {
            2.0 / 30.0
        } else {
            4.0 / 30.0
        }
    }
}

enum PlayerControlLoss {
    Slide(PlayerSlide),
    Crash(PlayerCrash),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerSlideDirection {
    Left,
    Right,
}

const TURN_BUFFER_SIZE: usize = 3;

const OFFROAD_SHAKE_OFFSETS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

pub struct Player {
    turn_buffer: [PlayerFrameTurn; TURN_BUFFER_SIZE],

    offroad_shake_index: usize,
    offroad_shake_timer: Timer,

    control_loss: Option<PlayerControlLoss>,

    racer_ent: Entity,

    brake_light_ent: Entity,
    sand_blast_ent: Entity,
    smoke_ent: Entity,
    turbo_flare_ent: Entity,
}

impl Player {
    pub fn get_racer_ent(&self) -> Entity {
        self.racer_ent
    }

    pub fn crash(&mut self) {
        match self.control_loss {
            // Don't override an existing crash, it will reset sprite cycles and stuff
            Some(PlayerControlLoss::Crash(_)) => return,
            _ => {
                self.control_loss = Some(PlayerControlLoss::Crash(PlayerCrash {
                    resetting: false,
                    pre_reset_timer: Timer::from_seconds(1.0, false),
                    sprite_cycle_idx: 0,
                    sprite_cycle_timer: None,
                }));
            }
        }
    }

    pub fn slide(&mut self, direction: PlayerSlideDirection) {
        match self.control_loss {
            // Slides do not override a crash
            Some(PlayerControlLoss::Crash(_)) => return,
            _ => {
                self.control_loss = Some(PlayerControlLoss::Slide(PlayerSlide {
                    direction,
                    timer: Timer::from_seconds(PLAYER_SLIDE_DURATION, false),
                }));
            }
        }
    }

    fn is_crashing(&self) -> bool {
        match &self.control_loss {
            Some(PlayerControlLoss::Crash(_)) => true,
            _ => false,
        }
    }

    fn reset_turn_buffer(&mut self) {
        for b in self.turn_buffer.as_mut() {
            b.left = false;
            b.right = false;
        }
    }
}

// No cycle or LOD to worry about, unlike tires
const BRAKE_LIGHT_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -1), (2, -2), (4, -5), (0, -8)])];
fn make_brake_light_overlay() -> RacerOverlay {
    RacerOverlay::new(
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
fn make_sand_blast_overlay() -> RacerOverlay {
    RacerOverlay::new(
        1,
        2,
        1,
        false,
        false,
        &SAND_BLAST_SPRITE_DESC,
        &SAND_BLAST_OFFSETS,
    )
}

const SMOKE_OFFSETS: [OverlayOffsets; 1] = [OverlayOffsets([
    (0, -16),
    (-8, -16),
    (-14, -16),
    (-22, -16),
])];
fn make_smoke_overlay() -> RacerOverlay {
    RacerOverlay::new(1, 2, 1, false, false, &SMOKE_SPRITE_DESC, &SMOKE_OFFSETS)
}

const TURBO_FLARE_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -6), (2, -7), (2, -9), (-1, -10)])];
fn make_turbo_flare_overlay() -> RacerOverlay {
    RacerOverlay::new(
        1,
        1,
        1,
        true,
        true,
        &TURBO_FLARE_SPRITE_DESC,
        &TURBO_FLARE_OFFSETS,
    )
}

const PLAYER_MIN_SPEED: f32 = 1.4;
pub const PLAYER_MAX_NORMAL_SPEED: f32 = 9.0;
const PLAYER_MAX_TURBO_SPEED: f32 = RACER_MAX_SPEED;

const PLAYER_SPEED_MIN_ACCEL: f32 = 0.4;
const PLAYER_SPEED_MAX_ACCEL: f32 = 3.0;
const PLAYER_SPEED_TURBO_ACCEL: f32 = 0.75;

const PLAYER_COAST_DRAG: f32 = 0.75;
const PLAYER_BRAKE_DRAG: f32 = 3.6;
const PLAYER_OFFROAD_DRAG: f32 = 1.8;
const PLAYER_CRASH_DRAG: f32 = 3.0;

const PLAYER_TURN_ACCEL: f32 = 1200.0;
const PLAYER_TURN_FALLOFF: f32 = 1800.0;

const PLAYER_CRASH_RESET_SPEED: f32 = 300.0;
const PLAYER_SLIDE_DURATION: f32 = 2.0 / 3.0;
const PLAYER_SLIDE_STRENGTH: f32 = 300.0;

const BRAKE_LIGHT_OFFSET_Z: f32 = 0.1;
const TURBO_FLARE_OFFSET_Z: f32 = 0.15;
const SAND_BLAST_OFFSET_Z: f32 = 0.2;
const SMOKE_OFFSET_Z: f32 = 0.2;

const PLAYER_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 64,
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
const SMOKE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 2,
};
const TURBO_FLARE_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 4,
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
                .with_system(update_player_speed.system())
                .with_system(update_player_crash.system())
                .with_system(test_modify_player.system()),
            update_player_road_position: SystemSet::new()
                .with_system(update_player_road_position.system()),
            update_player_visuals: SystemSet::new()
                .with_system(update_player_shake.system())
                .with_system(update_player_bike_sprites.system())
                .with_system(update_brake_lights.system())
                .with_system(update_sand_blasts.system())
                .with_system(update_turbo_flare.system())
                .with_system(update_smoke.system()),
        }
    }
}

fn startup_player(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    racer_assets: Res<RacerAssets>,
    asset_server: Res<AssetServer>,
    debug_assets: Res<DebugAssets>,
) {
    let bike_tex = asset_server.load("textures/player_atlas.png");
    let bike_atlas = PLAYER_SPRITE_DESC.make_atlas(bike_tex);
    let brake_light_tex = asset_server.load("textures/brake_light_atlas.png");
    let brake_light_atlas = BRAKE_LIGHT_SPRITE_DESC.make_atlas(brake_light_tex);
    let sand_blast_tex = asset_server.load("textures/sand_blast_atlas.png");
    let sand_blast_atlas = SAND_BLAST_SPRITE_DESC.make_atlas(sand_blast_tex);
    let turbo_flare_tex = asset_server.load("textures/turbo_flare_atlas.png");
    let turbo_flare_atlas = TURBO_FLARE_SPRITE_DESC.make_atlas(turbo_flare_tex);
    let smoke_tex = asset_server.load("textures/smoke_atlas.png");
    let smoke_atlas = SMOKE_SPRITE_DESC.make_atlas(smoke_tex);

    let racer_ent = make_racer(
        &mut commands,
        racer_assets,
        texture_atlases.add(bike_atlas),
        0.0,
        Vec3::new(0.0, 0.0, ROAD_OBJ_BASE_Z - 0.5),
    );

    let brake_light_xform = Transform::from_translation(Vec3::new(0.0, 0.0, BRAKE_LIGHT_OFFSET_Z));
    let brake_light_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(brake_light_atlas),
            transform: brake_light_xform,
            ..Default::default()
        })
        .insert(make_brake_light_overlay())
        .insert(LocalVisible::default())
        .id();

    let sand_blast_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(sand_blast_atlas),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, SAND_BLAST_OFFSET_Z)),
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(make_sand_blast_overlay())
        .insert(LocalVisible::default())
        .id();

    let smoke_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(smoke_atlas),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, SMOKE_OFFSET_Z)),
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true))
        .insert(make_smoke_overlay())
        .insert(LocalVisible::default())
        .id();

    let turbo_flare_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(turbo_flare_atlas),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, TURBO_FLARE_OFFSET_Z)),
            ..Default::default()
        })
        .insert(Timer::from_seconds(TIME_STEP, true))
        .insert(make_turbo_flare_overlay())
        .insert(LocalVisible::default())
        .id();

    let debug_box = spawn_collision_debug_box(
        &mut commands,
        &debug_assets,
        Vec2::new(0.0, -f32::conv(PLAYER_SPRITE_DESC.tile_size) * 0.5),
        Vec2::new(PLAYER_COLLISION_WIDTH, 1.0),
    );

    commands.entity(racer_ent).push_children(&[
        brake_light_ent,
        sand_blast_ent,
        smoke_ent,
        turbo_flare_ent,
        debug_box,
    ]);

    commands.insert_resource(Player {
        turn_buffer: [PlayerFrameTurn {
            left: false,
            right: false,
        }; TURN_BUFFER_SIZE],
        offroad_shake_timer: Timer::from_seconds(1.0 / 15.0, true),
        offroad_shake_index: 0,
        control_loss: None,
        racer_ent,
        brake_light_ent,
        sand_blast_ent,
        smoke_ent,
        turbo_flare_ent,
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

    match player.control_loss.as_mut() {
        Some(PlayerControlLoss::Slide(slide)) => {
            racer.turn_rate = if slide.direction == PlayerSlideDirection::Left {
                PLAYER_SLIDE_STRENGTH
            } else {
                -PLAYER_SLIDE_STRENGTH
            };

            if slide
                .timer
                .tick(Duration::from_secs_f32(TIME_STEP))
                .just_finished()
            {
                player.control_loss = None;
                racer.turn_rate = 0.0;
                player.reset_turn_buffer();
            }
        }
        Some(PlayerControlLoss::Crash(_)) => {
            racer.turn_rate = 0.0;
        }
        _ => {}
    };
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
    let is_turboing = input.turbo.is_pressed() && racer.speed >= PLAYER_MAX_NORMAL_SPEED;
    let is_crashing = player.is_crashing();

    if player.control_loss.is_some() {
        speed_change -= if is_crashing {
            PLAYER_CRASH_DRAG
        } else {
            PLAYER_COAST_DRAG
        };
    } else if is_braking {
        speed_change -= PLAYER_BRAKE_DRAG;
    } else if is_turboing {
        speed_change += PLAYER_SPEED_TURBO_ACCEL;
    } else if racer.speed > PLAYER_MAX_NORMAL_SPEED {
        let to_normal_cap = (racer.speed - PLAYER_MAX_NORMAL_SPEED) / TIME_STEP;
        speed_change -= f32::min(PLAYER_COAST_DRAG * 2.0, to_normal_cap);
    } else if is_accelerating {
        let accel_scale = f32::max(1.0 - (racer.speed / PLAYER_MAX_NORMAL_SPEED), 0.0);
        let accel = PLAYER_SPEED_MIN_ACCEL
            + ((PLAYER_SPEED_MAX_ACCEL - PLAYER_SPEED_MIN_ACCEL) * accel_scale);

        let accel_cap = f32::max((PLAYER_MAX_NORMAL_SPEED - racer.speed) / TIME_STEP, 0.0);
        speed_change += f32::min(accel, accel_cap);
    } else {
        speed_change -= PLAYER_COAST_DRAG;
    }

    let is_offroad = is_offroad(&road_static, &road_dyn);
    if is_offroad {
        speed_change -= PLAYER_OFFROAD_DRAG;
    }

    racer.speed = f32::clamp(
        racer.speed + (speed_change * TIME_STEP),
        if is_crashing { 0.0 } else { PLAYER_MIN_SPEED },
        PLAYER_MAX_TURBO_SPEED,
    );
}

fn update_player_road_position(
    player: Res<Player>,
    racers: Query<&Racer>,
    mut road_dyn: ResMut<RoadDynamic>,
) {
    let racer = racers.get(player.racer_ent).expect(PLAYER_NOT_INIT);
    road_dyn.advance_z(racer.speed * TIME_STEP);

    let is_sliding = match &player.control_loss {
        Some(PlayerControlLoss::Slide(_)) => true,
        _ => false,
    };

    let turn_rate = if is_sliding {
        -racer.turn_rate
    } else {
        racer.turn_rate
    };
    let mut road_x = road_dyn.x_offset;
    road_x -= turn_rate * TIME_STEP;

    // Apply the road's curvature against the player
    road_x += road_dyn.get_road_x_pull(0.0, racer.speed) * TIME_STEP;
    road_dyn.x_offset = f32::clamp(road_x, -500.0, 500.0);
}

fn update_player_shake(
    mut player: ResMut<Player>,
    mut xforms: Query<&mut Transform>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
) {
    let mut xform = xforms.get_mut(player.racer_ent).expect(PLAYER_NOT_INIT);

    let xform_offset = if is_offroad(&road_static, &road_dyn) && !player.is_crashing() {
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
    mut tire_query: Query<(&mut RacerOverlay, With<Tire>)>,
) {
    let (mut sprite, racer) = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    let mut tire_visible = true;

    match player.control_loss.as_ref() {
        Some(PlayerControlLoss::Crash(crash)) => {
            tire_visible = false;
            sprite.index = PLAYER_SPRITE_DESC.get_sprite_index(crash.sprite_cycle_idx, 3);
            sprite.flip_x = false;
        }
        _ => {
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
    };

    if let Ok((mut tire_overlay, _)) = tire_query.get_mut(racer.tire_ent) {
        tire_overlay.is_visible = tire_visible;
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

    overlay.is_visible = !player.is_crashing() && input.brake.is_pressed();
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

    overlay.is_visible = !player.is_crashing() && is_offroad;
}

fn update_smoke(
    player: Res<Player>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut overlay_query: Query<(&mut Timer, &mut RacerOverlay)>,
) {
    let (mut timer, mut overlay) = overlay_query
        .get_mut(player.smoke_ent)
        .expect(PLAYER_NOT_INIT);

    let is_sliding = match &player.control_loss {
        Some(PlayerControlLoss::Slide(_)) => true,
        _ => false,
    };

    let is_active = is_sliding && !is_offroad(&road_static, &road_dyn);
    if is_active {
        timer.tick(Duration::from_secs_f32(TIME_STEP));
        if timer.just_finished() {
            overlay.sprite_cycle_pos =
                (overlay.sprite_cycle_pos + 1) % overlay.get_sprite_cycle_length()
        }
    }

    overlay.is_visible = is_active;
}

fn update_turbo_flare(
    player: Res<Player>,
    input: Res<JoyrideInput>,
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut overlay_query: Query<(&mut Timer, &mut RacerOverlay)>,
    racer_query: Query<&Racer>,
) {
    let (mut timer, mut overlay) = overlay_query
        .get_mut(player.turbo_flare_ent)
        .expect(PLAYER_NOT_INIT);
    let racer = racer_query.get(player.racer_ent).expect(PLAYER_NOT_INIT);

    if is_offroad(&road_static, &road_dyn)
        || !input.turbo.is_pressed()
        || racer.speed <= PLAYER_MAX_NORMAL_SPEED
        || player.is_crashing()
    {
        overlay.is_visible = false;
        return;
    }

    timer.tick(Duration::from_secs_f32(TIME_STEP));
    if timer.just_finished() {
        overlay.is_visible = !overlay.is_visible;
        overlay.sprite_cycle_pos =
            (overlay.sprite_cycle_pos + 1) % overlay.get_sprite_cycle_length()
    }
}

fn update_player_crash(
    mut player: ResMut<Player>,
    mut racer_query: Query<(&mut Racer, &mut LocalVisible)>,
    mut road_dyn: ResMut<RoadDynamic>,
) {
    let player: &mut Player = &mut player;

    let crash = match player.control_loss.as_mut() {
        Some(PlayerControlLoss::Crash(crash)) => crash,
        _ => return,
    };

    let (mut racer, mut visible) = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);
    let tick_duration = Duration::from_secs_f32(TIME_STEP);

    if crash.resetting {
        let remaining = road_dyn.x_offset / TIME_STEP;
        let mut is_visible = false;

        if remaining <= PLAYER_CRASH_RESET_SPEED {
            road_dyn.x_offset = 0.0;
            player.control_loss = None;
            racer.speed = PLAYER_MIN_SPEED;
            is_visible = true;
            player.reset_turn_buffer();
        } else {
            road_dyn.x_offset -= PLAYER_CRASH_RESET_SPEED * TIME_STEP;
        }

        if visible.is_visible != is_visible {
            visible.is_visible = is_visible;
        }
    } else {
        if racer.speed <= 0.0 {
            crash.sprite_cycle_idx = 2;
            crash.pre_reset_timer.tick(tick_duration);
            if crash.pre_reset_timer.just_finished() {
                crash.resetting = true;
            }
        } else {
            //let timer: &mut Timer =
            let next_cycle_time =
                Duration::from_secs_f32(PlayerCrash::next_sprite_cycle_time(racer.speed));
            let cycle_timer = crash
                .sprite_cycle_timer
                .get_or_insert(Timer::new(next_cycle_time, false));

            cycle_timer.tick(tick_duration);
            if cycle_timer.just_finished() {
                crash.sprite_cycle_idx = (crash.sprite_cycle_idx + 1) % 4;
                cycle_timer.set_duration(next_cycle_time);
                cycle_timer.reset();
            }
        }
    }
}

fn test_modify_player(
    input: Res<JoyrideInput>,
    mut player: ResMut<Player>,
    mut racer_query: Query<&mut Racer>,
) {
    let mut racer = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    //if input.debug == JoyrideInputState::JustPressed {
    // player.control_loss = Some(PlayerControlLoss::Slide(PlayerSlide {
    //     direction: PlayerSlideDirection::Right,
    //     timer: Timer::from_seconds(PLAYER_SLIDE_DURATION, false),
    // }));
    //player.crash();
    //}

    // if input.left == JoyrideInputState::JustPressed {
    //     racer.turn_rate = f32::max(racer.turn_rate - MAX_TURN_RATE / 4.0, -MAX_TURN_RATE);
    // }
    // if input.right == JoyrideInputState::JustPressed {
    //     racer.turn_rate = f32::min(racer.turn_rate + MAX_TURN_RATE / 4.0, MAX_TURN_RATE);
    // }
}
