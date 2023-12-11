use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_pickleback::*;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                0.045,
            ))),
        )
        .add_plugins(LogPlugin::default())
        .add_plugins(PicklebackClientPlugin)
        .run();
}
