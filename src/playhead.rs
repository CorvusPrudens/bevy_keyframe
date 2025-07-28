use super::{AnimationDuration, Animations};
use bevy_ecs::prelude::*;

#[derive(Component, Debug, Default)]
pub struct AnimationPlayhead {
    playhead: f32,
    previous_position: f32,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct PlayheadMove {
    pub start: f32,
    pub end: f32,
}

#[derive(Event, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SequenceEvent {
    SequenceStarted,
    SequenceCompleted,
}

impl AnimationPlayhead {
    pub fn get(&self) -> f32 {
        self.playhead
    }

    pub fn get_mut(&mut self) -> &mut f32 {
        &mut self.playhead
    }

    pub fn set(&mut self, playhead: f32) {
        self.playhead = playhead;
    }

    /// Move the playhead to a position without triggering any side-effects.
    pub fn jump_to(&mut self, playhead: f32) {
        self.playhead = playhead;
        self.previous_position = playhead;
    }

    /// Return the previous playhead position.
    ///
    /// This advances the stored previous position to the current playhead.
    fn advance(&mut self) -> f32 {
        let previous_position = self.previous_position;
        self.previous_position = self.playhead;

        previous_position
    }

    pub(super) fn handle_movement(
        mut playheads: Query<(Entity, &mut Self), Changed<Self>>,
        animation_leaves: Query<&Animations>,
        animations: Query<(Entity, &AnimationDuration)>,
        mut commands: Commands,
    ) -> Result {
        for (playhead_entity, mut playhead) in &mut playheads {
            let previous_position = playhead.advance();
            let difference = playhead.get() - previous_position;

            if difference > 0.0 {
                // find the animation node
                let playhead_instant = playhead.get();
                let mut time = 0f32;

                let mut leaves = animation_leaves.iter_leaves(playhead_entity).peekable();

                while let Some(leaf) = leaves.next() {
                    let (node, duration) = animations.get(leaf)?;

                    let duration = duration.0.as_secs_f32();

                    let node_start = time;
                    let node_end = node_start + duration;

                    // If true, some part of the range occupied by this node has been
                    // swept over.
                    if previous_position <= node_end {
                        let start = (previous_position - node_start).max(0.0);
                        let end = (playhead_instant - node_start).min(duration);

                        // The playhead move does not overlap this node.
                        if playhead_instant < node_start {
                            continue;
                        }

                        // Start of the sequence
                        if previous_position == 0.0 {
                            commands
                                .entity(playhead_entity)
                                .trigger(SequenceEvent::SequenceStarted);
                        }

                        commands.entity(node).trigger(PlayheadMove { start, end });

                        // If true, the playhead stopped within this node's range.
                        if playhead_instant < node_end {
                            break;
                        } else if playhead_instant >= node_end && leaves.peek().is_none() {
                            // the last leaf, so we're done!
                            commands
                                .entity(playhead_entity)
                                .trigger(SequenceEvent::SequenceCompleted);
                        }
                    }

                    time += duration;
                }
            } else if difference < 0.0 {
                todo!("handle reverse playback");
            }
        }

        Ok(())
    }
}
