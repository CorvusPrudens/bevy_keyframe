use crate::Animate;

use super::{AnimationDuration, Animations};
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;

#[derive(Resource, Default)]
pub(super) struct PlayheadSteps(HashMap<usize, Vec<PlayheadStep>>);

struct PlayheadStep {
    playhead: Entity,
    start: bool,
    end: bool,
    entity: Entity,
    movement: PlayheadMove,
}

#[derive(Component, Debug, Default)]
pub struct AnimationPlayhead {
    playhead: f32,
    previous_position: f32,
}

#[derive(Event, Component, Debug, Clone, Copy)]
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

    pub(super) fn apply_movement(world: &mut World) -> Result {
        let stages = world
            .resource::<PlayheadSteps>()
            .0
            .keys()
            .max()
            .copied()
            .map(|s| s + 1)
            .unwrap_or(0);

        // bevy_log::info!("total stages: {stages}");

        for stage in 0..stages {
            let Some(items) = world.resource_mut::<PlayheadSteps>().0.remove(&stage) else {
                continue;
            };

            if items.is_empty() {
                continue;
            }

            for PlayheadStep {
                playhead,
                start,
                end,
                entity,
                movement,
            } in items
            {
                bevy_log::info!("movement: {movement:?}");
                world.get_entity_mut(entity)?.insert(movement);

                if start || end {
                    let mut playhead = world.get_entity_mut(playhead)?;

                    if start {
                        playhead.trigger(SequenceEvent::SequenceStarted);
                    }
                    if end {
                        playhead.trigger(SequenceEvent::SequenceCompleted);
                    }
                }
            }

            world.try_schedule_scope(Animate, |world, schedule| {
                schedule.run(world);
            })?;
        }

        Ok(())
    }

    pub(super) fn handle_movement(
        mut playheads: Query<(Entity, &mut Self), Changed<Self>>,
        animation_leaves: Query<&Animations>,
        animations: Query<&AnimationDuration>,
        mut steps: ResMut<PlayheadSteps>,
    ) -> Result {
        for (playhead_entity, mut playhead) in &mut playheads {
            let previous_position = playhead.advance();
            let difference = playhead.get() - previous_position;

            if difference > 0.0 {
                // find the animation node
                let playhead_instant = playhead.get();
                let mut time = 0f32;
                let mut step = 0;

                let mut leaves = animation_leaves.iter_leaves(playhead_entity).peekable();

                while let Some(leaf) = leaves.next() {
                    let duration = animations.get(leaf)?;

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
                            break;
                        }

                        let started = previous_position == 0.0;
                        let ended = playhead_instant >= node_end && leaves.peek().is_none();

                        steps.0.entry(step).or_default().push(PlayheadStep {
                            playhead: playhead_entity,
                            start: started,
                            end: ended,
                            entity: leaf,
                            movement: PlayheadMove { start, end },
                        });

                        step += 1;

                        // If true, the playhead stopped within this node's range.
                        if playhead_instant < node_end {
                            break;
                        }
                    }

                    time += duration;
                }
            } else if difference < 0.0 {
                // find the animation node
                let playhead_instant = playhead.get();
                let mut time = 0f32;

                let mut swept_leaves = Vec::new();
                let mut first_leaf = true;

                for leaf in animation_leaves.iter_leaves(playhead_entity) {
                    let duration = animations.get(leaf)?;

                    let duration = duration.0.as_secs_f32();

                    let node_start = time;
                    let node_end = node_start + duration;

                    // If true, some part of the range occupied by this node has been
                    // swept over.
                    if previous_position <= node_end && previous_position > node_start {
                        let start = (previous_position - node_start).max(0.0);
                        let end = (playhead_instant - node_start).clamp(0.0, duration);

                        // The playhead move does not overlap this node.
                        if playhead_instant > node_end {
                            time += duration;
                            continue;
                        }

                        swept_leaves.push((first_leaf, previous_position, start, end, leaf));
                    }

                    time += duration;
                    first_leaf = false;
                }

                // now manage swept leaves in reverse direction
                for (step, (first_leaf, previous_position, start, end, leaf)) in
                    swept_leaves.into_iter().rev().enumerate()
                {
                    let started = previous_position >= time;
                    let ended = playhead_instant <= 0.0 && first_leaf;

                    steps.0.entry(step).or_default().push(PlayheadStep {
                        playhead: playhead_entity,
                        start: started,
                        end: ended,
                        entity: leaf,
                        movement: PlayheadMove { start, end },
                    });
                }
            }
        }

        Ok(())
    }
}
