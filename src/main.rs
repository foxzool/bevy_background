use background_node::{BackgroundNode, BackgroundPipeline, BACKGROUND_NODE};
use bevy::{
    core_pipeline::node,
    prelude::*,
    render::{render_graph::RenderGraph, RenderApp},
};
use bevy::log::LogPlugin;

mod background_node;

fn main() {
    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins_with(DefaultPlugins, |plugins| plugins.disable::<LogPlugin>())

        .add_startup_system(setup);

    let render_app = app.sub_app_mut(RenderApp);
    render_app.init_resource::<BackgroundPipeline>();
    let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

    graph.add_node(BACKGROUND_NODE, BackgroundNode::default());
    graph
        .add_node_edge(node::MAIN_PASS_DEPENDENCIES, BACKGROUND_NODE)
        .unwrap();


    // it's now work before main pass
    graph
        .add_node_edge(node::CLEAR_PASS_DRIVER, BACKGROUND_NODE)
        .unwrap();
    graph
        .add_node_edge(BACKGROUND_NODE, node::MAIN_PASS_DRIVER)
        .unwrap();
    graph
        .remove_node_edge(node::CLEAR_PASS_DRIVER, node::MAIN_PASS_DRIVER)
        .unwrap();
    graph
        .remove_node_edge(node::MAIN_PASS_DEPENDENCIES, node::MAIN_PASS_DRIVER)
        .unwrap();


    // it's worked after main pass
    // graph
    // .add_node_edge(node::MAIN_PASS_DRIVER, BACKGROUND_NODE)
    // .unwrap();

    // bevy_mod_debugdump::print_render_graph(&mut app);

    app.run();
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
}
