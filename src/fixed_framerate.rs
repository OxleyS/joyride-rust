use bevy::{ecs::schedule::ShouldRun, prelude::*};

pub struct FixedFramerate {
    pub fixed_step: f64,
    pub max_runs_per_step: Option<u32>,
    pub drop_time_after_max_runs: bool,
}

struct FixedFramerateState {
    last_time: bevy::utils::Instant,
    accum_seconds: f64,
    num_updates: u32,
    framerate: FixedFramerate,
}

impl FixedFramerateState {
    fn new(framerate: FixedFramerate) -> Self {
        Self {
            last_time: bevy::utils::Instant::now(),
            accum_seconds: 0.0,
            num_updates: 0,
            framerate,
        }
    }
}

// A zero-dependency system for fixed-framerate, usable at the top-level scheduler
pub fn create_fixed_framerate_run_criteria(
    fixed_framerate: FixedFramerate,
) -> impl System<In = (), Out = ShouldRun> {
    let mut state = FixedFramerateState::new(fixed_framerate);
    let system_fn = move || {
        let cur_time = bevy::utils::Instant::now();
        let elapsed_secs = cur_time.duration_since(state.last_time).as_secs_f64();

        state.accum_seconds += elapsed_secs;
        state.last_time = cur_time;

        let hit_run_cap = if let Some(run_cap) = state.framerate.max_runs_per_step {
            state.num_updates >= run_cap
        } else {
            false
        };

        let step_accumulated = state.accum_seconds >= state.framerate.fixed_step;
        if !step_accumulated || hit_run_cap {
            if step_accumulated && state.framerate.drop_time_after_max_runs {
                state.accum_seconds = 0.0;
            }
            state.num_updates = 0;
            return ShouldRun::No;
        }

        state.accum_seconds -= state.framerate.fixed_step;
        state.num_updates += 1;
        ShouldRun::YesAndCheckAgain
    };

    system_fn.system()
}
