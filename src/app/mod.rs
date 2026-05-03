mod config;
use std::{path::Path, sync::Arc};

use crate::game::GameState;
pub use config::RendererConfig;
use rend3::{Renderer, graph, types::glam};
use rend3_framework::AssetLoader;
use rend3_gltf::{GltfLoadSettings, load_gltf};

pub struct App {
    config: RendererConfig,
    renderer: Option<Arc<Renderer>>,
    routines: Option<Arc<rend3_framework::DefaultRoutines>>,
    game: GameState,
}
impl App {
    pub fn new(config: RendererConfig) -> Self {
        Self {
            config,
            renderer: None,
            routines: None,
            game: GameState::default(),
        }
    }
}

//ctrl-клик по ::App, читаем что там и если считаем нужным поменять - меняем тут.
impl rend3_framework::App for App {
    //похуй, правая, я просто понадеюсь что это ни на что не повлияет. это что то про правило бурачика вроде
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Right;
    fn sample_count(&self) -> rend3::types::SampleCount {
        self.config.samples.0
    }
    fn scale_factor(&self) -> f32 {
        self.config.scale
    }

    fn setup(
        &mut self,
        window: &winit::window::Window,
        renderer: &std::sync::Arc<rend3::Renderer>,
        routines: &std::sync::Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        self.renderer = Some(renderer.clone());
        self.routines = Some(routines.clone());

        let loader = AssetLoader::new_local("assets/", "", "");
        let gltf_bytes =
            pollster::block_on(loader.get_asset(rend3_framework::AssetPath::External("scene.glb")))
                .expect("Failed to read scene.glb");

        let settings = GltfLoadSettings::default();
        let (loaded_scene, instance) = pollster::block_on(load_gltf(
            renderer,
            &gltf_bytes,
            &settings,
            |uri| async move {
                let path = format!("assets/{}", uri);
                std::fs::read(&path) // возвращает Result<Vec<u8>, std::io::Error>
            },
        ))
        .expect("ERROR default GLTF load failed");
        self.game.world.gltf_instance = Some(instance);
    }

    fn handle_event(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        base_rendergraph: &rend3_routine::base::BaseRenderGraph,
        surface: Option<&Arc<rend3::types::Surface>>,
        resolution: rend3::types::glam::UVec2,
        event: rend3_framework::Event<'_, ()>,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
    ) {
        match event {
            winit::event::Event::RedrawRequested(_) => {
                let camera_data = rend3::types::Camera {
                    projection: self.game.camera.projection(),
                    view: self.game.camera.view_matrix(),
                };
                renderer.set_camera_data(camera_data);

                if let Some(surface) = surface {
                    let frame = surface
                        .get_current_texture()
                        .expect("Failed to get surface texture");

                    let mut graph = rend3::graph::RenderGraph::new();
                    let ready = rend3::graph::ReadyData {
                        d2_texture: rend3::managers::TextureManagerReadyOutput {
                            bg: std::collections::HashMap::new(),
                        },
                        d2c_texture: rend3::managers::TextureManagerReadyOutput {
                            bg: std::collections::HashMap::new(),
                        },
                        directional_light_cameras: vec![],
                    };
                    base_rendergraph.add_to_graph(
                        &mut graph,
                        &ready, // <- ReadyData обязателен
                        &routines.pbr.lock(),
                        Some(&routines.skybox.lock()).map(|v| &**v), // <- разворачиваем MutexGuard
                        &routines.tonemapping.lock(),
                        resolution,
                        self.sample_count(),
                        glam::Vec4::new(
                            self.game.world.ambient_light,
                            self.game.world.ambient_light,
                            self.game.world.ambient_light,
                            1.0,
                        ),
                    );
                    let cmd_bufs: Vec<wgpu::CommandBuffer> = vec![];
                    graph.execute(renderer, output, cmd_bufs, &ready);

                    frame.present();
                }
                window.request_redraw();
            }
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    control_flow(winit::event_loop::ControlFlow::Exit);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
