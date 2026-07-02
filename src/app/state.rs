use std::{collections::HashMap, sync::Arc};

use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    app::{
        RendererConfig,
        config::PresentMode,
        light::*,
        systems::{
            buffer_layouts::BufferLayouts,
            camera_system::CameraSystem,
            lightning_system::LightingSystem,
            model_system::{ModelSystem, RenderMode},
            texture_system::TextureSystem,
        },
    },
    game::{GameState, skybox::Skybox},
};

pub struct State {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub models: ModelSystem,
    pub lighting: LightingSystem,
    pub camera: CameraSystem,
    pub textures: TextureSystem,
    pub bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,

    pub pbr_pipeline: wgpu::RenderPipeline,
    pub skybox_pipeline: Option<wgpu::RenderPipeline>,
    pub skybox: Option<Skybox>,

    // Новые поля для скайбокса
    pub skybox_vertex_buffer: wgpu::Buffer,
    pub skybox_index_buffer: wgpu::Buffer,
}

impl State {
    pub async fn new(
        window: Arc<Window>,
        config: &RendererConfig,
        camera: &crate::game::camera::Camera,
    ) -> anyhow::Result<Self> {
        // 1. Инфраструктура
        let (surface, device, queue, surface_config) =
            Self::init_wgpu(window.clone(), config).await?;

        // 2. Реестр раскладок
        let buffer_layouts = BufferLayouts::new(&device);

        // 3. Подсистемы
        let (camera_system, textures, lighting, mut models, skybox, skybox_vb, skybox_ib) =
            Self::init_systems(&device, &queue, &surface_config, &buffer_layouts, camera)?;

        // 4. Пайплайны
        let (pbr_pipeline, skybox_pipeline) =
            Self::init_pipelines(&device, &surface_config, &buffer_layouts);

        // 5. Загрузка контента
        Self::load_content(&device, &queue, &mut models).await?;

        // 6. Сборка
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config: surface_config,
            pbr_pipeline,
            skybox_pipeline,
            models,
            lighting,
            camera: camera_system,
            textures,
            bind_group_layouts: buffer_layouts.layouts,
            skybox,
            skybox_vertex_buffer: skybox_vb,
            skybox_index_buffer: skybox_ib,
        })
    }

    // region: Init Helpers
    #[doc(hidden)]
    async fn init_wgpu(
        window: Arc<Window>,
        config: &RendererConfig,
    ) -> anyhow::Result<(
        wgpu::Surface<'static>,
        wgpu::Device,
        wgpu::Queue,
        wgpu::SurfaceConfiguration,
    )> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let size = window.inner_size();
        let surface = instance.create_surface(window)?;

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

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_config = wgpu::SurfaceConfiguration {
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
        surface.configure(&device, &surface_config);

        Ok((surface, device, queue, surface_config))
    }

    #[doc(hidden)]
    fn init_systems(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        layouts: &BufferLayouts,
        camera: &crate::game::camera::Camera,
    ) -> anyhow::Result<(
        CameraSystem,
        TextureSystem,
        LightingSystem,
        ModelSystem,
        Option<Skybox>,
        wgpu::Buffer,
        wgpu::Buffer,
    )> {
        let camera_system = CameraSystem::new(
            device,
            queue,
            layouts.get("camera").unwrap(),
            config.width,
            config.height,
            Some(camera),
        );

        let textures = TextureSystem::new(
            device,
            queue,
            config,
            layouts.get("texture").unwrap(),
            "src/assets/scene.glb",
        )?;

        let lighting = LightingSystem::new(
            device,
            config,
            &camera_system.buffer,
            &camera_system.screen_buffer,
            layouts.get("light_fragment").unwrap(),
            layouts.get("light_culling").unwrap(),
        );

        let models = ModelSystem::new(device, config.width, config.height);

        let skybox = TextureSystem::load_skybox(
            &device,
            &queue,
            layouts.get("skybox").unwrap(),
            &[
                "src/assets/happy-tree.png",
                "src/assets/happy-tree.png",
                "src/assets/happy-tree.png",
                "src/assets/happy-tree.png",
                "src/assets/happy-tree.png",
                "src/assets/happy-tree.png",
            ],
        )
        .ok();

        // Создаём куб для скайбокса
        let (skybox_vb, skybox_ib) = create_skybox_cube(device);

        Ok((
            camera_system,
            textures,
            lighting,
            models,
            skybox,
            skybox_vb,
            skybox_ib,
        ))
    }

    #[doc(hidden)]
    fn init_pipelines(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        layouts: &BufferLayouts,
    ) -> (wgpu::RenderPipeline, Option<wgpu::RenderPipeline>) {
        // PBR pipeline
        let pbr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pbr.wgsl").into()),
        });

        let pbr_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PBR Pipeline Layout"),
            bind_group_layouts: &[
                Some(layouts.get("camera").unwrap()),
                Some(layouts.get("texture").unwrap()),
                Some(layouts.get("light_fragment").unwrap()),
            ],
            immediate_size: 0,
        });

        let vertex_buffer_layout = layouts.vertex_layouts.get("master").unwrap();
        let pbr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PBR Pipeline"),
            layout: Some(&pbr_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &pbr_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout.clone()],
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

        // Skybox pipeline
        let skybox_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/skybox.wgsl").into()),
        });

        let skybox_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Skybox Pipeline Layout"),
                bind_group_layouts: &[
                    Some(layouts.get("camera").unwrap()),
                    Some(layouts.get("skybox").unwrap()),
                ],
                immediate_size: 0,
            });

        let skybox_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        };

        let skybox_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox Pipeline"),
            layout: Some(&skybox_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &skybox_shader,
                entry_point: Some("vs_main"),
                buffers: &[skybox_vertex_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &skybox_shader,
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
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        (pbr_pipeline, Some(skybox_pipeline))
    }

    #[doc(hidden)]
    async fn load_content(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        models: &mut ModelSystem,
    ) -> anyhow::Result<()> {
        let mesh_data = crate::game::gltf_loader::load_gltf("src/assets/scene.glb")?;
        for mesh in &mesh_data {
            models.add_model(
                device,
                queue,
                &mesh.vertices,
                &mesh.indices,
                RenderMode::GpuDriven,
                Mat4::IDENTITY,
                Vec4::new(0.0, 0.0, 0.0, 1.0),
            );
        }
        Ok(())
    }
    // endregion

    pub fn render(&mut self, game: &GameState) -> anyhow::Result<()> {
        let lights: Vec<Light> = game
            .world
            .lights
            .iter()
            .map(|l| Light {
                position: [l.position[0], l.position[1], l.position[2], 0.0],
                color: [l.color[0], l.color[1], l.color[2], 0.0],
                intensity: l.intensity,
                radius: l.radius,
            })
            .collect();

        self.lighting.update(&self.queue, &lights);

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                self.surface.configure(&self.device, &self.config);
                texture
            }
            _ => return Ok(()),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.lighting.run_culling(&mut encoder);

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
                    view: &self.models.depth_view,
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

            // Модели
            render_pass.set_pipeline(&self.pbr_pipeline);
            render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
            render_pass.set_bind_group(1, &self.textures.atlas.bind_group, &[]);
            render_pass.set_bind_group(2, &self.lighting.light_bind_group, &[]);

            for model in self.models.models.iter().flatten() {
                render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..model.index_count, 0, 0..1);
            }

            // Скайбокс
            if let Some(ref skybox_pipeline) = self.skybox_pipeline {
                if let Some(ref skybox) = self.skybox {
                    render_pass.set_pipeline(skybox_pipeline);
                    render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
                    render_pass.set_bind_group(1, &skybox.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.skybox_vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        self.skybox_index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..36, 0, 0..1);
                }
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
            self.models.resize(&self.device, width, height);
            self.camera.resize(&self.queue, width, height);
        }
    }

    pub fn update(&mut self, game: &mut GameState) {
        game.camera_controller.update(&mut game.camera, 0.016);
        self.camera.update(&self.queue, &game.camera);
    }
}

