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

pub fn spawn_empty_parent<'a, 'b>(
    commands: &'b mut Commands<'a>,
    position: Vec3,
) -> EntityCommands<'a, 'b> {
    // TODO: Add a component + system to hide children if this is invisible
    let mut ent_commands = commands.spawn();
    ent_commands
        .insert(Transform::from_translation(position))
        .insert(GlobalTransform::default());
    ent_commands
}
