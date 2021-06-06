use std::time::Duration;

use bevy::prelude::*;
use easy_cast::*;

use crate::{
    joyride::{FIELD_HEIGHT, FIELD_WIDTH, TIME_STEP},
    player::{Player, Racer, RACER_MAX_NORMAL_SPEED},
    util::SpriteGridDesc,
};

struct SpeedText {
    ents: [Entity; 3],
    flash_timer: Timer,
    should_flash: bool,
}

const MAX_NORMAL_DISPLAY_SPEED: u32 = 280;

const TEXT_Z: f32 = 800.0;

const SMALL_NUM_WIDTH: f32 = 7.0;
const SMALL_NUM_SPRITE_DESC: SpriteGridDesc = SpriteGridDesc {
    tile_size: 32,
    rows: 1,
    columns: 10,
};

const TEXT_NOT_INIT: &str = "Text not initialized";

pub fn startup_speed_text(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let nums_tex = asset_server.load("textures/small_num_atlas.png");
    let nums_atlas = texture_atlases.add(SMALL_NUM_SPRITE_DESC.make_atlas(nums_tex));

    let base_pos = Vec2::new(
        f32::conv(FIELD_WIDTH) - 50.0,
        f32::conv(FIELD_HEIGHT) - 10.0,
    );

    // Placeholder value. Unfortunately, building by iterating over (0..3) loses the fixed size
    let mut ents = [Entity::new(0); 3];
    for (i, ent) in ents.iter_mut().enumerate() {
        let i: f32 = i.cast();
        let start: f32 = (SMALL_NUM_WIDTH * 0.5).floor();

        let t = Vec3::new(
            base_pos.x + start + (SMALL_NUM_WIDTH * i),
            base_pos.y,
            TEXT_Z,
        );

        *ent = commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: nums_atlas.clone(),
                transform: Transform::from_translation(t),
                ..Default::default()
            })
            .id()
    }

    commands.insert_resource(SpeedText {
        ents,
        flash_timer: Timer::from_seconds(1.0, true),
        should_flash: false,
    });
}

pub fn add_text_update_systems(system_set: SystemSet) -> SystemSet {
    system_set.with_system(update_speed_text.system())
}

fn update_speed_text(
    player: Res<Player>,
    racers: Query<&Racer>,
    mut speed_text: ResMut<SpeedText>,
    mut texts: Query<&mut TextureAtlasSprite>,
) {
    let speed = racers.get(player.get_racer_ent()).map_or(0.0, |r| r.speed);
    let speed_mph =
        u32::conv_nearest(speed * f32::conv(MAX_NORMAL_DISPLAY_SPEED) / RACER_MAX_NORMAL_SPEED);

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

    for (digit, ent) in digits.iter().zip(&speed_text.ents) {
        let mut sprite = texts.get_mut(*ent).expect(TEXT_NOT_INIT);
        sprite.index = *digit;
        sprite.color = color;
    }
}
