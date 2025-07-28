#![allow(clippy::type_complexity)]

use bevy_app::prelude::*;
use bevy_ecs::{
    component::HookContext,
    prelude::*,
    system::{RunSystemOnce, SystemId},
    world::DeferredWorld,
};
use bevy_math::{Curve, curve::EaseFunction};
use dynamic_systems::DynamicObservers;
use std::{marker::PhantomData, time::Duration};

pub mod drivers;
mod dynamic_systems;
mod lens;
mod lerp;
pub mod playhead;

pub use lens::{DynamicFieldLens, FieldLens};
pub use lerp::AnimationLerp;

#[derive(Debug)]
pub struct KeyframePlugin;

#[derive(SystemSet, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum AnimationSystems {
    Driver,
    Playhead,
}

impl Plugin for KeyframePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<dynamic_systems::DynamicSystemRegistry>()
            .init_resource::<dynamic_systems::DynamicObserverRegistry>()
            .configure_sets(
                PreUpdate,
                AnimationSystems::Playhead.after(AnimationSystems::Driver),
            )
            .add_systems(
                PreUpdate,
                (
                    drivers::TimeDriver::drive_playhead.in_set(AnimationSystems::Driver),
                    playhead::AnimationPlayhead::handle_movement.in_set(AnimationSystems::Playhead),
                ),
            )
            .add_systems(
                Last,
                dynamic_systems::handle_insertions
                    .run_if(resource_changed::<dynamic_systems::DynamicSystemRegistry>),
            )
            .add_observer(drivers::TimeDriver::observe_sequence)
            .add_observer(AnimationCallback::observe_movement);
    }
}

#[derive(Debug, Component)]
#[relationship(relationship_target = Animations)]
pub struct AnimationOf(pub Entity);

#[derive(Debug, Component)]
#[relationship_target(relationship = AnimationOf, linked_spawn)]
#[require(playhead::AnimationPlayhead, Animation)]
pub struct Animations(Vec<Entity>);

#[doc(hidden)]
pub use bevy_ecs::spawn::Spawn;

#[macro_export]
macro_rules! animations {
    [$($effect:expr),*$(,)?] => {
        <$crate::Animations>::spawn(($($crate::Spawn($effect)),*))
    };
}

#[derive(Component, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Animation {
    #[default]
    Sequence,
    Parallel,
    Leaf,
}

#[derive(Component, Default, PartialEq, Eq)]
pub enum AnimationComplete {
    #[default]
    Preserve,
    Remove,
    Despawn,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationDuration(pub Duration);

impl AnimationDuration {
    pub fn secs(seconds: f32) -> Self {
        Self(Duration::from_secs_f32(seconds))
    }
}

#[derive(Component, Default, PartialEq, Eq)]
pub struct SampleRunner;

#[derive(Component, Default, PartialEq, Eq)]
pub struct AnimationEvent<T>(pub T);

#[derive(Component)]
struct FetchAttempts<T> {
    count: usize,
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for FetchAttempts<T> {
    fn default() -> Self {
        Self {
            count: 0,
            marker: PhantomData,
        }
    }
}

#[derive(Component, Default, Debug)]
#[require(AnimationDuration, FetchAttempts<T>)]
#[component(on_add = Self::on_add_hook)]
pub struct Keyframe<T: AnimationLerp + Clone + Send + Sync + 'static>(pub T);

#[derive(Component, Debug, Clone, Copy)]
#[require(AnimationDuration)]
pub struct AnimationCurve(pub EaseFunction);

impl Default for AnimationCurve {
    fn default() -> Self {
        AnimationCurve(EaseFunction::Linear)
    }
}

#[derive(Debug, Component)]
struct StartValue<T>(T);

#[derive(Event)]
#[event(auto_propagate, traversal = &'static AnimationOf)]
pub struct FetchStartValue<T> {
    source: Entity,
    _marker: PhantomData<fn() -> T>,
}

#[derive(Event)]
#[event(auto_propagate, traversal = &'static AnimationOf)]
pub struct AnimatedValue<T>(pub T);

impl<T: AnimationLerp + Clone + Send + Sync + 'static> Keyframe<T> {
    fn on_add_hook(mut world: DeferredWorld, _context: HookContext) {
        world
            .commands()
            .add_observer_dynamic(Self::observe_movement);
    }

    fn handle_movement(
        target: Entity,
        playhead_move: playhead::PlayheadMove,
        q: Query<(
            Entity,
            &Self,
            &AnimationDuration,
            Option<&AnimationCurve>,
            Option<&StartValue<T>>,
        )>,
        mut commands: Commands,
        try_again: bool,
    ) -> Result {
        let Ok((entity, keyframe, duration, curve, start_value)) = q.get(target) else {
            return Ok(());
        };

        match start_value {
            None => {
                if !try_again {
                    return Err("failed to fetch initial value for animation keyframe".into());
                }

                commands.entity(entity).trigger(FetchStartValue::<T> {
                    source: entity,
                    _marker: PhantomData,
                });

                // try again after the data would have been added
                commands.queue(move |world: &mut World| {
                    world.run_system_once(
                        move |q: Query<(
                            Entity,
                            &Self,
                            &AnimationDuration,
                            Option<&AnimationCurve>,
                            Option<&StartValue<T>>,
                        )>,
                              commands: Commands| {
                            Self::handle_movement(entity, playhead_move, q, commands, false)
                        },
                    )
                });
            }
            Some(start_value) => {
                let duration = duration.0.as_secs_f32();
                let t = if duration == 0.0 {
                    1.0
                } else {
                    playhead_move.end / duration
                };

                let t = match curve {
                    Some(curve) => curve.0.sample(t).unwrap_or(t),
                    None => t,
                };

                let new_value = start_value.0.animation_lerp(&keyframe.0, t);

                commands.entity(entity).trigger(AnimatedValue(new_value));
            }
        }

        Ok(())
    }

