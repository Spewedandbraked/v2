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
pub fn load_gltf_textures(path: &str) -> Result<Vec<Vec<u8>>> {
    let (document, buffers, images) = gltf::import(path)?;
    let mut textures = Vec::new();
    
    for image in document.images() {
        match image.source() {
            gltf::image::Source::View { view, .. } => {
                let data = &buffers[view.buffer().index()][view.offset()..view.offset() + view.length()];
                textures.push(data.to_vec());
            }
            gltf::image::Source::Uri { uri, .. } => {
                let path = std::path::Path::new(path).parent().unwrap().join(uri);
                textures.push(std::fs::read(path)?);
            }
        }
    }
    Ok(textures)
}