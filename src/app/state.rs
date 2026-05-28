use std::{collections::HashMap, sync::Arc};

use glam::{Mat4, Vec4};
use image::GenericImageView;
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
            model_system::{ModelSystem, RenderMode}, texture_system::TextureSystem,
        },
    },
    game::{GameState, gltf_loader},
};

pub struct State {
    pub window: Arc<Window>,
    // wgpu-инфраструктура
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    // Подсистемы
    pub models: ModelSystem,
    pub lighting: LightingSystem,
    pub camera: CameraSystem,
    pub textures: TextureSystem,
    pub bind_group_layouts: HashMap<String, wgpu::BindGroupLayout>,

    // Пайплайны
    pub pbr_pipeline: wgpu::RenderPipeline,
    pub skybox_pipeline: Option<wgpu::RenderPipeline>,
}

impl State {
    pub async fn new(
        window: Arc<Window>,
        config: &RendererConfig,
        camera: &crate::game::camera::Camera,
    ) -> anyhow::Result<Self> {
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

        let buffer_layouts = BufferLayouts::new(&device);

        let camera_system = CameraSystem::new(
            &device,
            &queue,
            buffer_layouts.get("camera").unwrap(),
            config.width,
            config.height,
            Some(camera),
        );
        let texture_data = gltf_loader::load_gltf_textures("src/assets/scene.glb")?;

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
        let textures = TextureSystem::new(
            &device,
            &queue,
            &config,
            buffer_layouts.get("texture").unwrap(),
            "src/assets/scene.glb",
        )?;
        let skybox_pipeline: Option<wgpu::RenderPipeline> = None;

        let lighting = LightingSystem::new(
            &device,
            &config,
            &camera_system.buffer,
            &camera_system.screen_buffer,
            buffer_layouts.get("light_fragment").unwrap(),
            buffer_layouts.get("light_culling").unwrap(),
        );
        let mut models = ModelSystem::new(&device, config.width, config.height);

        let pbr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pbr.wgsl").into()),
        });

        let pbr_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PBR Pipeline Layout"),
            bind_group_layouts: &[
                Some(buffer_layouts.get("camera").unwrap()),  // @group(0)
                Some(buffer_layouts.get("texture").unwrap()), // @group(1)
                Some(buffer_layouts.get("light_fragment").unwrap()), // @group(2)
            ],
            immediate_size: 0,
        });
        let vertex_buffer_layout = buffer_layouts.vertex_layouts.get("master").unwrap();
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
        let mesh_data = crate::game::gltf_loader::load_gltf("src/assets/scene.glb")?;
        for mesh in &mesh_data {
            models.add_model(
                &device,
                &queue,
                &mesh.vertices,
                &mesh.indices,
                RenderMode::GpuDriven,
                Mat4::IDENTITY,
                Vec4::new(0.0, 0.0, 0.0, 1.0), // bounding-сфера (радиус 1)
            );
        }
        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pbr_pipeline,
            skybox_pipeline,
            models,
            lighting,
            camera: camera_system,
            textures,
            bind_group_layouts: buffer_layouts.layouts,
        })
    }
    pub fn render(
        &mut self,
        camera: &crate::game::camera::Camera,
        lights: &[Light],
    ) -> anyhow::Result<()> {
        self.lighting.update(&self.queue, lights);

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

        // Compute-проход: culling источников
        self.lighting.run_culling(&mut encoder);

        // Render pass
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
