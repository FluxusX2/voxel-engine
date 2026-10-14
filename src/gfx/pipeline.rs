use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{camera, Vec3};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    vertex_attr_array, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState,
    Buffer, BufferAddress, BufferBindingType, BufferDescriptor, BufferUsages,
    Color, ColorTargetState, ColorWrites, CommandEncoder, CompareFunction,
    DepthStencilState, Extent3d, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, ShaderStages, StoreOp, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexState,
    VertexStepMode,
};
use wgpu::Device;

use crate::gfx::camera::Camera;
use crate::world::mesh::Mesh;

const SHADOW_MAP_SIZE: u32 = 2048;
const SHADOW_RADIUS: f32 = 80.0;
const LIGHT_DIRECTION: Vec3 = Vec3::new(-0.4, 0.8, -0.3);

const ATTRIBUTES: [VertexAttribute; 4] =
    vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32];

const SHADOW_ATTRIBUTES: [VertexAttribute; 1] =
    vertex_attr_array![0 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
    ao: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LightUniform {
    direction: [f32; 4],
    color: [f32; 4],
    ground_ambient: [f32; 4],
    sky_ambient: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MatrixUniform {
    matrix: [[f32; 4]; 4],
}

pub struct VoxelRenderer {
    pipeline: RenderPipeline,
    shadow_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,

    camera_buffer: Buffer,
    light_buffer: Buffer,
    light_camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    shadow_camera_bind_group: BindGroup,
    shadow_bind_group: BindGroup,

    _depth_texture: Texture,
    depth_view: TextureView,
    _shadow_texture: Texture,
    shadow_view: TextureView,
    _shadow_sampler: Sampler,

    width: u32,
    height: u32,
    camera: Camera,
}

impl VoxelRenderer {
    pub fn new(
        device: &Device,
        format: TextureFormat,
        width: u32,
        height: u32,
        queue: &Queue,
        mesh: &Mesh,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: cast_slice(&mesh.vertices),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Chunk Index Buffer"),
            contents: cast_slice(&mesh.indices),
            usage: BufferUsages::INDEX,
        });

        let light = LightUniform {
            direction: [-0.4, 0.8, -0.3, 0.0],
            color: [1.0, 0.95, 0.85, 0.0],
            ground_ambient: [0.18, 0.16, 0.13, 0.0],
            sky_ambient: [0.32, 0.40, 0.55, 0.0],
        };

        let light_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Lighting Uniform Buffer"),
            contents: bytemuck::bytes_of(&light),
            usage: BufferUsages::UNIFORM,
        });

        let camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: std::mem::size_of::<MatrixUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_camera_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Light Camera Uniform Buffer"),
            size: std::mem::size_of::<MatrixUniform>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: light_camera_buffer.as_entire_binding(),
                },
            ],
        });

        let shadow_camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow Camera Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shadow_camera_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shadow Camera Bind Group"),
            layout: &shadow_camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: light_camera_buffer.as_entire_binding(),
            }],
        });

        let shadow_texture = device.create_texture(&TextureDescriptor {
            label: Some("Shadow Map"),
            size: Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let shadow_view = shadow_texture.create_view(&TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Shadow Comparison Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_map_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Shadow Map Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(
                            wgpu::SamplerBindingType::Comparison,
                        ),
                        count: None,
                    },
                ],
            });

        let shadow_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Shadow Map Bind Group"),
            layout: &shadow_map_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        let cube_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Cube Shader"),
            source: ShaderSource::Wgsl(
                include_str!("../shader/cube.wgsl").into(),
            ),
        });

        let cube_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Cube Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group_layout),
                    Some(&shadow_map_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Cube Render Pipeline"),
            layout: Some(&cube_pipeline_layout),
            vertex: VertexState {
                module: &cube_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &ATTRIBUTES,
                })],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Option::from(true),
                depth_compare: Option::from(CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &cube_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let shadow_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shadow Shader"),
            source: ShaderSource::Wgsl(
                include_str!("../shader/shadow.wgsl").into(),
            ),
        });

        let shadow_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Shadow Pipeline Layout"),
                bind_group_layouts: &[Some(&shadow_camera_bind_group_layout)],
                immediate_size: 0,
            });

        let shadow_pipeline =
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Shadow Depth Pipeline"),
                layout: Some(&shadow_pipeline_layout),
                vertex: VertexState {
                    module: &shadow_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &SHADOW_ATTRIBUTES,
                    })],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Option::from(true),
                    depth_compare: Option::from(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: Default::default(),
                fragment: None,
                multiview_mask: None,
                cache: None,
            });

        let mut camera = Camera::default();
        if width > 0 && height > 0 {
            camera.set_aspect_ratio(width as f32 / height as f32);
        }

        let (depth_texture, depth_view) = create_depth_texture(device, width, height);

        let mut renderer = Self {
            pipeline,
            shadow_pipeline,
            vertex_buffer,
            index_buffer,
            index_count: mesh.indices.len() as u32,
            camera_buffer,
            light_buffer,
            light_camera_buffer,
            camera_bind_group,
            shadow_camera_bind_group,
            shadow_bind_group,
            _depth_texture: depth_texture,
            depth_view,
            _shadow_texture: shadow_texture,
            shadow_view,
            _shadow_sampler: shadow_sampler,
            width,
            height,
            camera,
        };

        renderer.update_camera(queue, width, height);
        renderer
    }

    pub fn resize(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }

        if self.width != width || self.height != height {
            let (depth_texture, depth_view) =
                create_depth_texture(device, width, height);
            self._depth_texture = depth_texture;
            self.depth_view = depth_view;
            self.width = width;
            self.height = height;
        }

        self.update_camera(queue, width, height);
    }

    pub fn render(&self, encoder: &mut CommandEncoder, target: TextureView) {
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow Map Pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(
                    RenderPassDepthStencilAttachment {
                        view: &self.shadow_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(1.0),
                            store: StoreOp::Store,
                        }),
                        stencil_ops: None,
                    },
                ),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Cube Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(
                RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                },
            ),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.shadow_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(
            self.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }

    pub fn update_camera(&mut self, queue: &Queue, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.camera.set_aspect_ratio(width as f32 / height as f32);

        let camera_matrix = self.camera.view_projection_matrix();
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::bytes_of(&MatrixUniform {
                matrix: camera_matrix.to_cols_array_2d(),
            }),
        );

        let center = self.camera.position();
        let light_direction = LIGHT_DIRECTION.normalize();
        let light_eye = center + light_direction * 160.0;

        let light_view = camera::rh::view::look_at_mat4(light_eye, center, Vec3::Y);
        let light_projection = camera::rh::proj::directx::orthographic(
            -SHADOW_RADIUS,
            SHADOW_RADIUS,
            -SHADOW_RADIUS,
            SHADOW_RADIUS,
            0.1,
            320.0,
        );
        let light_view_projection = light_projection * light_view;

        queue.write_buffer(
            &self.light_camera_buffer,
            0,
            bytemuck::bytes_of(&MatrixUniform {
                matrix: light_view_projection.to_cols_array_2d(),
            }),
        );
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

fn create_depth_texture(
    device: &Device,
    width: u32,
    height: u32,
) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("Depth Texture"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}