use bevy::{ecs::schedule::ShouldRun, prelude::*};

struct FixedFramerateState {
    start_time: bevy::utils::Instant,
    last_time: bevy::utils::Instant,
    accum_seconds: f64,
    num_updates: u32,
    fixed_step: f64,
}

impl FixedFramerateState {
    fn new(fixed_step: f64) -> Self {
        Self {
            start_time: bevy::utils::Instant::now(),
            last_time: bevy::utils::Instant::now(),
            accum_seconds: 0.0,
            num_updates: 0,
            fixed_step,
        }
    }
}

// A zero-dependency system for fixed-framerate, usable at the top-level scheduler
pub fn create_fixed_framerate_run_criteria(
    fixed_step: f64,
) -> impl System<In = (), Out = ShouldRun> {
    let mut state = FixedFramerateState::new(fixed_step);
    let system_fn = move || {
        let cur_time = bevy::utils::Instant::now();
        let elapsed_secs = cur_time.duration_since(state.last_time).as_secs_f64();

        state.accum_seconds += elapsed_secs;
        state.last_time = cur_time;

        println!("{}", state.start_time.elapsed().as_secs_f64());

        state.num_updates = 0;
        if state.accum_seconds < state.fixed_step || state.num_updates >= 5 {
            if state.num_updates >= 5 {
                state.accum_seconds = 0.0;
            }
            state.num_updates = 0;
            return ShouldRun::No;
        }

        state.accum_seconds -= state.fixed_step;
        state.num_updates += 1;
        ShouldRun::YesAndCheckAgain
    };

    system_fn.system()
}
