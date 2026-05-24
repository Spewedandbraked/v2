pub mod camera;
mod game_object;
mod player;
mod world;
pub mod gltf_model;
pub mod gltf_loader;

use player::Player;
use world::World;

use crate::{app::light::Light, game::camera::{Camera, CameraController}};

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
        
        // Красный источник слева
        world.lights.push(Light {
            position: [-5.0, 2.0, 0.0],
            color: [1.0, 0.0, 0.0],
            intensity: 13.0,
            radius: 110.0,
        });
        
        // Зелёный источник сверху
        world.lights.push(Light {
            position: [0.0, 5.0, 0.0],
            color: [0.0, 1.0, 0.0],
            intensity: 13.0,
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