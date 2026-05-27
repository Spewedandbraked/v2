pub mod camera;
mod game_object;
pub mod gltf_loader;
pub mod gltf_model;
mod player;
mod world;

use player::Player;
use world::World;

use crate::{
    app::light::Light,
    game::camera::{Camera, CameraController},
};

pub struct GameState {
    pub world: World,
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub player: Option<Player>,
}
// impl Default for GameState {
//     fn default() -> Self {
//         Self {
//             world: World::default(),
//             camera: Camera::default(),
//             camera_controller: CameraController::default(),
//             player: None,
//         }
//     }
// }
impl Default for GameState {
    fn default() -> Self {
        let mut world = World::default();
        world.lights.push(Light {
            position: [11.0, 1.0, 0.0, 0.0],
            color: [2.0, 2.0, 2.0, 0.0],
            intensity: 0.1,
            radius: 110.0,
        });


        Self {
            world,
            camera: Camera::default(),
            camera_controller: CameraController::default(),
            player: None,
        }
    }
}
