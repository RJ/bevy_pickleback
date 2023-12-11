use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    log::LogPlugin,
    prelude::*,
};
use bevy_pickleback::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            0.016,
        ))),
    )
    .add_plugins(LogPlugin::default())
    .add_plugins(PicklebackServerPlugin)
    .add_systems(Update, blah);
    app.run();
}

fn blah(server: Res<Server>, mut ex: EventWriter<AppExit>) {
    if server.time() > 10.0 {
        // ex.send(AppExit);
    }
}
