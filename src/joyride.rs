use bevy::prelude::*;

pub const FIELD_WIDTH: u32 = 320;
pub const FIELD_HEIGHT: u32 = 240;

pub struct JoyrideGame {}

pub fn startup_joyride(mut commands: Commands) {
    commands.insert_resource(JoyrideGame {});

    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scaling_mode = bevy::render::camera::ScalingMode::None;
    camera.orthographic_projection.left = 0.0;
    camera.orthographic_projection.top = FIELD_HEIGHT as f32;
    camera.orthographic_projection.right = FIELD_WIDTH as f32;
    camera.orthographic_projection.bottom = 0.0;
    commands.spawn_bundle(camera);
}
