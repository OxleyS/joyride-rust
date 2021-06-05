use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{self, JoyrideInput, JoyrideInputState, FIELD_WIDTH, TIME_STEP},
    road::RoadDynamic,
    util::SpriteGridDesc,
};

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerStageLabels {
    UpdatePlayerState,
}

struct Player {
    is_braking: bool,
    racer_ent: Entity,

    tire_ent: Entity,
    brake_light_ent: Entity,
}

struct Racer {
    turn_rate: f32,
    speed: f32,
    lod_level: u8,
}

struct RacerOverlay {
    pub offset_cycle_pos: u8,

    racer: Entity,

    offset_cycle_length: u8,
    num_lod_levels: u8,
    sprite_desc: &'static SpriteGridDesc,

    // Laid out as [[OverlayOffsets; offset_cycle_length]; num_lod_levels;], except continuously
    offset_table: &'static [OverlayOffsets],
}

impl RacerOverlay {
    pub fn new(
        racer: Entity,
        offset_cycle_length: u8,
        num_lod_levels: u8,
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
        assert!(
            sprite_desc.columns as usize >= NUM_TURN_LEVELS,
            "Sprite grid not wide enough for all turn levels"
        );
        assert!(
            sprite_desc.rows >= num_lod_levels as u32,
            "Sprite grid not tall enough for all LOD levels"
        );
        Self {
            racer,
            offset_cycle_pos: 0,
            offset_cycle_length,
            num_lod_levels,
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
    RacerOverlay::new(racer, 2, 4, &TIRE_SPRITE_DESC, &TIRE_OFFSETS)
}

// No cycle or LOD to worry about, unlike tires
const BRAKE_LIGHT_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, -1), (-2, -2), (-4, -5), (0, -8)])];
fn make_brake_light_overlay(racer: Entity) -> RacerOverlay {
    RacerOverlay::new(racer, 1, 1, &BRAKE_LIGHT_SPRITE_DESC, &BRAKE_LIGHT_OFFSETS)
}

const NUM_RACER_LODS: usize = 4;

const NUM_TURN_LEVELS: usize = 4;

const MAX_TURN_RATE: f32 = 10.0;

const RACER_MIN_SPEED: f32 = 1.4;
const RACER_MAX_NORMAL_SPEED: f32 = 9.0;
const RACER_MAX_TURBO_SPEED: f32 = 10.43;

const BIKE_SPRITE_Z: f32 = 3.0;
const TIRE_SPRITE_Z: f32 = 3.1;
const BRAKE_LIGHT_SPRITE_Z: f32 = 3.1;

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
            speed: RACER_MIN_SPEED,
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

    commands
        .entity(racer_ent)
        .push_children(&[tire_ent, brake_light_ent]);

    commands.insert_resource(Player {
        is_braking: false,
        racer_ent,
        tire_ent,
        brake_light_ent,
    })
}

pub fn add_player_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        .with_system(
            test_modify_player
                .system()
                .label(PlayerStageLabels::UpdatePlayerState),
        )
        .with_system(
            update_player_state
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
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
            advance_player_on_road
                .system()
                .after(PlayerStageLabels::UpdatePlayerState),
        )
}

fn update_player_state(mut player: ResMut<Player>, input: Res<JoyrideInput>) {
    player.is_braking = input.brake == JoyrideInputState::Pressed;
}

fn advance_player_on_road(
    player: Res<Player>,
    mut road: ResMut<RoadDynamic>,
    racers: Query<&Racer>,
) {
    let racer = racers.get(player.racer_ent).expect(PLAYER_NOT_INIT);
    road.advance_z(racer.speed * joyride::TIME_STEP);
}

fn update_bike_sprites(
    player: Res<Player>,
    mut racer_query: Query<(&mut TextureAtlasSprite, &Racer)>,
) {
    let (mut sprite, racer) = racer_query
        .get_mut(player.racer_ent)
        .expect(PLAYER_NOT_INIT);

    let RacerSpriteParams { sprite_x, flip_x } = get_turning_sprite_desc(racer.turn_rate);
    let sprite_y = if flip_x { 1 } else { 0 }; // TODO: Actually flip the sprite instead?
    sprite.index = BIKE_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
}

fn update_tires(
    mut overlay_query: Query<(&mut RacerOverlay, &mut Timer)>,
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

fn update_racer_offsets(
    mut overlay_query: Query<(&RacerOverlay, &mut TextureAtlasSprite, &mut Transform)>,
    racer_query: Query<&Racer>,
) {
    for (overlay, mut sprite, mut xform) in overlay_query.iter_mut() {
        let (turn_rate, lod_level) = racer_query
            .get(overlay.racer)
            .map_or((0.0, 0), |r| (r.turn_rate, r.lod_level));

        let RacerSpriteParams { sprite_x, flip_x } = get_turning_sprite_desc(turn_rate);

        let lod_idx = u8::min(lod_level, overlay.num_lod_levels - 1);
        let offsets_idx = (overlay.offset_cycle_length * lod_idx) + overlay.offset_cycle_pos;

        let offsets = &overlay.offset_table[offsets_idx as usize];
        let mut turn_level_offset = offsets.0[sprite_x as usize];

        if flip_x {
            turn_level_offset.0 = -turn_level_offset.0;
        }
        sprite.flip_x = flip_x;

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
    sprite_x: u32,
    flip_x: bool,
}

fn get_turning_sprite_desc(turn_rate: f32) -> RacerSpriteParams {
    let turn_div = turn_rate / (MAX_TURN_RATE / f32::conv(NUM_TURN_LEVELS));
    let turn_div_trunc = i32::conv_trunc(turn_div);
    let sprite_x = u32::min(3, u32::conv(turn_div_trunc.abs()));

    RacerSpriteParams {
        sprite_x,
        flip_x: turn_div_trunc >= 0,
    }
}

fn get_tire_cycle_seconds(speed: f32) -> f32 {
    f32::clamp((RACER_MAX_TURBO_SPEED / speed) / 16.0, 0.02, 4.0)
}
