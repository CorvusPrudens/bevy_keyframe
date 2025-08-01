// use firewheel::Volume;
use bevy_color::{Color, Mix};
use bevy_math::prelude::*;

pub trait AnimationLerp {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self;
}

impl AnimationLerp for f32 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }
}

impl AnimationLerp for f64 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount as f64)
    }
}

impl AnimationLerp for Vec2 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }
}

impl AnimationLerp for Vec3 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }
}

impl AnimationLerp for Color {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.mix(other, amount)
    }
}

#[cfg(feature = "firewheel")]
mod firewheel {
    use super::AnimationLerp;
    use bevy_math::FloatExt;
    use firewheel::{
        Volume,
        clock::{InstantMusical, InstantSeconds},
        diff::Notify,
    };

    fn clamp(db: f32) -> f32 {
        if db < -96.0 { -96.0 } else { db }
    }

    impl AnimationLerp for Volume {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            match (self, other) {
                (Self::Linear(a), Self::Linear(b)) => Self::Linear(a.animation_lerp(b, amount)),
                (Self::Decibels(a), Self::Decibels(b)) => {
                    Self::Decibels(a.animation_lerp(b, amount))
                }
                (Self::Decibels(a), b) => {
                    Self::Decibels(a.animation_lerp(&clamp(b.decibels()), amount))
                }
                (a, Self::Decibels(b)) => {
                    Self::Decibels(clamp(a.decibels()).animation_lerp(b, amount))
                }
            }
        }
    }

    impl AnimationLerp for InstantSeconds {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }
    }

    impl AnimationLerp for InstantMusical {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }
    }

    impl<T: AnimationLerp> AnimationLerp for Notify<T> {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Notify::new(self.as_ref().animation_lerp(other.as_ref(), amount))
        }
    }
}
