use rend3::types::glam::{self, Quat, Vec3, Vec3A};

pub struct CameraController {
    pub position: glam::Vec3A,
    pub rotation: Quat,
    pub fov: f32,
    pub near: f32,
}
impl Default for CameraController {
    fn default() -> Self {
        let position = Vec3::new(0.0, 2.0, 5.0);
        let look_at = Vec3::new(0.0, 0.0, 0.0);
        let direction = (look_at - position).normalize();
        let rotation = Quat::from_rotation_arc(-glam::Vec3::Z, direction);
        Self {
            position: Vec3A::from(position),
            rotation,
            fov: 110.0,
            near: 0.1,
        }
    }
}
impl CameraController {
    pub fn view_matrix(&self) -> glam::Mat4 {
        // .conjugate() убрать если мир зеркален!!!!!!! слова: зеркало, мир хуйня, ебал мать
        let rotation = glam::Mat4::from_quat(self.rotation.conjugate());
        let translation = glam::Mat4::from_translation((-self.position).into());
        rotation * translation
    }
    pub fn projection(&self) -> rend3::types::CameraProjection {
        rend3::types::CameraProjection::Perspective {
            vfov: self.fov,
            near: self.near,
        }
    }
}
