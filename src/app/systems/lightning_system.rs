pub struct LightingSystem {
    pub light_buffer: wgpu::Buffer,
    pub light_grid: wgpu::Buffer,
    pub light_index_count: wgpu::Buffer,
    pub light_culling_pipeline: wgpu::ComputePipeline,
    pub light_culling_bind_group: wgpu::BindGroup,
    pub light_bind_group: wgpu::BindGroup,
    pub num_tiles_x: u32,
    pub num_tiles_y: u32,

    pub lights: Vec<Option<Light>>,
    pub free_slots: Vec<u32>,
}

use crate::app::light::{Light, MAX_LIGHTS, MAX_LIGHTS_PER_TILE, TILE_SIZE};
use std::num::NonZero;

impl LightingSystem {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera_buffer: &wgpu::Buffer,
        screen_buffer: &wgpu::Buffer,
        light_bind_group_layout: &wgpu::BindGroupLayout,
        light_culling_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let num_tiles_x = (config.width + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles_y = (config.height + TILE_SIZE - 1) / TILE_SIZE;
        let num_tiles = (num_tiles_x * num_tiles_y) as usize;

        // Буфер источников
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Buffer"),
            size: (256 * 48) as u64, // 256 источников × 48 байт
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Буфер индексов для тайлов
        let light_grid_size = num_tiles * MAX_LIGHTS_PER_TILE;
        let light_grid = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Grid"),
            size: (light_grid_size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Буфер счётчиков источников на тайл
        let light_index_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Light Index Count"),
            size: (num_tiles * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Pipeline layout
        let culling_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Culling Pipeline Layout"),
                bind_group_layouts: &[Some(&light_culling_bind_group_layout)],
                immediate_size: 0,
            });

        // Compute shader
        let culling_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Light Culling Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/light_culling.wgsl").into()),
        });

        // Compute pipeline
        let light_culling_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Light Culling Pipeline"),
                layout: Some(&culling_pipeline_layout),
                module: &culling_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        // Bind group для culling
        let light_culling_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Culling Bind Group"),
            layout: &light_culling_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: camera_buffer,
                        offset: 0,
                        size: NonZero::new(144u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: NonZero::new(12288u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_grid,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_index_count,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        // Bind group для фрагментного шейдера
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light Bind Group"),
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_grid,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_index_count,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &light_buffer,
                        offset: 0,
                        size: NonZero::new(12288u64),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: screen_buffer, 
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        Self {
            light_buffer,
            light_grid,
            light_index_count,
            light_culling_pipeline,
            light_culling_bind_group,
            light_bind_group,
            num_tiles_x,
            num_tiles_y,
            lights: Vec::new(),
            free_slots: Vec::new(),
        }
    }

    /// Добавить источник, вернуть его индекс
    pub fn add_light(&mut self, queue: &wgpu::Queue, light: Light) -> u32 {
        let index = if let Some(slot) = self.free_slots.pop() {
            self.lights[slot as usize] = Some(light);
            slot
        } else {
            self.lights.push(Some(light));
            (self.lights.len() - 1) as u32
        };

        let offset = index as u64 * std::mem::size_of::<Light>() as u64;
        queue.write_buffer(&self.light_buffer, offset, bytemuck::cast_slice(&[light]));

        index
    }

    /// Удалить источник по индексу
    pub fn remove_light(&mut self, queue: &wgpu::Queue, index: u32) {
        if let Some(slot) = self.lights.get_mut(index as usize) {
            *slot = None;
            // Обнуляем слот в буфере (intensity = 0 → шейдер пропустит)
            let empty = Light::default();
            let offset = index as u64 * std::mem::size_of::<Light>() as u64;
            queue.write_buffer(&self.light_buffer, offset, bytemuck::cast_slice(&[empty]));
            self.free_slots.push(index);
        }
    }

    /// Обновить существующий источник
    pub fn update_light(&self, queue: &wgpu::Queue, index: u32, light: Light) {
        let offset = index as u64 * std::mem::size_of::<Light>() as u64;
        queue.write_buffer(&self.light_buffer, offset, bytemuck::cast_slice(&[light]));
        // Если нужно обновить и CPU-сторону:
        // if let Some(slot) = self.lights.get_mut(index as usize) { *slot = Some(light); }
    }

    /// Получить ссылку на источник (для CPU-логики)
    pub fn get_light(&self, index: u32) -> Option<&Light> {
        self.lights.get(index as usize).and_then(|l| l.as_ref())
    }

    /// Получить мутабельную ссылку
    pub fn get_light_mut(&mut self, index: u32) -> Option<&mut Light> {
        self.lights.get_mut(index as usize).and_then(|l| l.as_mut())
    }

    /// Массовое обновление источников из слайса (для обратной совместимости)
    pub fn update(&self, queue: &wgpu::Queue, lights: &[Light]) {
        let mut gpu_lights = Vec::with_capacity(MAX_LIGHTS);
        gpu_lights.extend(lights);
        gpu_lights.resize(MAX_LIGHTS, Light::default());
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&gpu_lights));
    }

    /// Запустить compute-проход culling'а источников
    pub fn run_culling<'a>(&'a self, encoder: &'a mut wgpu::CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Light Culling Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&self.light_culling_pipeline);
        compute_pass.set_bind_group(0, &self.light_culling_bind_group, &[]);
        compute_pass.dispatch_workgroups(self.num_tiles_x, self.num_tiles_y, 1);
    }
}
