#![allow(clippy::type_complexity)]

use bevy_app::prelude::*;
use bevy_ecs::{
    component::HookContext,
    prelude::*,
    schedule::ScheduleLabel,
    system::SystemId,
    world::{DeferredWorld, EntityMutExcept, EntityRefExcept},
};
use bevy_math::{Curve, curve::EaseFunction};
use dynamic_systems::{DynamicObservers, DynamicSystems};
use lens::{AnimationLens, FieldGetter};
use playhead::PlayheadMove;
use std::{sync::Arc, time::Duration};

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
    Animate,
}

#[derive(ScheduleLabel, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Animate;

impl Plugin for KeyframePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<dynamic_systems::DynamicSystemRegistry>()
            .init_resource::<playhead::PlayheadSteps>()
            .init_resource::<dynamic_systems::DynamicObserverRegistry>()
            .init_schedule(Animate)
            .configure_sets(
                PreUpdate,
                (
                    AnimationSystems::Playhead.after(AnimationSystems::Driver),
                    AnimationSystems::Animate.after(AnimationSystems::Playhead),
                ),
            )
            .add_systems(
                PreUpdate,
                (
                    (default_animation_target, propagate_animation_target)
                        .chain()
                        .before(AnimationSystems::Driver),
                    drivers::TimeDriver::drive_playhead.in_set(AnimationSystems::Driver),
                    playhead::AnimationPlayhead::handle_movement.in_set(AnimationSystems::Playhead),
                    playhead::AnimationPlayhead::apply_movement.in_set(AnimationSystems::Animate),
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

// TODO: implement shift
#[derive(Component, Default, Debug)]
#[require(AnimationDuration)]
pub struct Shift<T: AnimationLerp + Clone + Send + Sync + 'static>(pub T);

#[derive(Component, Debug, Clone, Copy)]
#[require(AnimationDuration)]
pub struct AnimationCurve(pub EaseFunction);

impl Default for AnimationCurve {
    fn default() -> Self {
        AnimationCurve(EaseFunction::Linear)
    }
}

#[derive(Debug, Component, Clone)]
// #[component(on_insert = Self::on_add_hook)]
struct Interval<T: AnimationLerp> {
    pub start: T,
    pub end: T,
}

impl<T: AnimationLerp> Interval<T> {
    // fn on_add_hook(mut world: DeferredWorld, context: HookContext) {
    //     let mut commands = world.commands();
    //     commands.add_observer_dynamic(Self::observe_movement);
    //
    //     commands.queue(move |world: &mut World| {
    //         world.run_system_once(
    //             move |q: Query<(Entity, &Self, &AnimationDuration, Option<&AnimationCurve>)>,
    //                   mut commands: Commands|
    //                   -> Result {
    //                 let (entity, interval, duration, curve) = q.get(context.entity)?;
    //
    //                 let duration = duration.0.as_secs_f32();
    //                 let t = if duration == 0.0 {
    //                     1.0
    //                 } else {
    //                     interval.movement.end / duration
    //                 };
    //
    //                 let t = match curve {
    //                     Some(curve) => curve.0.sample(t).unwrap_or(t),
    //                     None => t,
    //                 };
    //
    //                 let new_value = interval.start.animation_lerp(&interval.end, t);
    //
    //                 commands.entity(entity).trigger(AnimatedValue(new_value));
    //
    //                 Ok(())
    //             },
    //         )
    //     })
    // }

    // fn observe_movement(
    //     trigger: Trigger<playhead::PlayheadMove>,
    //     q: Query<(Entity, &Self, &AnimationDuration, Option<&AnimationCurve>)>,
    //     mut commands: Commands,
    // ) -> Result {
    //     let Ok((entity, interval, duration, curve)) = q.get(trigger.target()) else {
    //         return Ok(());
    //     };
    //
    //     let duration = duration.0.as_secs_f32();
    //     let t = if duration == 0.0 {
    //         1.0
    //     } else {
    //         trigger.end / duration
    //     };
    //
    //     let t = match curve {
    //         Some(curve) => curve.0.sample(t).unwrap_or(t),
    //         None => t,
    //     };
    //
    //     let new_value = interval.start.animation_lerp(&interval.end, t);
    //
    //     commands.entity(entity).trigger(AnimatedValue(new_value));
    //
    //     Ok(())
    // }
}

#[derive(Event)]
#[event(auto_propagate, traversal = &'static AnimationOf)]
pub struct FetchInterval<T: AnimationLerp> {
    source: Entity,
    direction: AnimationDirection,
    transformation: Arc<dyn Fn(&T) -> Interval<T> + Send + Sync>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimationDirection {
    Forwards,
    Backwards,
}

// #[derive(Event)]
// #[event(auto_propagate, traversal = &'static AnimationOf)]
// pub struct AnimatedValue<T>(pub T);

#[derive(Component, Debug)]
pub struct AnimationTarget(pub Entity);

fn default_animation_target(
    new_roots: Query<
        Entity,
        (
            Added<Animations>,
            Without<AnimationOf>,
            Without<AnimationTarget>,
        ),
    >,
    mut commands: Commands,
) {
    for new_root in &new_roots {
        commands.entity(new_root).insert(AnimationTarget(new_root));
    }
}

fn propagate_animation_target(
    lenses: Query<Entity, Added<AnimationTarget>>,
    hierarchy: Query<&Animations>,
    conflicts: Query<Has<AnimationTarget>>,
    mut commands: Commands,
) -> Result {
    for new_target in &lenses {
        fn recurse(
            new_target: Entity,
            node: Entity,
            hierarchy: &Query<&Animations>,
            conflicts: &Query<Has<AnimationTarget>>,
            mut commands: Commands,
        ) -> Result {
            for child in hierarchy.get(node).ok().iter().flat_map(|a| a.iter()) {
                if !conflicts.get(child)? {
                    commands.entity(child).insert(AnimationTarget(new_target));
                    recurse(new_target, child, hierarchy, conflicts, commands.reborrow())?;
                }
            }

            Ok(())
        }

        recurse(
            new_target,
            new_target,
            &hierarchy,
            &conflicts,
            commands.reborrow(),
        )?;
    }

    Ok(())
}

#[derive(Component, Default, Debug)]
#[require(AnimationDuration)]
#[component(on_add = Self::on_add_hook)]
pub struct Keyframe<T: AnimationLerp>(pub T);

impl<T: AnimationLerp> Keyframe<T> {
    fn on_add_hook(mut world: DeferredWorld, _context: HookContext) {
        // world
        //     .commands()
        //     .add_observer_dynamic(Self::observe_movement);
    }

    fn handle_movement(
        delta: Query<(
            &Self,
            &AnimationDuration,
            &AnimationLens<T>,
            &AnimationTarget,
            Option<&Interval<T>>,
            Option<&AnimationCurve>,
        )>,
        lens: Query<&DynamicFieldLens<T>>,
        target: Query<EntityMut>,
        mut commands: Commands,
    ) -> Result {
        todo!();

        Ok(())
    }

    // fn observe_movement(
    //     trigger: Trigger<playhead::PlayheadMove>,
    //     mut set: ParamSet<(
    //         Query<(
    //             &Self,
    //             &AnimationDuration,
    //             &AnimationLens<T>,
    //             &AnimationTarget,
    //             Option<&Interval<T>>,
    //             Option<&AnimationCurve>,
    //         )>,
    //         Query<&DynamicFieldLens<T>>,
    //         Query<EntityMut>,
    //     )>,
    //     mut commands: Commands,
    // ) -> Result {
    //     let entity = trigger.target();
    //     let delta = set.p0();
    //     let Ok((keyframe, duration, lens_ref, target, interval, curve)) = delta.get(entity) else {
    //         return Ok(());
    //     };
    //
    //     // copy all the things
    //     let (keyframe, duration, lens_ref, target_entity, interval, curve) = (
    //         keyframe.0.clone(),
    //         duration.0,
    //         lens_ref.get(),
    //         target.0,
    //         interval.cloned(),
    //         curve.copied(),
    //     );
    //     let lens = set.p1().get(lens_ref)?.clone();
    //     let mut target = set.p2();
    //     let mut target = target.get_mut(target_entity)?;
    //
    //     // if we're moving forward and start at zero,
    //     // add the interval!
    //
    //     let just_started = trigger.start == 0.0 && trigger.end > 0.0;
    //
    //     let interval = match (just_started, interval) {
    //         (true, _) | (false, None) => {
    //             let start = lens.get_field(target.reborrow())?;
    //             let interval = Interval {
    //                 start,
    //                 end: keyframe,
    //             };
    //
    //             commands.entity(trigger.target()).insert(interval.clone());
    //
    //             interval
    //         }
    //
    //         (_, Some(interval)) => interval,
    //     };
    //
    //     let duration = duration.as_secs_f32();
    //     let t = if duration == 0.0 {
    //         1.0
    //     } else {
    //         trigger.end / duration
    //     };
    //
    //     let t = match curve {
    //         Some(curve) => curve.0.sample(t).unwrap_or(t),
    //         None => t,
    //     };
    //
    //     let new_value = interval.start.animation_lerp(&interval.end, t);
    //     lens.set_field(target, new_value)?;
    //
    //     Ok(())
    // }
}

#[derive(Component, Default, Debug)]
#[require(AnimationDuration)]
#[component(on_add = Self::on_add_hook)]
pub struct Delta<T: AnimationLerp + Clone + Send + Sync + 'static>(pub T);

impl<T: AnimationLerp + Clone + Send + Sync + 'static> Delta<T> {
    fn on_add_hook(mut world: DeferredWorld, _context: HookContext) {
        world
            .commands()
            .add_systems_dynamic(Animate, || Self::handle_movement);
    }

    fn handle_movement(
        delta: Query<
            (
                Entity,
                &Self,
                &AnimationDuration,
                &AnimationLens<T>,
                &AnimationTarget,
                &PlayheadMove,
                Option<&Interval<T>>,
                Option<&AnimationCurve>,
            ),
            Changed<PlayheadMove>,
        >,
        lens: Query<&DynamicFieldLens<T>>,
        mut target: Query<FieldGetter<T>>,
        mut commands: Commands,
    ) -> Result {
        for (entity, delta, duration, lens_ref, target_ref, movement, interval, curve) in &delta {
            let lens = lens.get(lens_ref.get())?;
            let mut target = target.get_mut(target_ref.0)?;

            // TODO: is this a reasonable skip condition?
            if movement.start == movement.end {
                continue;
            }

            let forwards = movement.start < movement.end;

            if forwards {
                // if we're moving forward and start at zero,
                // add the interval!
                let just_started = movement.start == 0.0;

                let interval = match (just_started, interval) {
                    (true, _) | (false, None) => {
                        let start = lens.get_field(target.reborrow())?;
                        let interval = Interval {
                            end: start.forwards_delta(&delta.0),
                            start,
                        };

                        commands.entity(entity).insert(interval.clone());

                        interval
                    }
                    (_, Some(interval)) => interval.clone(),
                };

                let duration = duration.0.as_secs_f32();
                let t = if duration == 0.0 {
                    1.0
                } else {
                    movement.end / duration
                };

                let t = match curve {
                    Some(curve) => curve.0.sample(t).unwrap_or(t),
                    None => t,
                };

                let new_value = interval.start.animation_lerp(&interval.end, t);
                lens.set_field(target, new_value)?;
            } else {
                let duration = duration.0.as_secs_f32();
                let just_started = movement.start >= duration;

                let interval = match (just_started, interval) {
                    (true, _) | (false, None) => {
                        let start = lens.get_field(target.reborrow())?;
                        let interval = Interval {
                            start: start.backwards_delta(&delta.0),
                            end: start,
                        };

                        commands.entity(entity).insert(interval.clone());

                        interval
                    }
                    (_, Some(interval)) => interval.clone(),
                };

                let t = if duration == 0.0 {
                    1.0
                } else {
                    movement.end / duration
                };

                let t = match curve {
                    Some(curve) => curve.0.sample(t).unwrap_or(t),
                    None => t,
                };

                let new_value = interval.start.animation_lerp(&interval.end, t);
                lens.set_field(target, new_value)?;
            }
        }

        Ok(())
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
