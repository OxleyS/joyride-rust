use crate::boxed_array;
use crate::joyride::{FIELD_HEIGHT, FIELD_WIDTH};
use bevy::{
    core::AsBytes,
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use core::mem::size_of;
use std::convert::TryInto;

const ROAD_WIDTH: usize = 640;
const MAX_ROAD_DRAW_HEIGHT: usize = 170;

const ROAD_NOT_INIT: &str = "Road was not initialized";

struct RoadStatic {
    render_tex: Handle<Texture>,
}

struct RoadDynamic {
    sprite: Entity,
}

struct RoadDrawing {
    draw_buffer: Box<[u32; ROAD_WIDTH * MAX_ROAD_DRAW_HEIGHT]>,
}

pub fn startup_road(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let render_tex = Texture::new(
        Extent3d::new(
            ROAD_WIDTH.try_into().unwrap(),
            MAX_ROAD_DRAW_HEIGHT.try_into().unwrap(),
            1,
        ),
        TextureDimension::D2,
        vec![0; ROAD_WIDTH * MAX_ROAD_DRAW_HEIGHT * size_of::<u32>()],
        TextureFormat::Rgba8UnormSrgb,
    );
    let tex_handle = textures.add(render_tex);

    commands.insert_resource(RoadStatic {
        render_tex: tex_handle.clone(),
    });

    let mut xform = Transform::default();
    xform.translation = Vec3::new(
        (FIELD_WIDTH as f32) * 0.5,
        (MAX_ROAD_DRAW_HEIGHT as f32) * 0.5,
        0.0,
    );

    let sprite = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(tex_handle.into()),
            transform: xform,
            ..Default::default()
        })
        .id();

    commands.insert_resource(RoadDynamic { sprite });
    commands.insert_resource(RoadDrawing {
        draw_buffer: boxed_array![0; ROAD_WIDTH * MAX_ROAD_DRAW_HEIGHT],
    });
}

pub fn add_road_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        .with_system(update_road.system())
        .with_system(render_road.system())
}

fn update_road(mut road_dyn: ResMut<RoadDynamic>) {}

fn render_road(
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut road_draw: ResMut<RoadDrawing>,
    mut textures: ResMut<Assets<Texture>>,
) {
    for px in road_draw.draw_buffer.iter_mut() {
        *px = 0xFFFF0000;
    }

    let dest_tex = textures
        .get_mut(road_static.render_tex.clone())
        .expect(ROAD_NOT_INIT);
    dest_tex
        .data
        .copy_from_slice(road_draw.draw_buffer.as_bytes());
}
