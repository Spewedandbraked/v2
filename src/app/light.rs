#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    pub position: [f32; 4],
    pub color: [f32; 4],
    pub intensity: f32,
    pub radius: f32,
}

pub const MAX_LIGHTS: usize = 256;
pub const MAX_LIGHTS_PER_TILE: usize = 64;
pub const TILE_SIZE: u32 = 16;

impl Default for Light {
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            color: [0.0; 4],
            intensity: 0.0,
            radius: 0.0,
        }
    }
}