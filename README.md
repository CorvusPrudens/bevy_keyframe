# Bevy Keyframe

`bevy_keyframe` is a work-in-progress animation system for Bevy.
`bevy_keyframe` attempts to decouple animation as much as possible via
a flexible timelines, an arbitrarily drivable playhead, and
lazily evaluated animation boundaries.

The performance is made acceptable, if not particularly great,
by attempting to use the ECS for what it's best at. This is achieved
by breaking up animations into "phases," where each phase happens
in sequence by running the `Animate` schedule multiple times.

This is currently a very rough draft. The `Keyframe` entity hasn't been
updated, so only `Delta`s and `AnimationCallback`s work at the moment.
