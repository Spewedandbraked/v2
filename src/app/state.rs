use std::{num::NonZero, sync::Arc};

use image::GenericImageView;
use winit::window::Window;

use crate::{
    app::{
        RendererConfig,
        camera::{CameraUniform, ScreenUniform},
        config::PresentMode,
        light::*,
        model::Vertex,
        texture::TextureAtlas,
    },
    game::{GameState, gltf_loader},
};
use wgpu::util::DeviceExt;

pub struct State {
    window: Arc<Window>,
    // wgpu-инфраструктура
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Пайплайны
    pbr_pipeline: wgpu::RenderPipeline,
    skybox_pipeline: Option<wgpu::RenderPipeline>,

    // Общие ресурсы (одинаковые для всех сцен)
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    screen_buffer: wgpu::Buffer,

    // Менеджеры GPU-ресурсов
    texture_atlas: TextureAtlas,
    material_bind_groups: Vec<wgpu::BindGroup>,

    // GPU-driven
    indirect_buffer: Option<wgpu::Buffer>,
    scene_transform_buffer: Option<wgpu::Buffer>,

    // Forward+ (Tiled Shading)
    light_buffer: wgpu::Buffer,
    light_grid: wgpu::Buffer,
    light_index_count: wgpu::Buffer,
    light_culling_pipeline: wgpu::ComputePipeline,
    light_culling_bind_group: wgpu::BindGroup,
    light_bind_group: wgpu::BindGroup,
    num_tiles_x: u32,
    num_tiles_y: u32,

    vertex_buffers: Vec<wgpu::Buffer>,
    index_buffers: Vec<wgpu::Buffer>,
    vertex_counts: Vec<u32>,
    index_counts: Vec<u32>,

    // Буфер глубины
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

impl State {
    pub async fn new(window: Arc<Window>, config: &RendererConfig) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: {
                surface_caps
                    .formats
                    .iter()
                    .find(|f| f.is_srgb())
                    .copied()
                    .unwrap_or(surface_caps.formats[0])
            },
            width: size.width,
            height: size.height,
            present_mode: {
                let desired_mode = config.present_mode.as_ref().map(|m| match m {
                    PresentMode::Immediate => wgpu::PresentMode::Immediate,
                    PresentMode::Fifo => wgpu::PresentMode::Fifo,
                    PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
                });
                match desired_mode {
                    Some(mode) if surface_caps.present_modes.contains(&mode) => mode,
                    _ => surface_caps.present_modes[0],
                }
            },
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: config.max_frame_latency.unwrap_or(2) as u32,
        };
        surface.configure(&device, &config);

