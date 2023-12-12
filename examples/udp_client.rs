use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_pickleback::*;
use pickleback::{prelude::ClientState, PicklebackClient};
use std::time::Duration;

fn main() {
    App::new()
        // .add_plugins(MinimalPlugins)
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                0.016,
            ))),
        )
        .add_plugins(LogPlugin::default())
        // set up events manually, since we are using schedule runner
        // otherwise we'll miss events..
        .add_plugins(PicklebackClientPlugin)
        .add_systems(Update, (process_events, dump_stats))
        .run();
}

fn process_events(mut ev: ResMut<Events<ClientState>>) {
    for e in ev.drain() {
        info!("****** STATE CHANGE: {e:?}");
    }
}

fn dump_stats(client: Res<PicklebackClient>, time: Res<Time>, mut countdown: Local<f64>) {
    *countdown -= time.delta_seconds_f64();
    if *countdown > 0.0 {
        return;
    }
    *countdown = 10.0;
    //
    info!("stats {:?}", client.stats());
    info!("rtt {:?}", client.rtt());
    info!("loss {:?}", client.packet_loss());
}
