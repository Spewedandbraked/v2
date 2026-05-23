#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
        }
    }

    pub fn update_view_proj(&mut self, camera: &crate::game::camera::Camera) {
        let view = glam::Mat4::look_at_rh(
            camera.position,
            camera.position + camera.forward(),
            glam::Vec3::Y,
        );
        let projection = glam::Mat4::perspective_rh(
            camera.fov.to_radians(),
            camera.aspect,
            0.1,
            100.0,
        );
        self.view_proj = (projection * view).to_cols_array_2d();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScreenUniform {
    pub width: f32,
    pub height: f32,
}