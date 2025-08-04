use bevy::prelude::*;
use bevy_ecs::spawn::SpawnIter;
use bevy_keyframe::{drivers::TimeDriver, *};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, KeyframePlugin))
        .add_systems(Startup, startup)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let target_color = Color::WHITE;
    let start_color = target_color.with_alpha(0.0);
    let font_size = 52.0;

    commands.spawn((
        Text2d::new("Bevy Keyframe"),
        TextFont {
            font_size,
            ..Default::default()
        },
        TextColor(start_color),
        Transform::default(),
        title_shift(0.25, target_color, font_size),
    ));
}

fn title_shift(initial_delay: f32, target_color: Color, font_size: f32) -> impl Bundle {
    (
        lens!(Transform::translation),
        lens!(TextColor::0),
        TimeDriver::default(),
        animations![
            AnimationDuration::secs(initial_delay),
            (
                Keyframe(Vec3::new(0.0, 100.0, 0.0)),
                Keyframe(target_color),
                AnimationDuration::secs(1.3),
                AnimationCurve(EaseFunction::QuarticInOut),
            ),
            AnimationCallback::new(move |mut commands: Commands| {
                commands.spawn((Transform::from_xyz(0.0, 100.0, -1.0), shadow(font_size)));
            }),
        ],
    )
}

fn shadow(font_size: f32) -> impl Bundle {
    let start_color = Color::WHITE;
    let target_color = Color::srgba_u8(86, 130, 89, 96);
    let shadow_elements = 2;

    let items = (1..1 + shadow_elements).map(move |i| {
        let color = start_color.mix(&target_color, i as f32 / shadow_elements as f32);
        let dist = i as f32 * 5.0;
        let z = -i as f32;

        let curve_and_duration = (
            AnimationCurve(EaseFunction::CubicInOut),
            AnimationDuration::secs(0.75),
        );

        (
            Text2d::new("Bevy Keyframe"),
            TextFont {
                font_size,
                ..Default::default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, 0.0, z),
            lens!(Transform::translation),
            TimeDriver {
                mode: drivers::PlaybackMode::Repeat(drivers::RepeatMode::Restart),
                ..Default::default()
            },
            animations![
                (curve_and_duration, Keyframe(Vec3::new(-dist, dist, z))),
                (curve_and_duration, Keyframe(Vec3::new(0.0, 0.0, z))),
                (curve_and_duration, Keyframe(Vec3::new(dist, -dist, z))),
                (curve_and_duration, Keyframe(Vec3::new(0.0, 0.0, z))),
            ],
        )
    });

    Children::spawn(SpawnIter(items))
}
