use bevy::log::LogPlugin;
use bevy::prelude::*;
use crossbeam_channel::{unbounded};
use nokhwa::{CameraFormat, FrameFormat};

use background_node::BackgroundNodePlugin;

use crate::background_node::Webcam;

mod background_node;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins_with(DefaultPlugins, |plugins| plugins.disable::<LogPlugin>())
        .add_plugin(BackgroundNodePlugin)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });


    let (sender, receiver) = unbounded();
    std::thread::spawn(move || {
        let mut camera = nokhwa::Camera::new(
            0,                                                              // index
            Some(CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)), // format
        )
            .unwrap();
        // open stream
        camera.open_stream().unwrap();
        loop {
            let frame = camera.frame().unwrap();
            // println!("width: {} height: {}", frame.width(), frame.height());
            let _ = sender.send(frame);
        }
    });

    commands.insert_resource(Webcam { receiver });
}