        let light_culling_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Light Culling Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/light_culling.wgsl").into()),
        });

        let num_tiles_x = (config.width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (config.height + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles = (num_tiles_x * num_tiles_y) as usize;

        let light_culling_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Culling Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // camera
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // lights
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_grid (read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_index_count (read_write)
                ],
            });

        let light_culling_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Culling Pipeline Layout"),
                bind_group_layouts: &[Some(&light_culling_bind_group_layout)],
                immediate_size: 0,
            });
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_grid
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_index_count
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // lights
                ],
            });

        let light_grid_size = num_tiles * MAX_LIGHTS_PER_TILE;
        let light_grid = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Grid"),
            size: (light_grid_size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let light_index_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Index Count"),
            size: (num_tiles * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Buffer"),
            size: (256 * 48) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_grid,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_index_count,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: NonZero::new(12288u64),
                    }),
                },
            ],
        });

        let camera = crate::game::camera::Camera::default();
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &camera_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let light_culling_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Culling Bind Group"),
            layout: &light_culling_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &camera_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_grid,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_index_count,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        // Screen uniform
        let screen_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Buffer"),
            size: std::mem::size_of::<ScreenUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let screen_uniform = ScreenUniform {
            width: config.width as f32,
            height: config.height as f32,
        };
        queue.write_buffer(&screen_buffer, 0, bytemuck::cast_slice(&[screen_uniform]));

        // Обновить light_bind_group_layout — добавить 4-й binding (screen)
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Light Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_grid
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // light_index_count
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // lights
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }, // screen
                ],
            });

        // Обновить light_bind_group — добавить 4-й entry
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_grid,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_index_count,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &screen_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        let texture_atlas = TextureAtlas::new(&device, &config);
        let skybox_pipeline: Option<wgpu::RenderPipeline> = None;
        let material_bind_groups: Vec<wgpu::BindGroup> = Vec::new();
        let indirect_buffer: Option<wgpu::Buffer> = None;
        let scene_transform_buffer: Option<wgpu::Buffer> = None;

        // PBR pipeline layout
        let pbr_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PBR Pipeline Layout"),
            bind_group_layouts: &[
                Some(&camera_bind_group_layout), // @group(0)
                Some(&texture_atlas.layout),     // @group(1) — используем layout из TextureAtlas
                Some(&light_bind_group_layout), // @group(2) — light_grid, light_index_count, lights, screen
            ],
            immediate_size: 0,
        });

        // PBR shader
        let pbr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pbr.wgsl").into()),
        });

        // Vertex buffer layout (для моделей, которые будут загружены)
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2, // tex_coords
                },
            ],
        };

        let pbr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PBR Pipeline"),
            layout: Some(&pbr_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &pbr_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &pbr_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            cache: None,
            multiview_mask: None,
        });
        // Загрузка модели
        let mesh_data = crate::game::gltf_loader::load_gltf("src/assets/scene.glb")?;
        let mut vertex_buffers = Vec::new();
        let mut index_buffers = Vec::new();
        let mut vertex_counts = Vec::new();
        let mut index_counts = Vec::new();

        for mesh in &mesh_data {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            vertex_counts.push(mesh.vertices.len() as u32);
            index_counts.push(mesh.indices.len() as u32);
            vertex_buffers.push(vertex_buffer);
            index_buffers.push(index_buffer);
        }

        // Загрузка текстур
        let texture_data = gltf_loader::load_gltf_textures("src/assets/scene.glb")?;

        // Создаём GPU-текстуры
        let mut gpu_textures = Vec::new();
        for (i, data) in texture_data.iter().enumerate() {
            let img = image::load_from_memory(data)?;
            let rgba = img.to_rgba8();
            let dimensions = img.dimensions();

            let size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Texture {}", i)),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.0),
                    rows_per_image: Some(dimensions.1),
                },
                size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            gpu_textures.push((texture, view));
        }

        // Заменить texture_atlas на первую загруженную текстуру
        let texture_atlas = if let Some((texture, view)) = gpu_textures.first() {
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });

            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Atlas Layout"),
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
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Texture Atlas Bind Group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            TextureAtlas {
                texture: texture.clone(),
                view: view.clone(),
                sampler,
                bind_group,
                layout,
            }
        } else {
            // fallback: пустая текстура
            TextureAtlas::new(&device, &config)
        };
        let light_culling_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Light Culling Pipeline"),
                layout: Some(&light_culling_pipeline_layout),
                module: &light_culling_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pbr_pipeline,
            skybox_pipeline,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            texture_atlas,
            material_bind_groups,
            indirect_buffer,
            scene_transform_buffer,
            light_buffer,
            light_grid,
            light_index_count,
            light_culling_pipeline,
            light_culling_bind_group,
            light_bind_group,
            num_tiles_x,
            num_tiles_y,
            screen_buffer,
            vertex_buffers,
            index_buffers,
            vertex_counts,
            index_counts,
            depth_texture,
            depth_view,
        })
    }
    pub fn render(&mut self, game: &GameState) -> anyhow::Result<()> {
        self.queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::cast_slice(&[ScreenUniform {
                width: self.config.width as f32,
                height: self.config.height as f32,
            }]),
        );
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.surface.configure(&self.device, &self.config);
                texture
            }
            _ => {
                anyhow::bail!("Failed to acquire surface texture");
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // 1. Обновляем буфер источников из GameState
        let mut lights = Vec::with_capacity(MAX_LIGHTS);
        lights.extend(game.world.lights.iter().map(|l| Light {
            position: l.position.into(),
            color: l.color.into(),
            intensity: l.intensity,
            radius: l.radius,
        }));
        lights.resize(
            MAX_LIGHTS,
            Light {
                position: [0.0; 3],
                color: [0.0; 3],
                intensity: 0.0,
                radius: 0.0,
            },
        );

        self.queue
            .write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&lights));

        // 2. Compute-проход: culling источников по тайлам
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Light Culling Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.light_culling_pipeline);
            compute_pass.set_bind_group(0, &self.light_culling_bind_group, &[]);
            compute_pass.dispatch_workgroups(self.num_tiles_x, self.num_tiles_y, 1);
        }

        // 3. Render pass (пока просто очистка экрана, без PBR)
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.pbr_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.texture_atlas.bind_group, &[]);
            render_pass.set_bind_group(2, &self.light_bind_group, &[]);

            for i in 0..self.vertex_buffers.len() {
                render_pass.set_vertex_buffer(0, self.vertex_buffers[i].slice(..));
                render_pass
                    .set_index_buffer(self.index_buffers[i].slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..self.index_counts[i], 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.window.request_redraw();

        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn update(&mut self, game: &mut GameState) {
        game.camera_controller.update(&mut game.camera, 0.016);
        self.camera_uniform.update_view_proj(&game.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
}
