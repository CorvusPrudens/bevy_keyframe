use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::IntoObserverSystem};
use bevy_platform::collections::{HashMap, HashSet, hash_map::Entry};
use std::any::TypeId;

#[expect(unused)]
pub trait DynamicSystems {
    fn add_systems_dynamic<F, S, M>(&mut self, schedule: impl ScheduleLabel, systems: F)
    where
        F: FnOnce() -> S + Send + Sync + 'static,
        S: IntoScheduleConfigs<Box<dyn System<In = (), Out = Result<(), BevyError>>>, M> + 'static;
}

#[derive(Resource, Default)]
pub(super) struct DynamicSystemRegistry(HashMap<TypeId, Option<DeferredSystemInsertion>>);

#[derive(Component)]
struct DeferredSystemInsertion(Box<dyn FnOnce(&mut World) + Send + Sync>);

impl DynamicSystems for Commands<'_, '_> {
    fn add_systems_dynamic<F, S, M>(&mut self, schedule: impl ScheduleLabel, systems: F)
    where
        F: FnOnce() -> S + Send + Sync + 'static,
        S: IntoScheduleConfigs<Box<dyn System<In = (), Out = Result<(), BevyError>>>, M> + 'static,
    {
        self.queue(|world: &mut World| {
            let id = TypeId::of::<S>();
            if let Entry::Vacant(e) = world.resource_mut::<DynamicSystemRegistry>().0.entry(id) {
                let inserter = DeferredSystemInsertion(Box::new(move |world: &mut World| {
                    // let schedules = world.resource::<Schedules>();
                    // for (label, sched) in schedules.iter() {
                    //     info!("sched: {label:#?}");
                    // }

                    world.schedule_scope(schedule, |_: &mut World, schedule: &mut Schedule| {
                        schedule.add_systems(systems());
                    })
                }));

                e.insert(Some(inserter));
            }
        });
    }
}

pub(super) fn handle_insertions(
    mut commands: Commands,
    mut registry: ResMut<DynamicSystemRegistry>,
) {
    for new_system in registry.0.values_mut().filter_map(|v| v.take()) {
        commands.queue(new_system.0);
    }
}

pub trait DynamicObservers {
    fn add_observer_dynamic<O, E, B, M>(&mut self, systems: O)
    where
        E: Event,
        B: Bundle,
        O: IntoObserverSystem<E, B, M> + 'static;
}

#[derive(Resource, Default)]
pub(super) struct DynamicObserverRegistry(HashSet<TypeId>);

impl DynamicObservers for Commands<'_, '_> {
    fn add_observer_dynamic<O, E, B, M>(&mut self, observer: O)
    where
        E: Event,
        B: Bundle,
        O: IntoObserverSystem<E, B, M> + 'static,
    {
        self.queue(|world: &mut World| {
            let id = TypeId::of::<O>();

            if world.resource_mut::<DynamicObserverRegistry>().0.insert(id) {
                world.add_observer(observer);
            }
        });
    }
}
