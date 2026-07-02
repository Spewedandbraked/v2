use crate::game::camera::Camera;

pub struct CameraSystem {
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub screen_buffer: wgpu::Buffer,
}

impl CameraSystem {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        camera: Option<&Camera>,
    ) -> Self {
        let aspect = width as f32 / height as f32;
        let fov = camera.map(|c| c.fov).unwrap_or(60.0);
        let mut uniform = CameraUniform::new(aspect, fov);
        if let Some(camera) = camera {
            uniform.update_view_proj(camera);
        } else {
            uniform.view_proj = glam::Mat4::IDENTITY.to_cols_array_2d();
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&[uniform]));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let screen_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Buffer"),
            size: std::mem::size_of::<ScreenUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &screen_buffer,
            0,
            bytemuck::cast_slice(&[ScreenUniform {
                width: width as f32,
                height: height as f32,
            }]),
        );
        Self {
            uniform,
            buffer,
            bind_group,
            screen_buffer,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        self.uniform.update_view_proj(camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.uniform.aspect = width as f32 / height as f32;
        queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::cast_slice(&[ScreenUniform {
                width: width as f32,
                height: height as f32,
            }]),
        );
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub inv_view: [[f32; 4]; 4],
    pub aspect: f32,
    pub fov: f32,
    _padding: [f32; 2],
}

impl CameraUniform {
    pub fn new(aspect: f32, fov: f32) -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            view: [[0.0; 4]; 4],
            inv_view: [[0.0; 4]; 4],
            aspect,
            fov,
            _padding: [0.0; 2],
        }
    }

    pub fn update_view_proj(&mut self, camera: &crate::game::camera::Camera) {
        let view = glam::Mat4::look_at_rh(
            camera.position,
            camera.position + camera.forward(),
            glam::Vec3::Y,
        );
        let proj = glam::Mat4::perspective_rh(camera.fov.to_radians(), self.aspect, 0.1, 100.0);

        self.view_proj = (proj * view).to_cols_array_2d();
        self.view = view.to_cols_array_2d();
        self.inv_view = view.inverse().to_cols_array_2d();
        self.fov = camera.fov;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScreenUniform {
    pub width: f32,
    pub height: f32,
}