fn create_skybox_cube(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer) {
    let vertices: [f32; 72] = [
        // +X (right)
         1.0, -1.0, -1.0,
         1.0, -1.0,  1.0,
         1.0,  1.0,  1.0,
         1.0,  1.0, -1.0,
        // -X (left)
        -1.0, -1.0,  1.0,
        -1.0, -1.0, -1.0,
        -1.0,  1.0, -1.0,
        -1.0,  1.0,  1.0,
        // +Y (top)
        -1.0,  1.0, -1.0,
         1.0,  1.0, -1.0,
         1.0,  1.0,  1.0,
        -1.0,  1.0,  1.0,
        // -Y (bottom)
        -1.0, -1.0,  1.0,
         1.0, -1.0,  1.0,
         1.0, -1.0, -1.0,
        -1.0, -1.0, -1.0,
        // +Z (front)
        -1.0, -1.0,  1.0,
        -1.0,  1.0,  1.0,
         1.0,  1.0,  1.0,
         1.0, -1.0,  1.0,
        // -Z (back)
         1.0, -1.0, -1.0,
         1.0,  1.0, -1.0,
        -1.0,  1.0, -1.0,
        -1.0, -1.0, -1.0,
    ];

    let indices: [u32; 36] = [
         0,  1,  2,  0,  2,  3, // +X
         4,  5,  6,  4,  6,  7, // -X
         8,  9, 10,  8, 10, 11, // +Y
        12, 13, 14, 12, 14, 15, // -Y
        16, 17, 18, 16, 18, 19, // +Z
        20, 21, 22, 20, 22, 23, // -Z
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Skybox Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Skybox Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    (vertex_buffer, index_buffer)
}