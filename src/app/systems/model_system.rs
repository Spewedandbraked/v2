use glam::{Mat4, Vec4};
use wgpu::util::DeviceExt;
use wgpu::wgt::DrawIndirectArgs;

use crate::app::model::Vertex;

pub struct ModelSystem {
    pub models: Vec<Option<Model>>,
    pub gpu_driven: GpuDrivenPool,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
}

pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub index_count: u32,
    pub render_mode: RenderMode,
    pub model_index: u32,
}

#[derive(Copy, Clone, PartialEq)]
pub enum RenderMode {
    CpuDriven,
    GpuDriven,
}

impl ModelSystem {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let max_models: usize = 1024; //TODO: переделать.

        let gpu_driven = GpuDrivenPool::new(device, max_models);
        Self {
            models: Vec::new(),
            gpu_driven,
            depth_texture,
            depth_view,
        }
    }
    pub fn add_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex],
        indices: &[u32],
        render_mode: RenderMode,
        transform: Mat4,
        bounding_sphere: Vec4,
    ) -> usize {
        let index = self.models.len();
        let model = Model::new(device, vertices, indices, render_mode, index as u32);
        self.models.push(Some(model));

        queue.write_buffer(
            &self.gpu_driven.transform_buffer,
            (index * 64) as u64,
            bytemuck::cast_slice(&[transform]),
        );
        queue.write_buffer(
            &self.gpu_driven.bounding_spheres,
            (index * 16) as u64,
            bytemuck::cast_slice(&[bounding_sphere]),
        );

        if render_mode == RenderMode::GpuDriven {
            let cmd = DrawIndirectArgs {
                vertex_count: indices.len() as u32,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            };
            queue.write_buffer(
                &self.gpu_driven.indirect_buffer,
                (index * std::mem::size_of::<DrawIndirectArgs>()) as u64,
                bytemuck::cast_slice(&[cmd]),
            );
        }

        index
    }
    
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.depth_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }
}

impl Model {
    pub fn new(
        device: &wgpu::Device,
        vertices: &[Vertex],
        indices: &[u32],
        render_mode: RenderMode,
        model_index: u32,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
            render_mode,
            model_index,
        }
    }
}

pub struct GpuDrivenPool {
    pub transform_buffer: wgpu::Buffer,
    pub bounding_spheres: wgpu::Buffer,
    pub indirect_buffer: wgpu::Buffer,
    pub culling_pipeline: Option<wgpu::ComputePipeline>,
    pub free_slots: Vec<u32>,
}

impl GpuDrivenPool {
    pub fn new(device: &wgpu::Device, max_models: usize) -> Self {
        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transform Buffer"),
            size: (max_models * 64) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bounding_spheres = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounding Spheres"),
            size: (max_models * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Buffer"),
            size: (max_models * std::mem::size_of::<DrawIndirectArgs>()) as u64,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            transform_buffer,
            bounding_spheres,
            indirect_buffer,
            culling_pipeline: None,
            free_slots: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> u32 {
        self.free_slots.pop().unwrap_or_else(|| {
            // Если свободных нет, возвращаем следующий индекс (буфер нужно расширить)
            // Пока panic
            todo!("Buffer expansion not implemented yet")
        })
    }

    pub fn free(&mut self, slot: u32) {
        self.free_slots.push(slot);
    }
}
