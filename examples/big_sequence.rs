use bevy::prelude::*;

fn animations(mut commands: Commands) {
    commands.spawn((
        VolumeNode::default(),
        (
            // Each animation tree is built up of nodes.
            //
            // Combinator nodes like this can contain leaves or
            // other combinators. Without a runner, an animation
            // node will not drive any animations directly.
            //
            // By default, animations will target the entity at the root of
            // the animation hierarchy. In this case, all these animations
            // will target the `VolumeNode` above.
            //
            // Note that `Animation::Parallel` is the default and does
            // not need to be specified.
            Animation::Parallel,
            animations![
                (
                    // A sequence combinator will run all its leaves in a sequence, one after
                    // another. Sequence is the default, so it doesn't need to be specified.
                    Animation::Sequence,
                    // Often, you'll want to remove animations when they complete (think one-shot
                    // animations).
                    AnimationComplete::Despawn,
                    // The time runner will advance the animation playhead according
                    // to the `Time` resource.
                    drivers::TimeDriver::default(),
                    // This type will get the `Volume::volume` field for animations.
                    //
                    // Animation entities will search up the hierarchy for the nearest
                    // lens if they don't have one.
                    VolumeLens,
                    // // If you don't want to faff about with lens types and traits for your own
                    // // components, you can also use this shorthand (NOTE: doesn't work for BSN
                    // // assets).
                    // lens!(VolumeNode::volume),
                    animations![
                        // Nodes without a runner will "inherit" their playhead from the nearest
                        // ancestor that has one.
                        // fade to full volume
                        (
                            Keyframe(Volume::Decibels(0.0)),
                            AnimationDuration::secs(0.5),
                        ),
                        // Here's a neested combinator that'll run all three leaves at once.
                        animations![
                            // A simple entity like this facilitates delays.
                            AnimationDuration::secs(0.5),
                            // You can also emit arbitrary events at specific points in the animtion.
                            // AnimationEvent(MyEvent),
                            // Or run arbitrary one-shot systems.
                            // AnimationSystem(|| println!("Hello, world!")),
                        ],
                        // fade back down
                        (
                            Keyframe(Volume::Decibels(-24.0)),
                            AnimationDuration::secs(0.5),
                        )
                    ],
                ),
                (
                    // An animation can simply be a single leaf node.
                    //
                    // Components like `AnimationEvent` have `Animation::Leaf` as a required component,
                    // so this also doesn't generally need to be specified.
                    Animation::Leaf,
                    // We can drive the animation playhead with arbitrary clocks,
                    // like the playhead of a sample.
                    SampleRunner,
                    animations![
                        AnimationDuration(Duration::from_secs_f32(0.5)),
                        // At exactly half a second into a piece of music, we'll trigger some
                        // behavior. Note that since we're following the sample's playhead,
                        // this will respect any pausing, changes in speed, or even reversed
                        // playback!
                        // AnimationEvent(MyEvent),
                    ]
                ),
                (
                    // // A runner like this doesn't have a concept of "start"
                    // // or "end," since it's an arbitrary parameter calculated from the world,
                    // // like how "inside-of-a-building" the player is.
                    // InsideAmountRunner,
                    // Instead of setting the values directly, we can also
                    // multiply their "base" values. The "base" value is
                    // determined by the blended animations or the initial
                    // value of the animation target if no other animations
                    // are present.
                    animations![
                        Modifier {
                            value: 0.75,
                            position: Duration::ZERO,
                        },
                        Modifier {
                            value: 1.0,
                            position: Duration::from_secs_f32(0.5),
                        },
                    ],
                ),
            ],
        ),
        // // For each field on animated types, you can specify a global blending mode.
        // // A sophisticated blending graph is not supported.
        // blend_modes![
        //     (
        //         blend_field!(Volume::volume),
        //         Blending {
        //             domain: BlendMode::Mean,
        //                  // BlendMode::Min,
        //                  // BlendMode::Max,
        //                  // BlendMode::Sum,
        //                  // BlendMode::Product,
        //             modifiers: BlendMode::Product,
        //         },
        //     ),
        // ],
    ));

    // The above is quite involved, but it doesn't have to be.
    // Simple animations are simple to spawn.
    fn fade_in(seconds: f32) -> impl Bundle {
        (
            drivers::TimeDriver::default(),
            lens!(VolumeNode::volume),
            animations![(
                Keyframe(Volume::Decibels(0.0)),
                AnimationDuration(Duration::from_secs_f32(seconds)),
            )],
        )
    }

    commands.spawn((VolumeNode::default(), fade_in(1.5)));
}
