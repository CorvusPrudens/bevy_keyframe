use bevy::prelude::*;
use bevy_color::palettes::css::GREEN;
use bevy_keyframe::{drivers::TimeDriver, *};
use drivers::{PlaybackMode, RepeatMode};
use std::f32::consts::FRAC_PI_2;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, KeyframePlugin))
        .add_systems(Startup, startup)
        .add_systems(Update, watch_tester)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let size = 50.0;
    let square = meshes.add(Rectangle::new(size, size));
    let square_material = materials.add(Color::from(GREEN));

    let scale = 200.0;
    let duration = 1.0;
    let hscale = scale / 2.0;

    let i = 0usize;
    // for i in 0..10000 {
    commands.spawn((
        Mesh2d(square.clone()),
        MeshMaterial2d(square_material.clone()),
        Transform::from_xyz(-scale * 1.5 + i as f32, -hscale, 0.0),
        trace_square(scale, duration),
    ));

    commands.spawn((
        Mesh2d(square.clone()),
        MeshMaterial2d(square_material.clone()),
        Transform::from_xyz(hscale + i as f32, -hscale, 0.0),
        trace_square(scale, duration),
    ));
    // }
}

#[derive(Component)]
struct Tester;

fn watch_tester(q: Query<Entity, With<Tester>>, mut commands: Commands) {
    for tester in &q {
        commands.entity(tester).log_components();
    }
}

fn trace_square(scale: f32, duration: f32) -> impl Bundle {
    let duration_and_curve = (
        AnimationDuration::secs(duration),
        AnimationCurve(EaseFunction::CubicInOut),
    );

    (
        lens!(Transform::translation),
        lens!(Transform::rotation),
        TimeDriver {
            mode: PlaybackMode::Repeat(RepeatMode::PingPong),
            ..Default::default()
        },
        animations![
            (duration_and_curve, Delta(Vec3::X * scale)),
            (
                duration_and_curve,
                Delta(Vec3::Y * scale),
                Delta(Quat::from_rotation_z(FRAC_PI_2)),
            ),
            (duration_and_curve, Delta(Vec3::NEG_X * scale)),
            (
                duration_and_curve,
                Delta(Vec3::NEG_Y * scale),
                Delta(Quat::from_rotation_z(-FRAC_PI_2)),
            ),
        ],
    )
}
