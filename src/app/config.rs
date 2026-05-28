use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct RendererConfig {
    pub present_mode: Option<PresentMode>,
    pub max_frame_latency: Option<u32>,
}
impl RendererConfig {
    pub fn load_or_default(path: &str) -> Self {
        if !Path::new(path).exists() {
            let config = Self::default();
            std::fs::write(path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
            config
        } else {
            let data: String = std::fs::read_to_string(path).unwrap();
            serde_json::from_str(&data).unwrap_or_else(|_| {
                eprintln!("Failed to parse config, falling back to default.");
                Self::default()
            })
        }
    }
}
impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            present_mode: Some(PresentMode::Mailbox),
            max_frame_latency: Some(2),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum PresentMode {
    Immediate, // 0
    Fifo,      // 1 (Vsync)
    Mailbox,   // 2 (Fast Vsync)
}
