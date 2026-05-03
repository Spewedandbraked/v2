use app::App;

use crate::app::RendererConfig;
mod app;
mod game;

fn main() {
    env_logger::init();

    let app = App::new(RendererConfig::load_or_default("renderer_config.json"));
}
