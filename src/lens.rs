use super::{
    AnimatedValue, AnimationOf, Animations, FetchStartValue, Keyframe, StartValue,
    dynamic_systems::DynamicObservers, lerp::AnimationLerp,
};
use bevy_ecs::{
    component::{HookContext, Mutable},
    prelude::*,
    world::DeferredWorld,
};
use std::{marker::PhantomData, sync::Arc};

pub trait FieldLens<T>: Send + Sync + 'static {
    fn get_field(&self, entity: &mut EntityMut) -> Result<T>;
    fn set_field(&self, entity: &mut EntityMut, value: T) -> Result;
}

#[derive(Component)]
#[component(on_add = Self::on_add_hook)]
pub struct DynamicFieldLens<T: AnimationLerp + Clone + Send + Sync + 'static>(
    Arc<dyn FieldLens<T>>,
);

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
        // world.commands().add_systems_dynamic(PreUpdate, || {
        //     Self::animate.after(Keyframe::<T>::fetch_start_value)
        // });

        let mut commands = world.commands();
        commands.add_observer_dynamic(Self::observe_animation);
        commands.add_observer_dynamic(Self::observe_start_value);
    }

    fn observe_start_value(
        mut trigger: Trigger<FetchStartValue<T>>,
        lens: Query<&Self>,
        source: Query<(Has<StartValue<T>>, Option<&AnimationOf>)>,
        ancestors: Query<&AnimationOf>,
        parents: Query<&Animations>,
        siblings: Query<&Keyframe<T>>,
        mut commands: Commands,
    ) -> Result {
        let (has_start_value, animation_of) = source.get(trigger.source)?;

        if has_start_value {
            return Ok(());
        }

        let Ok(lens) = lens.get(trigger.target()).map(|l| l.0.clone()) else {
            return Ok(());
        };

        trigger.propagate(false);

        // TODO: no parent -- could this be a root node?
        let Some(_parent) = animation_of else {
            commands.entity(trigger.source).log_components();

            // in this case, try to get the initial value from the animation target(?)
            return Err("not yet implemented".into());
        };

        // iterate through all the leaves to find the previous value
        // TODO: obviously this could be massively improved
        let root = ancestors.root_ancestor(trigger.source);
        let mut previous_keyframe = None;
        for leaf in parents.iter_leaves(root) {
            if leaf == trigger.source {
                break;
            }

            if let Ok(keyframe) = siblings.get(leaf) {
                previous_keyframe = Some(keyframe);
            }
        }

        match previous_keyframe {
            Some(keyframe) => {
                commands
                    .entity(trigger.source)
                    .insert(StartValue(keyframe.0.clone()));
            }
            None => {
                let animation_target = ancestors.root_ancestor(trigger.target());
                let animation_node = trigger.source;
                commands.queue(move |world: &mut World| -> Result {
                    let entity = world.entity_mut(animation_target);
                    let value = lens.get_field(&mut entity.into())?;

                    let start_value = StartValue(value.clone());

                    world.commands().entity(animation_node).insert(start_value);

                    Ok(())
                });
            }
        }

        Ok(())
    }

    fn observe_animation(
        mut trigger: Trigger<AnimatedValue<T>>,
        lens: Query<&Self>,
        animation_target: Query<&AnimationOf>,
        mut commands: Commands,
    ) -> Result {
        let Ok(lens) = lens.get(trigger.target()).map(|l| l.0.clone()) else {
            return Ok(());
        };
        let root = animation_target.root_ancestor(trigger.target());
        let value = trigger.0.clone();

        commands.queue(move |world: &mut World| -> Result {
            // TODO: we could accumulate values here for blending instead
            // of immediately applying them.
            let entity = world.get_entity_mut(root)?;
            lens.set_field(&mut entity.into(), value)?;

            Ok(())
        });

        trigger.propagate(false);

        Ok(())
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
    fn get_field(&self, entity: &mut EntityMut) -> Result<P> {
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

    fn set_field(&self, entity: &mut EntityMut, value: P) -> Result {
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
