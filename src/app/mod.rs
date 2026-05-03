mod config;
use std::{path::Path, sync::Arc};

use crate::game::GameState;
pub use config::RendererConfig;
use rend3::{Renderer, graph, types::glam};
use rend3_framework::{AssetLoader, Event};
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
    // fn scale_factor(&self) -> f32 {
    //     self.config.scale
    // }

    fn setup(
        &mut self,
        window: &winit::window::Window,
        renderer: &std::sync::Arc<rend3::Renderer>,
        routines: &std::sync::Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        self.renderer = Some(renderer.clone());
        self.routines = Some(routines.clone());

        let loader = AssetLoader::new_local("", "", "");
        let gltf_bytes = pollster::block_on(
            loader.get_asset(rend3_framework::AssetPath::External("src/assets/scene.glb")),
        )
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
        surface: Option<&Arc<wgpu::Surface>>,
        resolution: glam::UVec2,
        event: Event<'_, ()>,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
    ) {
        match event {
            rend3_framework::Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    control_flow(winit::event_loop::ControlFlow::Exit);
                }
                _ => {}
            },
            rend3_framework::Event::RedrawRequested(window_id) =>  {
                
            },
            _ => {}
        }
    }
}
