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
    is_forward: bool,
    is_backward: bool,
    is_left: bool,
    is_right: bool,
    is_up: bool,
    is_down: bool,
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
        }
    }

    /// Обработка клавиш — возвращает true, если клавиша обработана
    pub fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right = is_pressed;
                true
            }
            KeyCode::Space => {
                self.is_up = is_pressed;
                true
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                self.is_down = is_pressed;
                true
            }
            _ => false,
        }
    }

    /// Обработка движения мыши
    pub fn handle_mouse(&mut self, dx: f64, dy: f64) {
        // Будет вызываться из DeviceEvent::MouseMotion
        // Пока заглушка — позже добавим поворот камеры
    }

    /// Обновить позицию камеры на основе нажатых клавиш
    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        let speed = self.speed * dt;

        if self.is_forward {
            camera.position += camera.forward() * speed;
        }
        if self.is_backward {
            camera.position -= camera.forward() * speed;
        }
        if self.is_right {
            camera.position += camera.right() * speed;
        }
        if self.is_left {
            camera.position -= camera.right() * speed;
        }
        if self.is_up {
            camera.position += camera.up() * speed;
        }
        if self.is_down {
            camera.position -= camera.up() * speed;
        }
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new(5.0, 0.5)
    }
}
