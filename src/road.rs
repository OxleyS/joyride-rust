use crate::boxed_array;
use crate::joyride::{FIELD_HEIGHT, FIELD_WIDTH};
use bevy::{
    core::AsBytes,
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use core::mem::size_of;
use easy_cast::*;

const ROAD_DISTANCE: usize = 110;
const MAX_ROAD_DRAW_HEIGHT: usize = 170;
const NUM_ROAD_PIXELS: usize = (FIELD_WIDTH as usize) * MAX_ROAD_DRAW_HEIGHT;

const ROAD_CONVERGE_ADJUST: f32 = 1.075;
const CAMERA_HEIGHT: f32 = 75.0;
const COLOR_SWITCH_Z_INTERVAL: f32 = 0.4;
const PAVEMENT_WIDTH: f32 = 204.0;
const CENTER_LINE_WIDTH: f32 = 4.0;
const RUMBLE_STRIP_WIDTH: f32 = 40.0;

const ROAD_NOT_INIT: &str = "Road was not initialized";

#[derive(Clone, Copy)]
struct ShiftableColor(u32, u32);

struct RoadColors {
    offroad: ShiftableColor,
    rumble_strip: ShiftableColor,
    pavement: ShiftableColor,
    center_line: u32, // Shifts to match the pavement color
}

struct RoadStatic {
    render_tex: Handle<Texture>,
    z_map: Box<[f32; ROAD_DISTANCE]>,
    scale_map: Box<[f32; ROAD_DISTANCE]>,
    colors: RoadColors,
}

struct RoadDynamic {
    x_map: Box<[f32; ROAD_DISTANCE]>,
    y_map: Box<[f32; ROAD_DISTANCE]>,
    sprite: Entity,
    z_offset: f32,
}

struct RoadDrawing {
    draw_buffer: Box<[u32; NUM_ROAD_PIXELS]>,
}

pub fn startup_road(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let road_static = build_road_static(&mut textures);
    let road_dynamic = build_road_dynamic(
        &mut commands,
        road_static.render_tex.clone(),
        &mut materials,
    );

    commands.insert_resource(road_static);
    commands.insert_resource(road_dynamic);
    commands.insert_resource(RoadDrawing {
        draw_buffer: boxed_array![0; NUM_ROAD_PIXELS],
    });
}

pub fn add_road_update_systems(system_set: SystemSet) -> SystemSet {
    system_set
        .with_system(update_road.system())
        .with_system(render_road.system())
}

fn build_road_static(textures: &mut ResMut<Assets<Texture>>) -> RoadStatic {
    let render_tex = Texture::new(
        Extent3d::new(FIELD_WIDTH.cast(), MAX_ROAD_DRAW_HEIGHT.cast(), 1),
        TextureDimension::D2,
        vec![0; NUM_ROAD_PIXELS * size_of::<u32>()],
        TextureFormat::Rgba8UnormSrgb,
    );
    let tex_handle = textures.add(render_tex);

    let mut z_map = boxed_array![0.0; ROAD_DISTANCE];
    let mut scale_map = boxed_array![0.0; ROAD_DISTANCE];
    let flt_road_distance: f32 = ROAD_DISTANCE.cast();

    let half_field_height = f32::conv(FIELD_HEIGHT) * 0.5;
    for (i, (out_z, out_scale)) in z_map.iter_mut().zip(scale_map.iter_mut()).enumerate() {
        let i = f32::conv(i);
        let screen_y = f32::conv(FIELD_HEIGHT) - i;

        *out_z = CAMERA_HEIGHT / (screen_y - half_field_height);
        *out_scale = 1.0 / *out_z;
    }

    println!(
        "{} {} {}",
        z_map[0],
        scale_map[0],
        scale_map[scale_map.len() - 1]
    );

    // let mut scale_adjust = 1.0;
    // let mut slope = scale_map[0] - scale_map[1];
    // slope *= ROAD_CONVERGE_ADJUST;
    // for out_scale in scale_map.iter_mut() {
    //     *out_scale = scale_adjust;
    //     scale_adjust -= slope;
    // }

    let colors = RoadColors {
        center_line: 0xFFFFFFFFu32,
        offroad: ShiftableColor(0xFF91FFFFu32, 0xFF91DADAu32),
        rumble_strip: ShiftableColor(0xFFFFFFFF, 0xFF0000FF),
        pavement: ShiftableColor(0xFF333333, 0xFF333333), // TODO: Actual shift?
    };

    RoadStatic {
        z_map,
        scale_map,
        render_tex: tex_handle.clone(),
        colors,
    }
}

fn build_road_dynamic(
    commands: &mut Commands,
    render_tex: Handle<Texture>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) -> RoadDynamic {
    let mut xform = Transform::default();
    xform.translation = Vec3::new(
        (FIELD_WIDTH as f32) * 0.5,
        (MAX_ROAD_DRAW_HEIGHT as f32) * 0.5,
        0.0,
    );

    let sprite = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(render_tex.into()),
            transform: xform,
            ..Default::default()
        })
        .id();

    let default_x = f32::conv(FIELD_WIDTH) * 0.5;

    let x_map = boxed_array![default_x; ROAD_DISTANCE];
    let y_map = boxed_array![0.0; ROAD_DISTANCE];

    RoadDynamic {
        x_map,
        y_map,
        sprite,
        z_offset: 0.0,
    }
}

