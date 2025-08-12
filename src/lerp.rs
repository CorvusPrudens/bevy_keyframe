// use firewheel::Volume;
use bevy_color::{Color, Mix};
use bevy_math::prelude::*;

pub trait AnimationLerp: Default + Clone + Send + Sync + 'static {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self;
    fn difference(&self, other: &Self) -> Self;
    fn accumulate(&mut self, value: &Self);
}

impl AnimationLerp for f32 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn difference(&self, other: &Self) -> Self {
        *self - *other
    }

    fn accumulate(&mut self, value: &Self) {
        *self += *value;
    }
}

impl AnimationLerp for f64 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount as f64)
    }

    fn difference(&self, other: &Self) -> Self {
        *self - *other
    }

    fn accumulate(&mut self, value: &Self) {
        *self += *value;
    }
}

impl AnimationLerp for Vec2 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn difference(&self, other: &Self) -> Self {
        *self - *other
    }

    fn accumulate(&mut self, value: &Self) {
        *self += *value;
    }
}

impl AnimationLerp for Vec3 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn difference(&self, other: &Self) -> Self {
        *self - *other
    }

    fn accumulate(&mut self, value: &Self) {
        *self += *value;
    }
}

impl AnimationLerp for Quat {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn difference(&self, other: &Self) -> Self {
        *other * self.inverse()
    }

    fn accumulate(&mut self, value: &Self) {
        *self = *value * *self;
    }
}

impl AnimationLerp for Color {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.mix(other, amount)
    }

    fn difference(&self, other: &Self) -> Self {
        let a = bevy_color::Oklaba::from(*self);
        let b = bevy_color::Oklaba::from(*other);

        Color::from(a - b)
    }

    fn accumulate(&mut self, value: &Self) {
        let a = bevy_color::Oklaba::from(*self);
        let b = bevy_color::Oklaba::from(*value);

        *self = Color::from(a + b)
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

        fn difference(&self, other: &Self) -> Self {
            match (self, other) {
                (Self::Linear(a), Self::Linear(b)) => Self::Linear(a - b),
                (Self::Decibels(a), Self::Decibels(b)) => Self::Decibels(a - b),
                (Self::Decibels(a), b) => Self::Decibels(a - clamp(b.decibels())),
                (a, Self::Decibels(b)) => Self::Decibels(clamp(a.decibels()) - b),
            }
        }

        fn accumulate(&mut self, value: &Self) {
            let value = match (*self, *value) {
                (Self::Linear(a), Self::Linear(b)) => Self::Linear(a + b),
                (Self::Decibels(a), Self::Decibels(b)) => Self::Decibels(a + b),
                (Self::Decibels(a), b) => Self::Decibels(a + clamp(b.decibels())),
                (a, Self::Decibels(b)) => Self::Decibels(b + clamp(a.decibels())),
            };

            *self = value;
        }
    }

    impl AnimationLerp for InstantSeconds {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }

        fn difference(&self, other: &Self) -> Self {
            Self(self.0 - other.0)
        }

        fn accumulate(&mut self, value: &Self) {
            *self = Self(self.0 + value.0);
        }
    }

    impl AnimationLerp for InstantMusical {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }

        fn difference(&self, other: &Self) -> Self {
            Self(self.0 - other.0)
        }

        fn accumulate(&mut self, value: &Self) {
            *self = Self(self.0 + value.0);
        }
    }

    impl<T: AnimationLerp> AnimationLerp for Notify<T> {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Notify::new(self.as_ref().animation_lerp(other.as_ref(), amount))
        }

        fn difference(&self, other: &Self) -> Self {
            Notify::new(self.as_ref().difference(other.as_ref()))
        }

        fn accumulate(&mut self, value: &Self) {
            self.as_mut().accumulate(value);
        }
    }
}
