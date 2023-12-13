use bevy::{log::LogPlugin, prelude::*};
use bevy_pickleback::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        // 0.016,
        // ))),
        // )
        .add_plugins(LogPlugin::default())
        .add_plugins(PicklebackServerPlugin)
        .add_systems(Update, (process_events,));
    // .add_systems(Update, blah);
    app.run();
}
fn process_events(mut ev: ResMut<Events<ServerEvent>>) {
    for e in ev.drain() {
        info!("****** SERVER EVENT: {e:?}");
    }
}
// fn blah(server: Res<Server>, mut ex: EventWriter<AppExit>) {
//     if server.time() > 10.0 {
//         // ex.send(AppExit);
//     }
// }
