use std::fmt::{Debug, Formatter};

use bevy::{
    prelude::{Entity, Local, NonSendMut, Plugin, Query, Res, Update, debug},
    time::Time,
};
use kira::manager::AudioManager;

use crate::{KiraContext, util::TimerMs};

use super::KiraPlayingSounds;

pub struct KiraDebugPlugin;

impl Plugin for KiraDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, debug_kira_sys);
    }
}

struct DebugKiraManager<'a> {
    manager: &'a AudioManager,
}

struct DebugKiraContext<'a> {
    manager: Option<DebugKiraManager<'a>>,
}

impl<'a> Debug for DebugKiraContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KiraContext")
            .field("manager", &self.manager)
            .finish()
    }
}

impl<'a> Debug for DebugKiraManager<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manager")
            .field("state", &self.manager.state())
            .field("num_sounds", &self.manager.num_sounds())
            .field("num_sub_tracks", &self.manager.num_sub_tracks())
            .field("num_clocks", &self.manager.num_clocks())
            .field("sound_capacity", &self.manager.sound_capacity())
            .field("sub_track_capacity", &self.manager.sub_track_capacity())
            .field("clock_capacity", &self.manager.clock_capacity())
            .finish()
    }
}

// Jump through some hoops to make a debug type that can be printed via the debug trait which takes
// a non-mutable reference.
impl<'a> From<&'a mut KiraContext> for DebugKiraContext<'a> {
    fn from(context: &'a mut KiraContext) -> Self {
        let manager = context
            .get_manager()
            .map(|m| Some(DebugKiraManager { manager: m }))
            .unwrap_or(None);
        DebugKiraContext { manager }
    }
}

fn debug_kira_sys(
    mut kira: NonSendMut<KiraContext>,
    active: Query<(Entity, &KiraPlayingSounds)>,
    time: Res<Time>,
    mut looper: Local<TimerMs<1000>>,
) {
    looper.timer.tick(time.delta());
    if !looper.timer.just_finished() {
        return;
    }
    let context: DebugKiraContext = kira.as_mut().into();
    for (eid, active) in active.iter() {
        debug!("Eid: {:?} has {} active sounds.", eid, active.0.len());
    }
    debug!("Context: {:?}", context);
}
