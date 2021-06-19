use bevy::{input::InputSystem, prelude::*};

pub const FIELD_WIDTH: u32 = 320;
pub const FIELD_HEIGHT: u32 = 240;

// We lock the framerate, since this is a retro-style game, after all
pub const TIME_STEP: f32 = 1.0 / 30.0;

pub struct JoyrideGame {
    pub remaining_time: Timer,
}

#[derive(PartialEq, Eq)]
pub enum JoyrideInputState {
    JustPressed,
    Pressed,
    JustReleased,
    Released,
}

impl Default for JoyrideInputState {
    fn default() -> Self {
        Self::Released
    }
}

impl JoyrideInputState {
    pub fn is_pressed(&self) -> bool {
        *self == JoyrideInputState::JustPressed || *self == JoyrideInputState::Pressed
    }
}

#[derive(Default, PartialEq, Eq)]
pub struct JoyrideInput {
    pub left: JoyrideInputState,
    pub right: JoyrideInputState,
    pub up: JoyrideInputState,
    pub down: JoyrideInputState,
    pub accel: JoyrideInputState,
    pub brake: JoyrideInputState,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum InputStageLabels {
    UpdateInput,
}

fn startup_joyride(mut commands: Commands) {
    commands.insert_resource(JoyrideGame {
        remaining_time: Timer::from_seconds(100.0, false),
    });
    commands.insert_resource(JoyrideInput::default());

    let mut camera = OrthographicCameraBundle::new_2d();
    camera.orthographic_projection.scaling_mode = bevy::render::camera::ScalingMode::None;
    camera.orthographic_projection.left = 0.0;
    camera.orthographic_projection.top = FIELD_HEIGHT as f32;
    camera.orthographic_projection.right = FIELD_WIDTH as f32;
    camera.orthographic_projection.bottom = 0.0;
    commands.spawn_bundle(camera);
}

pub fn build_app(app: &mut AppBuilder) {
    app.add_startup_system(startup_joyride.system());

    // TODO: Probably not needed now that we changed how fixed framerate works
    app.add_system_to_stage(
        CoreStage::PreUpdate,
        update_input
            .system()
            .after(InputSystem)
            .label(InputStageLabels::UpdateInput),
    );
}

fn update_input(input: Res<Input<KeyCode>>, mut input_state: ResMut<JoyrideInput>) {
    update_input_state(&mut input_state.left, input.pressed(KeyCode::Left));
    update_input_state(&mut input_state.right, input.pressed(KeyCode::Right));
    update_input_state(&mut input_state.up, input.pressed(KeyCode::Up));
    update_input_state(&mut input_state.down, input.pressed(KeyCode::Down));
    update_input_state(&mut input_state.accel, input.pressed(KeyCode::Z));
    update_input_state(&mut input_state.brake, input.pressed(KeyCode::X));
}

fn update_input_state(input_state: &mut JoyrideInputState, press_state: bool) {
    let new_state = if press_state {
        match input_state {
            JoyrideInputState::Released | JoyrideInputState::JustReleased => {
                JoyrideInputState::JustPressed
            }
            JoyrideInputState::JustPressed => JoyrideInputState::Pressed,
            JoyrideInputState::Pressed => JoyrideInputState::Pressed,
        }
    } else {
        match input_state {
            JoyrideInputState::Pressed | JoyrideInputState::JustPressed => {
                JoyrideInputState::JustReleased
            }
            JoyrideInputState::JustReleased => JoyrideInputState::Released,
            JoyrideInputState::Released => JoyrideInputState::Released,
        }
    };

    *input_state = new_state;
}
