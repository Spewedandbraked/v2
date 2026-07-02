mod config;
pub mod light;
pub(crate) mod model;
mod state;
mod systems;

use std::sync::Arc;

use crate::{app::state::State, game::GameState};
pub use config::RendererConfig;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct App {
    state: Option<State>,
    config: RendererConfig,
    game: GameState,
}
impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            config: RendererConfig::load_or_default("renderer_config.json"),
            game: GameState::default(),
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state =
            Some(pollster::block_on(State::new(window, &self.config, &self.game.camera)).unwrap())
    }

    // это обработка касстомных ивентов в главном потоке.
    // это канал для передачи своих событий из любого места программы в главный цикл.
    // любое кастомное событие, которое не является системным (не клавиша, не мышь, не закрытие окна).
    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: State) {
        self.state = Some(event)
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update(&mut self.game);
                match state.render(&self.game) {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                // TODO: решить оверхед
                if let Some(mut instruction) = self.game.input_instructions.remove(&code) {
                    instruction.execute(&mut self.game, key_state);
                    self.game.input_instructions.insert(code, instruction);
                }
            }
            WindowEvent::Focused(true) => {
                state.window.request_redraw();
                state.surface.configure(&state.device, &state.config);
            }
            _ => { /* гейминг >:] */ }
        }
    }
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.game
                .camera_controller
                .handle_mouse(&mut self.game.camera, dx, dy);
        }
    }
}
pub fn run() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
