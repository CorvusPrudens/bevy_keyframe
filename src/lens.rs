use super::{Animations, dynamic_systems::DynamicObservers, lerp::AnimationLerp};
use crate::{
    AnimationCurve, AnimationDirection, AnimationDuration, AnimationSystems, AnimationTarget,
    Delta, Interval, Keyframe, dynamic_systems::DynamicSystems, playhead::PlayheadMove,
};
use bevy_app::PreUpdate;
use bevy_ecs::{
    component::{HookContext, Mutable},
    prelude::*,
    world::{DeferredWorld, EntityMutExcept},
};
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

// This is kinda stupid, so we'll want to find a better solution.
pub type FieldGetter<'w, T> = EntityMutExcept<
    'w,
    (
        DynamicFieldLens<T>,
        Delta<T>,
        Keyframe<T>,
        AnimationDuration,
        AnimationLens<T>,
        AnimationTarget,
        PlayheadMove,
        Interval<T>,
        AnimationCurve,
    ),
>;

pub trait FieldLens<T: AnimationLerp>: Send + Sync + 'static {
    fn get_field(&self, entity: FieldGetter<T>) -> Result<T>;
    fn set_field(&self, entity: FieldGetter<T>, value: T) -> Result;
}

#[derive(Component)]
pub struct AnimationLens<T: AnimationLerp> {
    lens: Entity,
    _marker: PhantomData<fn() -> T>,
}

impl<T: AnimationLerp> AnimationLens<T> {
    pub fn new(lens: Entity) -> Self {
        Self {
            lens,
            _marker: PhantomData,
        }
    }

    pub fn get(&self) -> Entity {
        self.lens
    }
}

fn propagate_lens_ref<T: AnimationLerp>(
    lenses: Query<Entity, Added<DynamicFieldLens<T>>>,
    hierarchy: Query<&Animations>,
    conflicts: Query<Has<DynamicFieldLens<T>>>,
    mut commands: Commands,
) -> Result {
    for new_lens_entity in &lenses {
        commands
            .entity(new_lens_entity)
            .insert(AnimationLens::<T>::new(new_lens_entity));

        fn recurse<T: AnimationLerp>(
            new_lens: Entity,
            node: Entity,
            hierarchy: &Query<&Animations>,
            conflicts: &Query<Has<DynamicFieldLens<T>>>,
            mut commands: Commands,
        ) -> Result {
            for child in hierarchy.get(node).ok().iter().flat_map(|a| a.iter()) {
                if !conflicts.get(child)? {
                    commands
                        .entity(child)
                        .insert(AnimationLens::<T>::new(new_lens));
                    recurse(new_lens, child, hierarchy, conflicts, commands.reborrow())?;
                }
            }

            Ok(())
        }

        recurse(
            new_lens_entity,
            new_lens_entity,
            &hierarchy,
            &conflicts,
            commands.reborrow(),
        )?;
    }

    Ok(())
}

#[derive(Component, Clone)]
#[component(on_add = Self::on_add_hook)]
pub struct DynamicFieldLens<T: AnimationLerp>(Arc<dyn FieldLens<T>>);

impl<T: AnimationLerp> FieldLens<T> for DynamicFieldLens<T> {
    fn get_field(&self, entity: FieldGetter<T>) -> Result<T> {
        self.0.get_field(entity)
    }

    fn set_field(&self, entity: FieldGetter<T>, value: T) -> Result {
        self.0.set_field(entity, value)
    }
}

impl<T> core::fmt::Debug for DynamicFieldLens<T>
where
    T: AnimationLerp + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynamicFieldLens").finish_non_exhaustive()
    }
}

impl<T> DynamicFieldLens<T>
where
    T: AnimationLerp + Clone + Send + Sync + 'static,
{
    pub fn new<F, C>(lens: F) -> Self
    where
        F: Fn(&mut C) -> &mut T + Send + Sync + 'static,
        C: Component<Mutability = Mutable>,
    {
        FunctionFieldLens::new(lens).into()
    }

    fn on_add_hook(mut world: DeferredWorld, _context: HookContext) {
        let mut commands = world.commands();
        commands.add_systems_dynamic(PreUpdate, || {
            propagate_lens_ref::<T>.before(AnimationSystems::Driver)
        });
    }
}

impl<C, P, F> From<FunctionFieldLens<C, P, F>> for DynamicFieldLens<P>
where
    F: Fn(&mut C) -> &mut P + Send + Sync + 'static,
    C: Component<Mutability = Mutable>,
    P: Clone + Send + Sync + AnimationLerp + 'static,
{
    fn from(value: FunctionFieldLens<C, P, F>) -> Self {
        Self(Arc::new(value))
    }
}

impl<C, P, F> FieldLens<P> for FunctionFieldLens<C, P, F>
where
    F: Fn(&mut C) -> &mut P + Send + Sync + 'static,
    C: Component<Mutability = Mutable>,
    P: Clone + Send + Sync + AnimationLerp + 'static,
{
    fn get_field(&self, mut entity: FieldGetter<P>) -> Result<P> {
        let value = entity
            .get_mut::<C>()
            .map(|mut c| (self.func)(&mut c).clone())
            .ok_or_else(|| {
                format!(
                    "expected component {} on animation target",
                    core::any::type_name::<C>()
                )
            })?;

        Ok(value)
    }

    fn set_field(&self, mut entity: FieldGetter<P>, value: P) -> Result {
        let mut component = entity.get_mut::<C>().ok_or_else(|| {
            format!(
                "expected component {} on animation target",
                core::any::type_name::<C>()
            )
        })?;

        *(self.func)(&mut component) = value;

        Ok(())
    }
}

#[derive(Debug)]
struct FunctionFieldLens<C, P, F> {
    func: F,
    marker: PhantomData<fn(C) -> P>,
}

impl<C, P, F> FunctionFieldLens<C, P, F>
where
    F: Fn(&mut C) -> &mut P + Send + Sync + 'static,
    C: Component<Mutability = Mutable>,
    P: Send + Sync + AnimationLerp + 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            marker: PhantomData,
        }
    }
}

#[macro_export]
macro_rules! lens {
    ($component:ident::$field:tt) => {
        $crate::DynamicFieldLens::new(|component: &mut $component| &mut component.$field)
    };
}
