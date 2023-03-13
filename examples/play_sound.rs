use bevy::{prelude::App, DefaultPlugins};
use bevy_mod_kira::KiraPlugin;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(KiraPlugin)
        .run();
}
