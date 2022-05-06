use std::num::NonZeroU32;
use futures_lite::future;
use bevy::render::render_resource::{BindGroup, BindGroupLayoutEntry, BindingType, Buffer, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension};
use bevy::{
    core_pipeline,
    prelude::*,
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
use bevy::render::renderer::RenderQueue;
use bevy::render::RenderStage;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use image::{Rgb, Rgba, RgbaImage, RgbImage};
use wgpu::BindGroupLayoutDescriptor;

pub const BACKGROUND_GRAPH: &str = "background_graph";
pub const BACKGROUND_NODE: &str = "background_node";
pub const BACKGROUND_PASS_DRIVER: &str = "background_pass_driver";

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    }, // A
    Vertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    }, // B
    Vertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    }, // C
    Vertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    }, // d
];

const INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

#[derive(Default)]
pub struct BackgroundNodePlugin;

impl Plugin for BackgroundNodePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WebcamImage(RgbaImage::new(640, 480)));
        app.add_system(handle_image);
        app.add_system(handle_tasks);
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<BackgroundPipeline>();
        render_app.add_system_to_stage(RenderStage::Extract, extract_texture);
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

pub struct Webcam {
    pub receiver: crossbeam_channel::Receiver<RgbImage>,
}

pub struct WebcamImage(RgbaImage);


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct BackgroundPipeline {
    render_pipeline: RenderPipeline,
}

impl FromWorld for BackgroundPipeline {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let device = world.resource::<RenderDevice>();
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Webcam Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("webcam_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });


        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Webcam Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RawRenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: RawVertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    diffuse_bind_group: Option<BindGroup>,
}

impl BackgroundNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),

            vertex_buffer: None,
            index_buffer: None,
            diffuse_bind_group: None
        }
    }
}

impl Node for BackgroundNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
         if   let Some(img) = world.get_resource::<WebcamImage>() {
             // println!("{:?}", img_res.0.width());
             let device = world.get_resource::<RenderDevice>().unwrap();
             let queue = world.get_resource::<RenderQueue>().unwrap();

             if self.index_buffer.is_none() {
                 let index_buffer = device.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
                     label: Some("Index Buffer"),
                     contents: bytemuck::cast_slice(INDICES),
                     usage: wgpu::BufferUsages::INDEX,
                 });
                 self.index_buffer = Some(index_buffer)
             }
             if self.vertex_buffer.is_none() {
                 let vertex_buffer = device.create_buffer_with_data(&wgpu::util::BufferInitDescriptor {
                     label: Some("Vertex Buffer"),
                     contents: bytemuck::cast_slice(VERTICES),
                     usage: wgpu::BufferUsages::VERTEX,
                 });
                 self.vertex_buffer = Some(vertex_buffer)
             }

             let dimensions = img.0.dimensions();

             let size = wgpu::Extent3d {
                 width: dimensions.0,
                 height: dimensions.1,
                 depth_or_array_layers: 1,
             };
             let texture = device.create_texture(&wgpu::TextureDescriptor {
                 label: Some("webcam_img"),
                 size,
                 mip_level_count: 1,
                 sample_count: 1,
                 dimension: wgpu::TextureDimension::D2,
                 format: wgpu::TextureFormat::Rgba8UnormSrgb,
                 usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
             });

             queue.write_texture(
                 wgpu::ImageCopyTexture {
                     aspect: wgpu::TextureAspect::All,
                     texture: &texture,
                     mip_level: 0,
                     origin: wgpu::Origin3d::ZERO,
                 },
                  &img.0,
                 wgpu::ImageDataLayout {
                     offset: 0,
                     bytes_per_row: NonZeroU32::new(4 * dimensions.0),
                     rows_per_image: NonZeroU32::new(dimensions.1),
                 },
                 size,
             );

             let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
             let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                 address_mode_u: wgpu::AddressMode::ClampToEdge,
                 address_mode_v: wgpu::AddressMode::ClampToEdge,
                 address_mode_w: wgpu::AddressMode::ClampToEdge,
                 mag_filter: wgpu::FilterMode::Linear,
                 min_filter: wgpu::FilterMode::Nearest,
                 mipmap_filter: wgpu::FilterMode::Nearest,
                 ..Default::default()
             });



             let texture_bind_group_layout =
                 device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                     entries: &[
                         wgpu::BindGroupLayoutEntry {
                             binding: 0,
                             visibility: wgpu::ShaderStages::FRAGMENT,
                             ty: wgpu::BindingType::Texture {
                                 multisampled: false,
                                 view_dimension: wgpu::TextureViewDimension::D2,
                                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
                             },
                             count: None,
                         },
                         wgpu::BindGroupLayoutEntry {
                             binding: 1,
                             visibility: wgpu::ShaderStages::FRAGMENT,
                             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                             count: None,
                         },
                     ],
                     label: Some("texture_bind_group_layout"),
                 });

             let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                 layout: &texture_bind_group_layout,
                 entries: &[
                     wgpu::BindGroupEntry {
                         binding: 0,
                         resource: wgpu::BindingResource::TextureView(&view),
                     },
                     wgpu::BindGroupEntry {
                         binding: 1,
                         resource: wgpu::BindingResource::Sampler(&sampler),
                     },
                 ],
                 label: Some("diffuse_bind_group"),
             });


             self.diffuse_bind_group = Some(diffuse_bind_group);


         }


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


            if let (Some(vertex_buffer), Some(index_buffer)) =
            (&self.vertex_buffer, &self.index_buffer)
            {

                let mut render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);

                render_pass.set_pipeline(&pipeline.render_pipeline);

                render_pass.set_bind_group(0, self.diffuse_bind_group.as_ref().unwrap(), &[]);
                render_pass.set_vertex_buffer(0, *vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(*index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
            }


            // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        Ok(())
    }
}


fn handle_image(
    mut commands: Commands,
    web_cam: ResMut<Webcam>,
    thread_pool: Res<AsyncComputeTaskPool>,
) {
    let receiver = web_cam.receiver.clone();
    let task = thread_pool.spawn(async move {
         let mut img =   None;
        while let Ok(img_rev) = receiver.try_recv() {
            img = Some(img_rev)
        }

        img
    });


    commands.spawn().insert(task);
}

fn handle_tasks(
    mut commands: Commands,
    mut image_tasks: Query<(Entity, &mut Task<Option<RgbImage>>)>,
    mut image: ResMut<WebcamImage>,
) {
    for (entity, mut task) in &mut image_tasks.iter_mut() {
        if let Some(rgb) = future::block_on(future::poll_once(&mut *task)) {
            if let Some(rgb) = rgb {
                // println!("rgb {:?}", rgb.dimensions());
                let rgba = rgb8_to_rgba8(rgb);
                image.0 = rgba;
            }


            // Task is complete, so remove task component from entity
            commands.entity(entity).despawn();

        }
    }
}



fn rgb8_to_rgba8(
    img: image::ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> image::ImageBuffer<Rgba<u8>, Vec<u8>> {
    let (width, height) = img.dimensions();
    let mut buffer: image::RgbaImage = image::ImageBuffer::new(width, height);
    for (to, &image::Rgb([r, g, b])) in buffer.pixels_mut().zip(img.pixels()) {
        *to = image::Rgba([r, g, b, 255]);
    }
    buffer
}

fn extract_texture(mut commands: Commands, image: Res<WebcamImage>) {
    commands.insert_resource(WebcamImage(image.0.clone()));
    }