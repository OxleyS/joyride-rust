use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{JoyrideInput, JoyrideInputState, FIELD_WIDTH, TIME_STEP},
    util::SpriteGridDesc,
};

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerStageLabels {
    UpdatePlayerState,
}

struct Player {
    turn_rate: f32,
    is_braking: bool,
    speed: f32,
    bike_ent: Entity,
    lod_level: u8,

    tire_ent: Entity,
    brake_light_ent: Entity,
}

// struct Racer {
//     turn_rate: f32,
//     speed: f32,
//     lod_level: u8,
// }

struct RacerOverlay {
    pub offset_cycle_pos: u8,

    offset_cycle_length: u8,
    num_lod_levels: u8,
    sprite_desc: &'static SpriteGridDesc,

    // Laid out as [[OverlayOffsets; offset_cycle_length]; num_lod_levels;], except continuously
    offset_table: &'static [OverlayOffsets],
}

impl RacerOverlay {
    pub fn new(
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
    OverlayOffsets([(0, 8), (1, 8), (3, 7), (10, 5)]),
    // Down cycle
    OverlayOffsets([(0, 5), (2, 5), (6, 3), (12, 3)]),
    // LOD level 1
    // Up cycle
    OverlayOffsets([(0, 6), (0, 5), (3, 5), (8, 3)]),
    // Down cycle
    OverlayOffsets([(0, 3), (2, 2), (5, 2), (12, 0)]),
    // LOD level 2
    // Up cycle
    OverlayOffsets([(1, 4), (1, 3), (2, 2), (6, 2)]),
    // Down cycle
    OverlayOffsets([(1, 2), (2, 1), (3, 0), (9, -1)]),
    // LOD level 3
    // Up cycle
    OverlayOffsets([(1, 1), (-1, 1), (4, 0), (7, 0)]),
    OverlayOffsets([(1, 0), (0, -1), (5, -2), (9, -2)]),
];
fn make_tire_overlay() -> RacerOverlay {
    RacerOverlay::new(2, 4, &TIRE_SPRITE_DESC, &TIRE_OFFSETS)
}

// No cycle or LOD to worry about, unlike tires
const BRAKE_LIGHT_OFFSETS: [OverlayOffsets; 1] =
    [OverlayOffsets([(0, 23), (-2, 22), (-4, 19), (0, 16)])];
fn make_brake_light_overlay() -> RacerOverlay {
    RacerOverlay::new(1, 1, &BRAKE_LIGHT_SPRITE_DESC, &BRAKE_LIGHT_OFFSETS)
}

const NUM_RACER_LODS: usize = 4;

const NUM_TURN_LEVELS: usize = 4;

const MAX_TURN_RATE: f32 = 10.0;
const MAX_TURBO_SPEED: f32 = 8.11;

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

    let bike_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(bike_atlas),
            transform: bike_xform,
            ..Default::default()
        })
        .id();

    let tire_xform = Transform::from_translation(Vec3::new(
        f32::conv(FIELD_WIDTH) * 0.5,
        f32::conv(TIRE_SPRITE_DESC.tile_size) * 0.5,
        TIRE_SPRITE_Z,
    ));

    let tire_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(tire_atlas),
            transform: tire_xform,
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, false))
        .insert(make_tire_overlay())
        .id();

    let brake_light_xform = Transform::from_translation(Vec3::new(
        f32::conv(FIELD_WIDTH) * 0.5,
        f32::conv(BRAKE_LIGHT_SPRITE_DESC.tile_size) * 0.5,
        BRAKE_LIGHT_SPRITE_Z,
    ));
    let brake_light_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlases.add(brake_light_atlas),
            transform: brake_light_xform,
            ..Default::default()
        })
        .insert(make_brake_light_overlay())
        .id();

    commands.insert_resource(Player {
        turn_rate: 0.0,
        is_braking: false,
        lod_level: 0,
        speed: 0.0,
        bike_ent,
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
}

fn update_player_state(mut player: ResMut<Player>, input: Res<JoyrideInput>) {
    player.is_braking = input.brake == JoyrideInputState::Pressed;
}

fn update_bike_sprites(
    player: ResMut<Player>,
    mut query: Query<(&mut Transform, &mut TextureAtlasSprite)>,
) {
    let RacerSpriteParams { sprite_x, flip_x } = get_turning_sprite_desc(player.turn_rate);

    let (_, mut sprite) = query.get_mut(player.bike_ent).expect(PLAYER_NOT_INIT);
    let sprite_y = if flip_x { 1 } else { 0 }; // TODO: Actually flip the sprite instead?
    sprite.index = BIKE_SPRITE_DESC.get_sprite_index(sprite_x, sprite_y);
}

fn update_tires(player: Res<Player>, mut query: Query<(&mut RacerOverlay, &mut Timer)>) {
    let (mut overlay, mut timer) = query.get_mut(player.tire_ent).expect(PLAYER_NOT_INIT);

    timer.tick(Duration::from_secs_f32(TIME_STEP));
    if timer.finished() {
        overlay.offset_cycle_pos = (overlay.offset_cycle_pos + 1) % overlay.offset_cycle_length;

        let new_secs = get_tire_cycle_seconds(player.speed);
        timer.set_duration(Duration::from_secs_f32(new_secs));
        timer.reset();
    }
}

fn update_brake_lights(player: Res<Player>, mut query: Query<&mut Visible>) {
    let mut visible = query
        .get_mut(player.brake_light_ent)
        .expect(PLAYER_NOT_INIT);

    visible.is_visible = player.is_braking;
}

fn update_racer_offsets(
    player: Res<Player>,
    mut query: Query<(&RacerOverlay, &mut TextureAtlasSprite, &mut Transform)>,
) {
    let RacerSpriteParams { sprite_x, flip_x } = get_turning_sprite_desc(player.turn_rate);
    for (overlay, mut sprite, mut xform) in query.iter_mut() {
        let lod_idx = u8::min(player.lod_level, overlay.num_lod_levels - 1);
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

        xform.translation.x = (f32::conv(FIELD_WIDTH) * 0.5) + f32::conv(turn_level_offset.0);
        xform.translation.y =
            (f32::conv(TIRE_SPRITE_DESC.tile_size) * 0.5) + f32::conv(turn_level_offset.1);
    }
}

fn test_modify_player(input: Res<JoyrideInput>, mut player: ResMut<Player>) {
    if input.left == JoyrideInputState::JustPressed {
        player.turn_rate = f32::max(player.turn_rate - MAX_TURN_RATE / 4.0, -MAX_TURN_RATE);
    }
    if input.right == JoyrideInputState::JustPressed {
        player.turn_rate = f32::min(player.turn_rate + MAX_TURN_RATE / 4.0, MAX_TURN_RATE);
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
    f32::clamp((MAX_TURBO_SPEED / speed) / 16.0, 0.02, 4.0)
}
