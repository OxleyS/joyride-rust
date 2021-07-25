use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::util::LocalVisible;

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
    });
}

fn update_debug_vis(
    coll_query: Query<(&mut LocalVisible, With<DebugCollision>)>,
    debug_cfg: Res<DebugConfig>,
) {
    coll_query.for_each_mut(|(mut local_vis, _)| {
        local_vis.is_visible = debug_cfg.debug_collision;
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
