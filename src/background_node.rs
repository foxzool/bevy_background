use bevy::{
    core_pipeline,
    prelude::{App, FromWorld, Plugin, QueryState, With, World},
    render::{
        mesh::PrimitiveTopology,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo},
        render_resource::{
            BlendComponent, BlendState, ColorTargetState, ColorWrites, Face, FrontFace, LoadOp,
            MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
            RawFragmentState, RawRenderPipelineDescriptor, RawVertexState, RenderPassDescriptor,
            RenderPipeline, ShaderModuleDescriptor, ShaderSource, TextureFormat,
        },
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget},
        RenderApp,
    },
};

pub const BACKGROUND_GRAPH: &str = "background_graph";
pub const BACKGROUND_NODE: &str = "background_node";
pub const BACKGROUND_PASS_DRIVER: &str = "background_pass_driver";

#[derive(Default)]
pub struct BackgroundNodePlugin;

impl Plugin for BackgroundNodePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<BackgroundPipeline>();
        let background_node = BackgroundNode::new(&mut render_app.world);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        let mut background_graph = RenderGraph::default();
        background_graph.add_node(BACKGROUND_NODE, background_node);
        graph.add_sub_graph(BACKGROUND_GRAPH, background_graph);

        graph.add_node(BACKGROUND_PASS_DRIVER, BackgroundPassDriverNode);
        graph
            .add_node_edge(
                core_pipeline::node::CLEAR_PASS_DRIVER,
                BACKGROUND_PASS_DRIVER,
            )
            .unwrap();
        graph
            .add_node_edge(
                core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                BACKGROUND_PASS_DRIVER,
            )
            .unwrap();
        graph
            .add_node_edge(
                BACKGROUND_PASS_DRIVER,
                core_pipeline::node::MAIN_PASS_DRIVER,
            )
            .unwrap();
        graph
            .remove_node_edge(
                core_pipeline::node::CLEAR_PASS_DRIVER,
                core_pipeline::node::MAIN_PASS_DRIVER,
            )
            .unwrap();
        graph
            .remove_node_edge(
                core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                core_pipeline::node::MAIN_PASS_DRIVER,
            )
            .unwrap();

        // bevy_mod_debugdump::print_render_graph(app);
    }
}

pub struct BackgroundPipeline {
    render_pipeline: RenderPipeline,
}

impl FromWorld for BackgroundPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: RawVertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(RawFragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        Self { render_pipeline }
    }
}

pub struct BackgroundPassDriverNode;

impl Node for BackgroundPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        graph.run_sub_graph(BACKGROUND_GRAPH, vec![])?;

        Ok(())
    }
}

pub struct BackgroundNode {
    query: QueryState<&'static ViewTarget, With<ExtractedView>>,
}

impl BackgroundNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for BackgroundNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for target in self.query.iter_manual(world) {
            let pipeline = world.get_resource::<BackgroundPipeline>().unwrap();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("background_pass"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: None,
            };

            let mut render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);

            render_pass.set_pipeline(&pipeline.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
