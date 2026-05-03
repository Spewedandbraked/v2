use app::App;

use crate::app::RendererConfig;
mod app;
mod game;

fn main() {

    rend3_framework::start(
        App::new(RendererConfig::load_or_default("renderer_config.json")),
        winit::window::WindowBuilder::new(),
    );
}
