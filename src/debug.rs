use bevy::{ecs::system::EntityCommands, prelude::*, utils::Instant};

use crate::{
    joyride::{JoyrideInput, JoyrideInputState},
    util::LocalVisible,
};

pub struct Systems {
    pub startup_debug: SystemSet,
    pub update_debug_vis: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_debug: SystemSet::new().with_system(startup_debug.system()),
            update_debug_vis: SystemSet::new().with_system(update_debug_vis.system()),
        }
    }
}

struct DebugCollision {}

pub struct DebugAssets {
    solid_color_mat: Handle<ColorMaterial>,
}

pub struct DebugConfig {
    pub debug_collision: bool,
    pub debug_road_seg_boundaries: bool,
    pub debug_gameplay: bool,
}

fn startup_debug(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.insert_resource(DebugAssets {
        solid_color_mat: materials.add(ColorMaterial {
            color: Color::Rgba {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.8,
            },
            texture: None,
        }),
    });
    commands.insert_resource(DebugConfig {
        debug_collision: false,
        debug_road_seg_boundaries: false,
        debug_gameplay: false,
    });
}

fn update_debug_vis(
    coll_query: Query<(&mut LocalVisible, With<DebugCollision>)>,
    mut debug_cfg: ResMut<DebugConfig>,
    input: Res<JoyrideInput>,
) {
    if input.debug == JoyrideInputState::JustPressed {
        debug_cfg.debug_collision = !debug_cfg.debug_collision;
    }

    coll_query.for_each_mut(|(mut local_vis, _)| {
        if local_vis.is_visible != debug_cfg.debug_collision {
            local_vis.is_visible = debug_cfg.debug_collision;
        }
    });
}

pub fn spawn_collision_debug_box(
    commands: &mut Commands,
    assets: &DebugAssets,
    offset: Vec2,
    size: Vec2,
) -> Entity {
    spawn_debug_box(commands, assets, offset, size)
        .insert(DebugCollision {})
        .id()
}

fn spawn_debug_box<'a, 'b>(
    commands: &'b mut Commands<'a>,
    assets: &DebugAssets,
    offset: Vec2,
    size: Vec2,
) -> EntityCommands<'a, 'b> {
    let mut ent_cmd = commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            size,
            ..Default::default()
        },
        material: assets.solid_color_mat.clone(),
        transform: Transform::from_translation(Vec3::new(offset.x, offset.y, 0.0)),
        ..Default::default()
    });

    ent_cmd.insert(LocalVisible::default());
    ent_cmd
}

pub struct LoopSectionTimer {
    start_time: Instant,
}

impl LoopSectionTimer {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[allow(dead_code)]
pub fn loop_section_timer_start(mut loop_section_timer: ResMut<LoopSectionTimer>) {
    loop_section_timer.start_time = Instant::now();
}

#[allow(dead_code)]
pub fn loop_section_timer_end(loop_section_timer: Res<LoopSectionTimer>) {
    let total_time = Instant::now().duration_since(loop_section_timer.start_time);
    let secs = total_time.as_secs_f64();
    println!("{}", secs);
}
