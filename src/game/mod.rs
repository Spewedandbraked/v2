mod camera;
mod world;

// pub use camera::CameraController;
// pub use world::World;

pub struct GameState {
    // pub world: World,
    // pub camera: CameraController,
}
impl Default for GameState {
    fn default() -> Self {
        Self {
            // world: World::default(),
            // camera: CameraController::default(),
        }
    }
}
