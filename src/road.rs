use crate::joyride::{FIELD_HEIGHT, FIELD_WIDTH};
use crate::{boxed_array, joyride};
use bevy::{
    core::AsBytes,
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use core::mem::size_of;
use easy_cast::*;
use lebe::Endian;

pub struct Systems {
    pub startup_road: SystemSet,
    pub update_road: SystemSet,
    pub draw_road: SystemSet,
    pub test_curve_road: SystemSet,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup_road: SystemSet::new().with_system(startup_road.system()),
            update_road: SystemSet::new()
                .with_system(update_road_curvature.system())
                .with_system(update_road_hills.system()),
            draw_road: SystemSet::new().with_system(render_road.system()),
            test_curve_road: SystemSet::new().with_system(test_curve_road.system()),
        }
    }
}

// Used for layering with other sprites
const ROAD_SPRITE_Z: f32 = 50.0;

// The number of pixel lines our coordinate maps stretch for, from the bottom of the screen
pub const ROAD_DISTANCE: usize = 110;

// Uphills move through the coordinate maps slower than one entry per pixel line.
// This specifies the maximum on-screen height the drawn road can be
const MAX_ROAD_DRAW_HEIGHT: usize = 170;

const NUM_ROAD_PIXELS: usize = (FIELD_WIDTH as usize) * MAX_ROAD_DRAW_HEIGHT;

// The distance from the bottom of the screen at which the road fully converges. Typically, when
// doing reverse projection, this is the center of the screen, but we fudge it for effect
const CONVERGE_DISTANCE: f32 = 113.4;

// How high the camera is off the ground. The higher this is, the faster Z increases every pixel line
const CAMERA_HEIGHT: f32 = 75.0;

// To better communicate movement, we switch road colors at every interval of Z
const COLOR_SWITCH_Z_INTERVAL: f32 = 0.5;

// The length (in Z) of a single road segment
const SEGMENT_LENGTH: f32 = 15.0;

const PAVEMENT_WIDTH: f32 = 125.0;
const CENTER_LINE_WIDTH: f32 = 2.0;
const RUMBLE_STRIP_WIDTH: f32 = 20.0;

const ROAD_NOT_INIT: &str = "Road was not initialized";

// Debug flags
const DEBUG_VIS_SEGMENTS: bool = true;

#[derive(Clone, Copy)]
struct QuadraticCoefficients {
    x2: f32,
    x: f32,
}

// The road warps for curves or hills according to quadratic functions, with the segment's curve/hill value as X
const CURVE_COEFF: QuadraticCoefficients = QuadraticCoefficients { x2: 1.0, x: 0.0 };
const HILL_COEFF: QuadraticCoefficients = QuadraticCoefficients { x2: 0.5, x: 0.5 };

#[derive(Clone, Copy)]
struct ShiftableColor(u32, u32);

struct RoadColors {
    offroad: ShiftableColor,
    rumble_strip: ShiftableColor,
    pavement: ShiftableColor,
    center_line: u32, // Shifts to match the pavement color
}

#[derive(Clone)]
struct RoadSegment {
    curve: f32,
    hill: f32,
}

pub struct RoadStatic {
    render_tex: Handle<Texture>,
    z_map: Box<[f32; ROAD_DISTANCE]>,
    scale_map: Box<[f32; ROAD_DISTANCE]>,
    colors: RoadColors,
    road_sprite: Entity,
}

// TODO: Can we encapsulate better?
pub struct RoadDynamic {
    // The height that this road will take up on-screen when drawn
    draw_height: usize,

    // Table of road X offsets. Affected by curvature
    x_map: Box<[f32; ROAD_DISTANCE]>,

    // Table that maps on-screen pixel lines to entries in the other tables
    // Affected by hills
    y_map: Box<[usize; MAX_ROAD_DRAW_HEIGHT]>,

    // The racer's offset from the center of the road
    pub x_offset: f32,

    // Used to shift colors during road drawing
    z_offset: f32,

    // The index of the segment the racer is currently in
    seg_idx: usize,

    // Their Z position within that segment
    seg_pos: f32,

    // TODO: Move to static once we read segs from file
    segs: Box<[RoadSegment]>,
}

impl RoadDynamic {
    pub fn advance_z(&mut self, advance_amount_z: f32) {
        assert!(advance_amount_z >= 0.0, "Can only move forward on the road");
        self.seg_pos += advance_amount_z;

        let num_advance_segs = (self.seg_pos / SEGMENT_LENGTH).floor();
        self.seg_idx += usize::conv_trunc(num_advance_segs);
        self.seg_pos -= num_advance_segs * SEGMENT_LENGTH;

        self.z_offset = (self.z_offset + advance_amount_z) % (COLOR_SWITCH_Z_INTERVAL * 2.0);
    }

