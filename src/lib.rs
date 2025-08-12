#![allow(clippy::type_complexity)]

use bevy_app::prelude::*;
use bevy_ecs::{
    component::HookContext, prelude::*, schedule::ScheduleLabel, system::SystemId,
    world::DeferredWorld,
};
use bevy_math::{Curve, curve::EaseFunction};
use dynamic_systems::DynamicSystems;
use lens::{AnimationLens, FieldGetter};
use playhead::PlayheadMove;
use std::time::Duration;

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
            .add_systems(Animate, AnimationCallback::handle_movement)
            .add_observer(drivers::TimeDriver::observe_sequence);
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
pub struct Interval<T: AnimationLerp> {
    pub start: T,
    pub end: T,
}

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

fn get_time(duration: Duration, instant: f32, curve: Option<&AnimationCurve>) -> f32 {
    let duration = duration.as_secs_f32();
    let t = if duration == 0.0 {
        1.0
    } else {
        instant / duration
    };

    match curve {
        Some(curve) => curve.0.sample(t).unwrap_or(t),
        None => t,
    }
}

// TODO: manage fetching
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
pub struct Delta<T: AnimationLerp>(pub T);

impl<T: AnimationLerp> Delta<T> {
    fn on_add_hook(mut world: DeferredWorld, _context: HookContext) {
        // dynamically register the necessary systems for convenience
        world
            .commands()
            .add_systems_dynamic(Animate, || Self::handle_movement);
    }

    // This is quite beautiful because it can be stateless. No fetching required.
    fn handle_movement(
        delta: Query<
            (
                &Self,
                &AnimationDuration,
                &AnimationLens<T>,
                &AnimationTarget,
                &PlayheadMove,
                Option<&AnimationCurve>,
            ),
            // This is the key bit. Any time this changes, we can evaluate an animation.
            Changed<PlayheadMove>,
        >,
        lens: Query<&DynamicFieldLens<T>>,
        mut target: Query<FieldGetter<T>>,
    ) -> Result {
        for (delta, duration, lens_ref, target_ref, movement, curve) in &delta {
            let lens = lens.get(lens_ref.get())?;
            let mut target = target.get_mut(target_ref.0)?;

            // TODO: is this a reasonable skip condition?
            if movement.start == movement.end {
                continue;
            }

            let default_value = T::default();

            let start_time = get_time(duration.0, movement.start, curve);
            let start = default_value.animation_lerp(&delta.0, start_time);

            let end_time = get_time(duration.0, movement.end, curve);
            let end = default_value.animation_lerp(&delta.0, end_time);

            let difference = end.difference(&start);

            let mut value = lens.get_field(target.reborrow())?;
            value.accumulate(&difference);
            lens.set_field(target, value)?;
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
        let mut commands = world.commands();
        commands.queue(move |world: &mut World| {
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

    fn handle_movement(
        q: Query<(&Self, &AnimationDuration, &PlayheadMove), Changed<PlayheadMove>>,
        mut commands: Commands,
    ) {
        for (callback, duration, movement) in &q {
            if movement.end >= duration.0.as_secs_f32() {
                if let Some(id) = callback.system_id {
                    commands.run_system(id);
                }
            }
        }
    }
}
