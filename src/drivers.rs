use super::playhead::{AnimationPlayhead, SequenceEvent};
use bevy_ecs::prelude::*;
use bevy_time::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlaybackState {
    Play,
    Pause,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PlaybackMode {
    Once,
    Repeat(RepeatMode),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RepeatMode {
    /// Restart the animation.
    Restart,
    /// Reverse the animation direction at each end.
    PingPong,
}

#[derive(Component, Debug, PartialEq)]
#[require(AnimationPlayhead)]
pub struct TimeDriver {
    pub speed: f32,
    pub state: PlaybackState,
    pub mode: PlaybackMode,
}

impl Default for TimeDriver {
    fn default() -> Self {
        Self {
            speed: 1.0,
            state: PlaybackState::Play,
            mode: PlaybackMode::Once,
        }
    }
}

impl TimeDriver {
    pub fn play(&mut self) {
        self.state = PlaybackState::Play;
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Pause;
    }

    pub(super) fn drive_playhead(mut q: Query<(&Self, &mut AnimationPlayhead)>, time: Res<Time>) {
        let delta = time.delta_secs();
        for (driver, mut playhead) in &mut q {
            let speed = driver.speed;

            *playhead.get_mut() += delta * speed;
        }
    }

    pub(super) fn observe_sequence(
        trigger: Trigger<SequenceEvent>,
        mut driver: Query<(&mut TimeDriver, &mut AnimationPlayhead)>,
    ) {
        if !matches!(*trigger, SequenceEvent::SequenceCompleted) {
            return;
        }
        let Ok((mut driver, mut playhead)) = driver.get_mut(trigger.target()) else {
            return;
        };

        match driver.mode {
            PlaybackMode::Once => {
                driver.pause();
            }
            PlaybackMode::Repeat(RepeatMode::Restart) => {
                // TODO: this doesn't wrap properly since it'll chop off
                // whatever fractional end bit there was
                playhead.jump_to(0.0);
            }
            PlaybackMode::Repeat(RepeatMode::PingPong) => {
                driver.speed = -driver.speed;
            }
        }
    }
}
