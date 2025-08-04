// use firewheel::Volume;
use bevy_color::{Color, Mix};
use bevy_math::prelude::*;

pub trait AnimationLerp: Clone + Send + Sync + 'static {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self;

    fn forwards_delta(&self, other: &Self) -> Self;

    fn backwards_delta(&self, other: &Self) -> Self;
}

impl AnimationLerp for f32 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        self + other
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        self - other
    }
}

impl AnimationLerp for f64 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount as f64)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        self + other
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        self - other
    }
}

impl AnimationLerp for Vec2 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        self + other
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        self - other
    }
}

impl AnimationLerp for Vec3 {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        self + other
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        self - other
    }
}

impl AnimationLerp for Quat {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.lerp(*other, amount)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        *self * *other
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        *self * -*other
    }
}

impl AnimationLerp for Color {
    fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
        self.mix(other, amount)
    }

    fn forwards_delta(&self, other: &Self) -> Self {
        let a = bevy_color::Oklaba::from(*self);
        let b = bevy_color::Oklaba::from(*other);

        Color::from(a + b)
    }

    fn backwards_delta(&self, other: &Self) -> Self {
        let a = bevy_color::Oklaba::from(*self);
        let b = bevy_color::Oklaba::from(*other);

        Color::from(a - b)
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

        fn forwards_delta(&self, other: &Self) -> Self {
            match (self, other) {
                (Self::Linear(a), Self::Linear(b)) => Self::Linear(a + b),
                (Self::Decibels(a), Self::Decibels(b)) => Self::Decibels(a + b),
                (Self::Decibels(a), b) => Self::Decibels(a + clamp(b.decibels())),
                (a, Self::Decibels(b)) => Self::Decibels(b + clamp(a.decibels())),
            }
        }

        fn backwards_delta(&self, other: &Self) -> Self {
            match (self, other) {
                (Self::Linear(a), Self::Linear(b)) => Self::Linear(a - b),
                (Self::Decibels(a), Self::Decibels(b)) => Self::Decibels(a - b),
                (Self::Decibels(a), b) => Self::Decibels(a - clamp(b.decibels())),
                (a, Self::Decibels(b)) => Self::Decibels(clamp(a.decibels()) - b),
            }
        }
    }

    impl AnimationLerp for InstantSeconds {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }

        fn forwards_delta(&self, other: &Self) -> Self {
            Self(self.0 + other.0)
        }

        fn backwards_delta(&self, other: &Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl AnimationLerp for InstantMusical {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Self(self.0.lerp(other.0, amount as f64))
        }

        fn forwards_delta(&self, other: &Self) -> Self {
            Self(self.0 + other.0)
        }

        fn backwards_delta(&self, other: &Self) -> Self {
            Self(self.0 - other.0)
        }
    }

    impl<T: AnimationLerp> AnimationLerp for Notify<T> {
        fn animation_lerp(&self, other: &Self, amount: f32) -> Self {
            Notify::new(self.as_ref().animation_lerp(other.as_ref(), amount))
        }

        fn forwards_delta(&self, other: &Self) -> Self {
            Notify::new(self.as_ref().forwards_delta(other.as_ref()))
        }

        fn backwards_delta(&self, other: &Self) -> Self {
            Notify::new(self.as_ref().backwards_delta(other.as_ref()))
        }
    }
}
