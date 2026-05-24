use glam::Mat4;

pub struct GltfModel {
    pub name: String,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub num_vertices: u32,
    pub num_indices: u32,
    pub transform: Mat4,
    pub texture_bind_group: Option<wgpu::BindGroup>,
}

impl Default for GltfModel {
    fn default() -> Self {
        Self {
            name: String::new(),
            vertex_buffer: None,
            index_buffer: None,
            num_vertices: 0,
            num_indices: 0,
            transform: Mat4::IDENTITY,
            texture_bind_group: None,
        }
    }
}