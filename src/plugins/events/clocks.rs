use bevy::prelude::Events;

use bevy::prelude::error;
use bevy::prelude::{Commands, Component, Entity, Query, ResMut};
use bevy::reflect::Reflect;

use kira::clock::ClockHandle;
use kira::ClockSpeed;

pub use crate::static_sound_loader::{StaticSoundAsset, StaticSoundFileLoader};

use crate::KiraContext;

#[derive(Component, Default, Reflect)]
pub struct KiraAssociatedClocks(#[reflect(ignore)] pub Vec<ClockHandle>);

pub struct AddClockEvent {
    entity: Entity,
    clock_speed: ClockSpeed,
}

impl AddClockEvent {
    pub fn new(entity: Entity, clock_speed: ClockSpeed) -> Self {
        Self {
            entity,
            clock_speed,
        }
    }
}

pub(super) fn do_add_clock_sys(
    mut commands: Commands,
    mut kira: ResMut<KiraContext>,
    mut query: Query<(Entity, Option<&mut KiraAssociatedClocks>)>,
    mut ev_add_clock: ResMut<Events<AddClockEvent>>,
) {
    for event in ev_add_clock.drain() {
        if let Some(manager) = kira.get_manager() {
            if let Ok(clock_handle) = manager.add_clock(event.clock_speed) {
                if let Ok((eid, clocks)) = query.get_mut(event.entity) {
                    match clocks {
                        Some(mut clocks) => {
                            clocks.0.push(clock_handle);
                        }
                        None => {
                            commands
                                .entity(eid)
                                .insert(KiraAssociatedClocks(vec![clock_handle]));
                        }
                    };
                } else {
                    error!(
                        "Failed to associate clock handle with entity: {:?}. \
                 The handle will be dropped.",
                        event.entity
                    );
                }
            }
        }
    }
}