    pub fn get_seg_curvature(&self, pos_offset: f32) -> f32 {
        let seg_idx =
            self.seg_idx + usize::conv_floor((self.seg_pos + pos_offset) / SEGMENT_LENGTH);
        get_bounded_seg(&self.segs, seg_idx).curve
    }

    pub fn get_draw_height_pixels(&self) -> usize {
        self.draw_height
    }
}

pub fn is_offroad(road_static: &RoadStatic, road_dyn: &RoadDynamic) -> bool {
    road_dyn.x_offset.abs() > (PAVEMENT_WIDTH + RUMBLE_STRIP_WIDTH) * road_static.scale_map[0]
}

pub struct DrawParams {
    pub scale: f32,
    pub draw_pos: (f32, f32),
}

pub fn get_draw_params_on_road(
    road_static: &RoadStatic,
    road_dyn: &RoadDynamic,
    x_pos: f32,
    z_pos: f32,
) -> Option<DrawParams> {
    let search_result_idx = road_static
        .z_map
        .binary_search_by(|z| z.partial_cmp(&z_pos).unwrap())
        .unwrap_or_else(|x| x);

    if search_result_idx == 0 || search_result_idx > ROAD_DISTANCE {
        return None;
    }

    let map_idx = search_result_idx - 1;
    let scale = road_static.scale_map[map_idx];

    let y_map_idx = {
        let result = road_dyn.y_map.binary_search(&map_idx).unwrap_or_else(|x| x);
        if result > 0 {
            result - 1
        } else {
            result
        }
    };

    if y_map_idx > road_dyn.draw_height {
        return None;
    }
    let x_offset = converge_x(x_pos, map_idx);

    Some(DrawParams {
        scale,
        draw_pos: ((road_dyn.x_map[map_idx] + x_offset), f32::conv(y_map_idx)),
    })
}

fn converge_x(x_pos: f32, road_map_idx: usize) -> f32 {
    let converge_scalar = f32::conv(road_map_idx) / f32::conv(ROAD_DISTANCE);
    x_pos * (1.0 - converge_scalar)
}

struct RoadDrawing {
    // Colors are expected to be RGBA
    draw_buffer: Box<[u32; NUM_ROAD_PIXELS]>,
}

impl Default for RoadDrawing {
    fn default() -> Self {
        Self {
            draw_buffer: boxed_array![0; NUM_ROAD_PIXELS],
        }
    }
}

fn startup_road(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let road_static = build_road_static(&mut commands, &mut textures, &mut materials);
    let road_dynamic = build_road_dynamic();

    commands.insert_resource(road_static);
    commands.insert_resource(road_dynamic);
}

fn build_road_static(
    commands: &mut Commands,
    textures: &mut ResMut<Assets<Texture>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) -> RoadStatic {
    // Create a texture that will be overwritten every frame
    let render_tex = Texture::new(
        Extent3d::new(FIELD_WIDTH.cast(), MAX_ROAD_DRAW_HEIGHT.cast(), 1),
        TextureDimension::D2,
        vec![0; NUM_ROAD_PIXELS * size_of::<u32>()],
        TextureFormat::Rgba8UnormSrgb,
    );
    let tex_handle = textures.add(render_tex);

    let mut z_map = boxed_array![0.0; ROAD_DISTANCE];
    let mut scale_map = boxed_array![0.0; ROAD_DISTANCE];

    let converge_y = f32::conv(FIELD_HEIGHT) - CONVERGE_DISTANCE;
    for (i, (out_z, out_scale)) in z_map.iter_mut().zip(scale_map.iter_mut()).enumerate() {
        // Calculate the screen-space Y coordinate of this line, with the converge distance as zero
        let screen_y = f32::conv(FIELD_HEIGHT) - f32::conv(i);

        // Reverse-projection to world-space to get the Z value at this line
        *out_z = CAMERA_HEIGHT / (screen_y - converge_y);

        // Precalculate the scale of objects (including the road itself) at this Z coordinate
        *out_scale = 1.0 / *out_z;
    }

    let colors = RoadColors {
        center_line: 0xFFFFFFFFu32,
        offroad: ShiftableColor(0xFFFF91FFu32, 0xDADA91FFu32),
        rumble_strip: ShiftableColor(0xFFFFFFFF, 0xFF0000FF),
        pavement: ShiftableColor(0x303030FF, 0x333333FF),
    };

    let mut xform = Transform::default();
    xform.translation = Vec3::new(
        (FIELD_WIDTH as f32) * 0.5,
        (MAX_ROAD_DRAW_HEIGHT as f32) * 0.5,
        ROAD_SPRITE_Z,
    );

    // Create a sprite to draw the road using the render texture
    let sprite = commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(tex_handle.clone().into()),
            transform: xform,
            ..Default::default()
        })
        .id();

    RoadStatic {
        z_map,
        scale_map,
        render_tex: tex_handle.clone(),
        colors,
        road_sprite: sprite,
    }
}

