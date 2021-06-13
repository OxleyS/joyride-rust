use bevy::ecs::system::EntityCommands;
use bevy::prelude::TextureAtlas;

use bevy::prelude::*;
use easy_cast::*;

// Create a heap-stored array without allocating the array on the stack first (which could overflow it)
// Thanks to r/rust for this code
#[macro_export]
macro_rules! boxed_array {
    ($val:expr ; $len:expr) => {{
        // Use a generic function so that the pointer cast remains type-safe
        fn vec_to_boxed_array<T>(vec: Vec<T>) -> Box<[T; $len]> {
            // Creates a slice, but does not annotate it with its const size
            let boxed_slice = vec.into_boxed_slice();

            // Attach the size annotation by yoinking the pointer, casting, and re-boxing.
            // This does not incur any allocation or copying
            let ptr = ::std::boxed::Box::into_raw(boxed_slice) as *mut [T; $len];
            unsafe { Box::from_raw(ptr) }
        }

        vec_to_boxed_array(vec![$val; $len])
    }};
}

pub struct SpriteGridDesc {
    pub tile_size: u32,
    pub rows: u32,
    pub columns: u32,
}

impl SpriteGridDesc {
    pub fn get_sprite_index(&self, x: u32, y: u32) -> u32 {
        return (y * self.columns) + x;
    }

    pub fn make_atlas(&self, texture: Handle<Texture>) -> TextureAtlas {
        let tile_size = Vec2::new(self.tile_size.cast(), self.tile_size.cast());
        TextureAtlas::from_grid(texture, tile_size, self.columns.cast(), self.rows.cast())
    }
}

pub struct LocalVisible {
    pub is_visible: bool,
}

impl Default for LocalVisible {
    fn default() -> Self {
        Self { is_visible: true }
    }
}

pub fn spawn_empty_parent<'a, 'b>(
    commands: &'b mut Commands<'a>,
    position: Vec3,
) -> EntityCommands<'a, 'b> {
    let mut ent_commands = commands.spawn();
    ent_commands
        .insert(Transform::from_translation(position))
        .insert(GlobalTransform::default())
        .insert(LocalVisible::default());
    ent_commands
}

pub fn propagate_visibility_system(
    mut root_query: Query<
        (Entity, Option<&Children>, &LocalVisible, &mut Visible),
        Without<Parent>,
    >,
    changed_vis_query: Query<Entity, Changed<LocalVisible>>,
    mut visible_query: Query<(&LocalVisible, &mut Visible), With<Parent>>,
    children_query: Query<Option<&Children>, (With<Parent>, With<Visible>)>,
) {
    for (entity, children, local_vis, mut visible) in root_query.iter_mut() {
        let mut changed = false;
        if changed_vis_query.get(entity).is_ok() {
            visible.is_visible = local_vis.is_visible;
            changed = true;
        }

        if let Some(children) = children {
            for child in children.iter() {
                propagate_visibility_recursive(
                    visible.is_visible,
                    &changed_vis_query,
                    &mut visible_query,
                    &children_query,
                    *child,
                    changed,
                );
            }
        }
    }
}

fn propagate_visibility_recursive(
    is_parent_visible: bool,
    changed_vis_query: &Query<Entity, Changed<LocalVisible>>,
    visible_query: &mut Query<(&LocalVisible, &mut Visible), With<Parent>>,
    children_query: &Query<Option<&Children>, (With<Parent>, With<Visible>)>,
    entity: Entity,
    mut changed: bool,
) {
    changed |= changed_vis_query.get(entity).is_ok();

    let visible = if let Ok((local_vis, mut visible)) = visible_query.get_mut(entity) {
        if changed {
            visible.is_visible = is_parent_visible && local_vis.is_visible;
        }
        is_parent_visible && local_vis.is_visible
    } else {
        return;
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.iter() {
            propagate_visibility_recursive(
                visible,
                changed_vis_query,
                visible_query,
                children_query,
                *child,
                changed,
            );
        }
    }
}