fn update_road(mut road_dyn: ResMut<RoadDynamic>, time: Res<Time>) {
    //road_dyn.z_offset =
    //(road_dyn.z_offset + time.delta_seconds()) % (COLOR_SWITCH_Z_INTERVAL * 2.0);
}

fn render_road(
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut road_draw: ResMut<RoadDrawing>,
    mut textures: ResMut<Assets<Texture>>,
) {
    let mut flt_map_idx: f32 = 0.0;
    let z_offset = road_dyn.z_offset;
    let field_width: usize = FIELD_WIDTH.cast();
    let colors = &road_static.colors;

    for cur_line in (0..MAX_ROAD_DRAW_HEIGHT).rev() {
        let map_idx: usize = flt_map_idx.cast_trunc();
        let px_line = road_draw
            .draw_buffer
            .get_mut((cur_line * field_width)..((cur_line + 1) * field_width))
            .unwrap();

        // Make any pixels we won't draw to transparent
        let no_draw = map_idx >= ROAD_DISTANCE;
        if no_draw {
            for px in px_line {
                *px = 0;
            }
            continue;
        }

        let road_z = road_static.z_map[map_idx];
        let road_scale = road_static.scale_map[map_idx];

        let num_color_switches = i32::conv_trunc((road_z + z_offset) / COLOR_SWITCH_Z_INTERVAL);
        let shift_color = num_color_switches % 2 != 0;

        let road_center = road_dyn.x_map[map_idx];
        let road_width = PAVEMENT_WIDTH * road_scale;
        let center_line_width = CENTER_LINE_WIDTH * road_scale;
        let rumble_width = RUMBLE_STRIP_WIDTH * road_scale;

        for (x, px) in px_line.iter_mut().enumerate() {
            let x: f32 = x.cast();
            let x_offset = (x - road_center).abs();

            let shiftable: ShiftableColor = if x_offset <= center_line_width {
                ShiftableColor(colors.center_line, colors.pavement.1)
            } else if x_offset <= road_width {
                colors.pavement
            } else if x_offset <= road_width + rumble_width {
                colors.rumble_strip
            } else {
                colors.offroad
            };

            let color = if shift_color {
                shiftable.1
            } else {
                shiftable.0
            };
            *px = color;
        }

        flt_map_idx += 1.0;
    }

    let dest_tex = textures
        .get_mut(road_static.render_tex.clone())
        .expect(ROAD_NOT_INIT);
    dest_tex
        .data
        .copy_from_slice(road_draw.draw_buffer.as_bytes());
}
