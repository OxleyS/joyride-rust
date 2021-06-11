use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{JoyrideGame, FIELD_HEIGHT, FIELD_WIDTH, TIME_STEP},
    player::{Player, PLAYER_MAX_NORMAL_SPEED},
    racer::Racer,
    util::SpriteGridDesc,
};

struct SpeedText {
    num_ents: [Entity; 3],
    flash_timer: Timer,
    should_flash: bool,

    km_ent: Entity,
    speed_ent: Entity,
}

struct TimeText {
    time_ent: Entity,
    num_ents: [Entity; 2],
}

const MAX_NORMAL_DISPLAY_SPEED: u32 = 280;

const TEXT_Z: f32 = 800.0;

const SMALL_NUM_WIDTH: f32 = 7.0;
const SMALL_NUM_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 10,
};

const LARGE_NUM_WIDTH: f32 = 8.0;
const LARGE_NUM_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 10,
};
const SMALL_TEXT_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 4,
};

const TEXT_NOT_INIT: &str = "Text not initialized";

pub fn startup_speed_text(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let small_nums_tex = asset_server.load("textures/small_num_atlas.png");
    let small_nums_atlas = texture_atlases.add(SMALL_NUM_SPRITE_DESC.make_atlas(small_nums_tex));
    let large_nums_tex = asset_server.load("textures/large_num_atlas.png");
    let large_nums_atlas = texture_atlases.add(LARGE_NUM_SPRITE_DESC.make_atlas(large_nums_tex));
    let small_texts_tex = asset_server.load("textures/small_text_atlas.png");
    let small_texts_atlas = texture_atlases.add(SMALL_TEXT_SPRITE_DESC.make_atlas(small_texts_tex));

    let field_width = f32::conv(FIELD_WIDTH);
    let field_height = f32::conv(FIELD_HEIGHT);

    let base_pos = Vec2::new(field_width - 48.0, field_height - 10.0);

    // Placeholder value. Unfortunately, building by iterating over (0..3) loses the fixed size
    let mut speed_num_ents = [Entity::new(0); 3];
    for (i, ent) in speed_num_ents.iter_mut().enumerate() {
        let i: f32 = i.cast();
        let start: f32 = (SMALL_NUM_WIDTH * 0.5).floor();

        let t = Vec3::new(
            base_pos.x + start + (SMALL_NUM_WIDTH * i),
            base_pos.y,
            TEXT_Z,
        );

        *ent = commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: small_nums_atlas.clone(),
                transform: Transform::from_translation(t),
                ..Default::default()
            })
            .id()
    }

    let km_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: small_texts_atlas.clone(),
            sprite: TextureAtlasSprite {
                color: Color::YELLOW,
                index: 0,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(
                field_width - 16.0,
                field_height - 10.0,
                TEXT_Z,
            )),
            ..Default::default()
        })
        .id();

    let speed_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: small_texts_atlas.clone(),
            sprite: TextureAtlasSprite {
                color: Color::YELLOW,
                index: 1,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(
                field_width - 72.0,
                field_height - 10.0,
                TEXT_Z,
            )),
            ..Default::default()
        })
        .id();

    let time_ent = commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: small_texts_atlas,
            sprite: TextureAtlasSprite {
                color: Color::YELLOW,
                index: 2,
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(
                field_width * 0.5,
                field_height - 10.0,
                TEXT_Z,
            )),
            ..Default::default()
        })
        .id();

    let time_num_ents: [Entity; 2] = [
        commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: large_nums_atlas.clone(),
                transform: Transform::from_translation(Vec3::new(
                    (field_width * 0.5) - LARGE_NUM_WIDTH * 0.5,
                    field_height - 30.0,
                    TEXT_Z,
                )),
                ..Default::default()
            })
            .id(),
        commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: large_nums_atlas.clone(),
                transform: Transform::from_translation(Vec3::new(
                    (field_width * 0.5) + LARGE_NUM_WIDTH * 0.5,
                    field_height - 30.0,
                    TEXT_Z,
                )),
                ..Default::default()
            })
            .id(),
    ];

    commands.insert_resource(SpeedText {
        num_ents: speed_num_ents,
        flash_timer: Timer::from_seconds(1.0, true),
        should_flash: false,
        km_ent,
        speed_ent,
    });

    commands.insert_resource(TimeText {
        time_ent,
        num_ents: time_num_ents,
    })
}

pub fn add_text_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        .with_system(update_speed_text.system())
        .with_system(update_time_text.system())
}

fn update_speed_text(
    player: Res<Player>,
    racers: Query<&Racer>,
    mut speed_text: ResMut<SpeedText>,
    mut texts: Query<&mut TextureAtlasSprite>,
) {
    let speed = racers.get(player.get_racer_ent()).map_or(0.0, |r| r.speed);
    let speed_mph =
        u32::conv_nearest(speed * f32::conv(MAX_NORMAL_DISPLAY_SPEED) / PLAYER_MAX_NORMAL_SPEED);

    let digits: [u32; 3] = if speed_mph <= 999 {
        [speed_mph / 100, (speed_mph / 10) % 10, speed_mph % 10]
    } else {
        [9, 9, 9]
    };

    if speed_mph >= MAX_NORMAL_DISPLAY_SPEED {
        speed_text.flash_timer.unpause();
    } else {
        speed_text.should_flash = true;
        speed_text.flash_timer.pause();
        speed_text.flash_timer.reset();
    }

    if speed_text
        .flash_timer
        .tick(Duration::from_secs_f32(TIME_STEP))
        .just_finished()
    {
        speed_text.should_flash = !speed_text.should_flash;
    }

    let color = if speed_text.should_flash && speed_mph >= MAX_NORMAL_DISPLAY_SPEED {
        Color::RED
    } else {
        Color::WHITE
    };

    for (digit, ent) in digits.iter().zip(&speed_text.num_ents) {
        let mut sprite = texts.get_mut(*ent).expect(TEXT_NOT_INIT);
        sprite.index = *digit;
        sprite.color = color;
    }
}

fn update_time_text(
    game: Res<JoyrideGame>,
    time_text: Res<TimeText>,
    mut texts: Query<&mut TextureAtlasSprite>,
) {
    let rem_seconds =
        game.remaining_time.duration().as_secs_f32() - game.remaining_time.elapsed_secs();

    let rem_seconds: u32 = u32::clamp(rem_seconds.cast_floor(), 0, 99);
    let digits: [u32; 2] = [(rem_seconds / 10), (rem_seconds % 10)];

    for (digit, ent) in digits.iter().zip(&time_text.num_ents) {
        let mut sprite = texts.get_mut(*ent).expect(TEXT_NOT_INIT);
        sprite.index = *digit;
    }
}
