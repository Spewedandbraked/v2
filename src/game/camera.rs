use glam::{Mat4, Quat, Vec3, Vec3A};

pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(position: Vec3, look_at: Vec3, fov: f32, aspect: f32) -> Self {
        let direction = (look_at - position).normalize();
        let rotation = Quat::from_rotation_arc(-Vec3::Z, direction);
        Self {
            position,
            rotation,
            fov,
            aspect,
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 2.0, 5.0), Vec3::ZERO, 60.0, 16.0 / 9.0)
    }
}

use winit::keyboard::KeyCode;

#[derive(Debug)]
pub struct CameraController {
    speed: f32,
    sensitivity: f32,
    pub is_forward: bool,
    pub is_backward: bool,
    pub is_left: bool,
    pub is_right: bool,
    pub is_up: bool,
    pub is_down: bool,
    yaw: f32,
    pitch: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            is_forward: false,
            is_backward: false,
            is_left: false,
            is_right: false,
            is_up: false,
            is_down: false,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn handle_mouse(&mut self, _camera: &mut Camera, dx: f64, dy: f64) {
        self.yaw -= dx as f32 * self.sensitivity;
        self.pitch -= dy as f32 * self.sensitivity;
        self.pitch = self.pitch.clamp(-1.5, 1.5); 
    }

    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        // Собираем кватернион из yaw/pitch
        camera.rotation = Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.0);

        let speed = self.speed * dt;
        let forward = camera.forward();
        let right = camera.right();

        if self.is_forward {
            camera.position += forward * speed;
        }
        if self.is_backward {
            camera.position -= forward * speed;
        }
        if self.is_right {
            camera.position += right * speed;
        }
        if self.is_left {
            camera.position -= right * speed;
        }
        if self.is_up {
            camera.position += Vec3::Y * speed;
        }
        if self.is_down {
            camera.position -= Vec3::Y * speed;
        }
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new(1.0, 0.01)
    }
}