fn build_road_dynamic() -> RoadDynamic {
    let default_x = f32::conv(FIELD_WIDTH) * 0.5;

    let x_map = boxed_array![default_x; ROAD_DISTANCE];
    let y_map = boxed_array![0; MAX_ROAD_DRAW_HEIGHT];

    RoadDynamic {
        x_map,
        y_map,
        draw_height: ROAD_DISTANCE,
        x_offset: 0.0,
        z_offset: 0.0,
        seg_idx: 0,
        seg_pos: 0.0,
        segs: Box::new([
            RoadSegment {
                curve: 0.0,
                hill: 0.0,
            },
            RoadSegment {
                curve: 0.0,
                hill: 0.0,
            },
        ]),
    }
}

fn test_curve_road(mut road_dyn: ResMut<RoadDynamic>, input: Res<Input<KeyCode>>) {
    let curve_amt = joyride::TIME_STEP * 0.25;
    let hill_amt = joyride::TIME_STEP * 0.01;

    if input.pressed(KeyCode::A) {
        road_dyn.segs[0].curve -= curve_amt;
        road_dyn.segs[1].curve -= curve_amt;
    }
    if input.pressed(KeyCode::D) {
        road_dyn.segs[0].curve += curve_amt;
        road_dyn.segs[1].curve += curve_amt;
    }
    if input.pressed(KeyCode::J) {
        road_dyn.segs[1].curve -= curve_amt;
    }
    if input.pressed(KeyCode::L) {
        road_dyn.segs[1].curve += curve_amt;
    }
    if input.pressed(KeyCode::I) {
        road_dyn.segs[0].hill -= hill_amt;
        road_dyn.segs[1].hill -= hill_amt;
    }
    if input.pressed(KeyCode::K) {
        road_dyn.segs[0].hill += hill_amt;
        road_dyn.segs[1].hill += hill_amt;
    }
}

fn get_bounded_seg(segs: &[RoadSegment], idx: usize) -> RoadSegment {
    let actual_idx = usize::clamp(idx, 0, segs.len() - 1);
    return segs[actual_idx].clone();
}

fn map_road_quadratic<F: Fn(&RoadSegment) -> f32>(
    coeff: QuadraticCoefficients,
    initial_value: f32,
    seg_value_func: F,
    road_static: &RoadStatic,
    segments: &[RoadSegment],
    mut seg_idx: usize,
    mut seg_pos: f32,
    out_map: &mut [f32; ROAD_DISTANCE],
) {
    let mut cur_value = initial_value;
    let mut delta_value = 0.0;
    let mut last_z = road_static.z_map[0];
    let mut cur_seg = get_bounded_seg(&segments, seg_idx);

    for (out_value, cur_z) in out_map.iter_mut().zip(road_static.z_map.iter()) {
        let delta_z = cur_z - last_z;

        seg_pos += delta_z;
        if seg_pos > SEGMENT_LENGTH {
            seg_idx += 1;
            cur_seg = get_bounded_seg(&segments, seg_idx);
        }

        let parameter = seg_value_func(&cur_seg);

        delta_value += (parameter * coeff.x2) * delta_z;
        cur_value += delta_value;
        *out_value = cur_value + (parameter * coeff.x);

        last_z = *cur_z;
    }
}

fn update_road_curvature(road_static: Res<RoadStatic>, mut road_dyn: ResMut<RoadDynamic>) {
    // Convert ResMut to a regular mutable reference - otherwise Rust can't properly split borrows
    // between individual struct fields, and complains about multiple-borrow
    let road_dyn: &mut RoadDynamic = &mut road_dyn;

    map_road_quadratic(
        CURVE_COEFF,
        f32::conv(FIELD_WIDTH) * 0.5,
        |seg| seg.curve,
        &road_static,
        &road_dyn.segs,
        road_dyn.seg_idx,
        road_dyn.seg_pos,
        &mut road_dyn.x_map,
    );

    // Assuming no curvature, focus the far end of the road to the center of the screen.
    // This ensures the player is "looking down the road" at all times.
    for (i, x) in road_dyn.x_map.iter_mut().enumerate() {
        *x += converge_x(road_dyn.x_offset, i);
    }
}

