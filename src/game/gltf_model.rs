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
use std::path::Path;
use anyhow::Result;
use crate::app::model::Vertex;

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn load_gltf(path: &str) -> Result<Vec<MeshData>> {
    let (document, buffers, _) = gltf::import(path)?;
    let mut meshes = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            // Позиции
            let positions = reader.read_positions()
                .map(|p| p.collect::<Vec<_>>())
                .unwrap_or_default();

            // Текстурные координаты (берём первый набор)
            let tex_coords = reader.read_tex_coords(0)
                .map(|t| t.into_f32().collect::<Vec<_>>())
                .unwrap_or_default();

            // Индексы
            let indices = reader.read_indices()
                .map(|i| i.into_u32().collect::<Vec<_>>())
                .unwrap_or_default();

            // Собираем вершины
            let vertices: Vec<Vertex> = positions
                .iter()
                .zip(tex_coords.iter())
                .map(|(&[x, y, z], &[u, v])| Vertex {
                    position: [x, y, z],
                    tex_coords: [u, v],
                })
                .collect();

            meshes.push(MeshData { vertices, indices });
        }
    }

    Ok(meshes)
}