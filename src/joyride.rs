use bevy::{core::FixedTimestep, input::InputSystem, prelude::*};
use easy_cast::*;

pub const FIELD_WIDTH: u32 = 320;
pub const FIELD_HEIGHT: u32 = 240;

// We lock the framerate, since this is a retro-style game, after all
pub const TIME_STEP: f32 = 1.0 / 30.0;

pub struct JoyrideGame {}

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
    pub accel: JoyrideInputState,
    pub brake: JoyrideInputState,
}

#[derive(SystemLabel, PartialEq, Eq, Clone, Copy, Hash, Debug)]
enum InputStageLabels {
    UpdateInstantInput,
}

struct JoyrideInputPressState {
    left: bool,
    right: bool,
    accel: bool,
    brake: bool,
}

impl Default for JoyrideInputPressState {
    fn default() -> Self {
        Self {
            left: false,
            right: false,
            accel: false,
            brake: false,
        }
    }
}

fn startup_joyride(mut commands: Commands) {
    commands.insert_resource(JoyrideGame {});
    commands.insert_resource(JoyrideInputPressState::default());
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
    app.add_system_to_stage(
        CoreStage::PreUpdate,
        update_instant_input
            .system()
            .after(InputSystem)
            .label(InputStageLabels::UpdateInstantInput),
    );
    app.add_system_set_to_stage(
        CoreStage::PreUpdate,
        SystemSet::new()
            .with_run_criteria(FixedTimestep::step(TIME_STEP.cast()))
            .with_system(update_fixedframe_input.system())
            .after(InputStageLabels::UpdateInstantInput),
    );
}

fn update_instant_input(
    input: Res<Input<KeyCode>>,
    mut press_state: ResMut<JoyrideInputPressState>,
) {
    if input.pressed(KeyCode::Left) {
        press_state.left = true;
    }
    if input.pressed(KeyCode::Right) {
        press_state.right = true;
    }
    if input.pressed(KeyCode::Z) {
        press_state.accel = true;
    }
    if input.pressed(KeyCode::X) {
        press_state.brake = true;
    }
}

fn update_fixedframe_input(
    mut press_state: ResMut<JoyrideInputPressState>,
    mut input_state: ResMut<JoyrideInput>,
) {
    update_input_state(&mut input_state.left, &mut press_state.left);
    update_input_state(&mut input_state.right, &mut press_state.right);
    update_input_state(&mut input_state.accel, &mut press_state.accel);
    update_input_state(&mut input_state.brake, &mut press_state.brake);
}

fn update_input_state(input_state: &mut JoyrideInputState, press_state: &mut bool) {
    let new_state = if *press_state {
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
    *press_state = false;
}