struct HillScratchPad {
    y_advancement_map: Box<[f32; ROAD_DISTANCE]>,
}

impl Default for HillScratchPad {
    fn default() -> Self {
        Self {
            y_advancement_map: boxed_array!(1.0; ROAD_DISTANCE),
        }
    }
}

fn update_road_hills(
    road_static: Res<RoadStatic>,
    mut road_dyn: ResMut<RoadDynamic>,
    mut scratch_pad: Local<HillScratchPad>,
) {
    map_road_quadratic(
        HILL_COEFF,
        1.0,
        |seg| seg.hill,
        &road_static,
        &road_dyn.segs,
        road_dyn.seg_idx,
        road_dyn.seg_pos,
        &mut scratch_pad.y_advancement_map,
    );

    let mut draw_height = MAX_ROAD_DRAW_HEIGHT;
    let mut flt_map_idx: f32 = 0.0;
    for cur_line in 0..MAX_ROAD_DRAW_HEIGHT {
        let map_idx = usize::conv_trunc(flt_map_idx);
        if map_idx >= ROAD_DISTANCE {
            draw_height = cur_line;
            break;
        }
        road_dyn.y_map[cur_line] = map_idx;

        let advancement = f32::max(scratch_pad.y_advancement_map[map_idx], 0.00001); // Clamp to ensure we always advance in the tables when drawing
        flt_map_idx += advancement;
    }

    road_dyn.draw_height = draw_height;
    road_dyn.y_map[draw_height..MAX_ROAD_DRAW_HEIGHT].fill(ROAD_DISTANCE);
}

fn render_road(
    road_static: Res<RoadStatic>,
    road_dyn: Res<RoadDynamic>,
    mut road_draw: Local<RoadDrawing>,
    mut textures: ResMut<Assets<Texture>>,
) {
    let field_width: usize = FIELD_WIDTH.cast();
    let colors = &road_static.colors;

    // Draw line-by-line, starting from the bottom
    for cur_line in (0..MAX_ROAD_DRAW_HEIGHT).rev() {
        let map_idx: usize = road_dyn.y_map[(MAX_ROAD_DRAW_HEIGHT - 1) - cur_line];
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

        let is_seg_boundary = if DEBUG_VIS_SEGMENTS && map_idx > 0 {
            let seg_num = usize::conv_trunc((road_z + road_dyn.seg_pos) / SEGMENT_LENGTH);
            let last_seg_num = usize::conv_trunc(
                (road_static.z_map[map_idx - 1] + road_dyn.seg_pos) / SEGMENT_LENGTH,
            );
            seg_num != last_seg_num
        } else {
            false
        };

        // Switch the exact color used for each part of the road, based on Z
        let num_color_switches =
            i32::conv_trunc((road_z + road_dyn.z_offset) / COLOR_SWITCH_Z_INTERVAL);
        let shift_color = num_color_switches % 2 != 0;

        let road_center = road_dyn.x_map[map_idx];
        let road_width = PAVEMENT_WIDTH * road_scale;
        let center_line_width = CENTER_LINE_WIDTH * road_scale;
        let rumble_width = RUMBLE_STRIP_WIDTH * road_scale;

        // For every pixel in this line, from left to right
        for (x, px) in px_line.iter_mut().enumerate() {
            let x: f32 = x.cast();

            // Calculate the distance from the center of the road
            let distance_from_center = (x - road_center).abs();

            // Use that distance to determine the part of the road this pixel is on
            let shiftable: ShiftableColor = if distance_from_center <= center_line_width {
                ShiftableColor(colors.center_line, colors.pavement.1)
            } else if distance_from_center <= road_width {
                colors.pavement
            } else if distance_from_center <= road_width + rumble_width {
                colors.rumble_strip
            } else {
                colors.offroad
            };

            // Write the color
            let color = if is_seg_boundary {
                0x00FF00FF
            } else if shift_color {
                shiftable.1
            } else {
                shiftable.0
            };
            *px = color.from_current_into_big_endian();
        }
    }

    // Copy the pixel data to the texture
    let dest_tex = textures
        .get_mut(road_static.render_tex.clone())
        .expect(ROAD_NOT_INIT);
    dest_tex
        .data
        .copy_from_slice(road_draw.draw_buffer.as_bytes());
}