    fn observe_movement(
        trigger: Trigger<playhead::PlayheadMove>,
        q: Query<(
            Entity,
            &Self,
            &AnimationDuration,
            Option<&AnimationCurve>,
            Option<&StartValue<T>>,
        )>,
        commands: Commands,
    ) -> Result {
        Self::handle_movement(trigger.target(), *trigger.event(), q, commands, true)
    }
}

#[derive(Component)]
#[require(AnimationDuration)]
#[component(on_insert = Self::on_insert_hook)]
pub struct AnimationCallback {
    unregistered_system: Option<Box<dyn FnOnce(&mut World) -> SystemId + Send + Sync>>,
    system_id: Option<SystemId>,
}

impl AnimationCallback {
    pub fn new<S, M>(system: S) -> Self
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static,
    {
        Self {
            unregistered_system: Some(Box::new(move |world| world.register_system(system))),
            system_id: None,
        }
    }

    fn on_insert_hook(mut world: DeferredWorld, context: HookContext) {
        world.commands().queue(move |world: &mut World| {
            let Some(system) = world
                .get_mut::<Self>(context.entity)
                .and_then(|mut cb| cb.unregistered_system.take())
            else {
                return;
            };

            let id = system(world);
            world.get_mut::<Self>(context.entity).unwrap().system_id = Some(id);
        });
    }

    fn observe_movement(
        trigger: Trigger<playhead::PlayheadMove>,
        q: Query<(&Self, &AnimationDuration)>,
        mut commands: Commands,
    ) {
        let Ok((callback, duration)) = q.get(trigger.target()) else {
            return;
        };

        if trigger.end >= duration.0.as_secs_f32() {
            if let Some(id) = callback.system_id {
                commands.run_system(id);
            }
        }
    }
}

// #[derive(Component, Default, PartialEq)]
// pub struct Modifier {
//     value: f32,
//     position: std::time::Duration,
// }

// #[derive(Component, Default, PartialEq, Eq)]
// pub struct VolumeLens;

// #[cfg(test)]
// mod test {
//     use bevy::log::LogPlugin;
//
//     use super::*;
//     use crate::test::run;
//
//     fn prepare_app<F: IntoSystem<(), (), M>, M>(startup: F) -> App {
//         let mut app = App::new();
//
//         app.add_plugins((
//             MinimalPlugins,
//             AssetPlugin::default(),
//             crate::SeedlingPlugin::<crate::profiling::ProfilingBackend> {
//                 graph_config: crate::startup::GraphConfiguration::Empty,
//                 ..crate::SeedlingPlugin::<crate::profiling::ProfilingBackend>::new()
//             },
//             AnimationPlugin,
//             LogPlugin::default(),
//         ))
//         .add_systems(Startup, startup);
//
//         app.finish();
//         app.cleanup();
//         app.update();
//
//         app
//     }
//
//     fn simple(mut commands: Commands) {
//         commands.spawn((
//             VolumeNode {
//                 volume: Volume::SILENT,
//             },
//             drivers::TimeDriver::default(),
//             lens!(VolumeNode::volume),
//             animations![
//                 Keyframe(Volume::Decibels(-24.0)),
//                 (
//                     Keyframe(Volume::Decibels(0.0)),
//                     AnimationCurve(EaseFunction::QuadraticInOut),
//                     AnimationDuration::secs(0.5),
//                 )
//             ],
//         ));
//     }
//
//     fn fade_in(seconds: f32) -> impl Bundle {
//         (
//             lens!(VolumeNode::volume),
//             animations![(
//                 Keyframe(Volume::Linear(1.0)),
//                 AnimationDuration::secs(seconds),
//             )],
//         )
//     }
//
//     fn fade_out(seconds: f32) -> impl Bundle {
//         (
//             lens!(VolumeNode::volume),
//             animations![(
//                 Keyframe(Volume::Linear(0.0)),
//                 AnimationDuration::secs(seconds),
//             )],
//         )
//     }
//
//     #[test]
//     fn test_playhead() {
//         let mut app = prepare_app(|mut commands: Commands| {
//             // simple(commands);
//
//             commands.spawn((
//                 VolumeNode {
//                     volume: Volume::SILENT,
//                 },
//                 drivers::TimeDriver::default(),
//                 animations![
//                     fade_in(1.5),
//                     AnimationDuration(Duration::from_secs(1)),
//                     fade_out(1.5),
//                 ],
//             ));
//         });
//
//         for _ in 0..16 {
//             run(
//                 &mut app,
//                 |q: Query<(
//                     Entity,
//                     &Keyframe<Volume>,
//                     &AnimationDuration,
//                     Option<&StartValue<Volume>>,
//                 )>,
//                  mut commands: Commands| {
//                     for (entity, node, duration, start) in &q {
//                         commands.entity(entity).log_components();
//                         println!("node: {node:?}, duration: {duration:?}, start: {start:?}");
//                     }
//                 },
//             );
//
//             run(
//                 &mut app,
//                 |q: Query<(Entity, &VolumeNode)>, mut commands: Commands| {
//                     for (entity, node) in &q {
//                         // commands.entity(entity).log_components();
//                         println!("node: {node:?}");
//                     }
//                 },
//             );
//             app.update();
//         }
//     }
// }
