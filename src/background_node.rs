use bevy::{
    prelude::{FromWorld, World},
    render::{
        render_graph::{Node, NodeRunError, RenderGraphContext},
        render_resource::{RenderPipeline, TextureFormat},
        renderer::{RenderContext, RenderDevice},
        texture::BevyDefault,
        view::ExtractedWindows,
    },
};
use bevy::window::WindowId;

pub const BACKGROUND_NODE: &str = "background_node";

pub struct BackgroundPipeline {
    render_pipeline: RenderPipeline,
}

impl FromWorld for BackgroundPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.get_resource::<RenderDevice>().unwrap();
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
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

#[derive(Default)]
pub struct BackgroundNode;

impl Node for BackgroundNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.get_resource::<BackgroundPipeline>().unwrap();
        let extracted_window =
            &world.get_resource::<ExtractedWindows>().unwrap().windows[&WindowId::primary()];

        let swap_chain_texture = extracted_window
            .swap_chain_texture
            .as_ref()
            .unwrap()
            .clone();
        let mut render_pass =
            render_context
                .command_encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &swap_chain_texture,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

        render_pass.set_pipeline(&pipeline.render_pipeline);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
