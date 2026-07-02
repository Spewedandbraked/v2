pub mod camera;
mod game_object;
pub mod gltf_loader;
pub mod gltf_model;
pub mod input_instructon;
mod player;
pub mod skybox;
mod world;

use std::collections::HashMap;

use player::Player;
use winit::{event::ElementState, keyboard::KeyCode};
use world::World;

use crate::{
    app::light::Light,
    game::{
        camera::{Camera, CameraController},
        input_instructon::InputInstruction,
    },
};

pub struct GameState {
    pub world: World,
    pub camera: Camera,
    pub camera_controller: CameraController,
    pub player: Option<Player>,
    pub input_instructions: HashMap<KeyCode, InputInstruction>,
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

        let mut input_instructions = HashMap::new();

        // W - вперед
        input_instructions.insert(
            KeyCode::KeyW,
            InputInstruction::new(KeyCode::KeyW)
                .on_pressed(|game| {
                    game.camera_controller.is_forward = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_forward = false;
                }),
        );

        // S - назад
        input_instructions.insert(
            KeyCode::KeyS,
            InputInstruction::new(KeyCode::KeyS)
                .on_pressed(|game| {
                    game.camera_controller.is_backward = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_backward = false;
                }),
        );

        // A - влево
        input_instructions.insert(
            KeyCode::KeyA,
            InputInstruction::new(KeyCode::KeyA)
                .on_pressed(|game| {
                    game.camera_controller.is_left = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_left = false;
                }),
        );

        // D - вправо
        input_instructions.insert(
            KeyCode::KeyD,
            InputInstruction::new(KeyCode::KeyD)
                .on_pressed(|game| {
                    game.camera_controller.is_right = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_right = false;
                }),
        );

        // Space - вверх
        input_instructions.insert(
            KeyCode::Space,
            InputInstruction::new(KeyCode::Space)
                .on_pressed(|game| {
                    game.camera_controller.is_up = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_up = false;
                }),
        );

        // Shift - вниз
        input_instructions.insert(
            KeyCode::ShiftLeft,
            InputInstruction::new(KeyCode::ShiftLeft)
                .on_pressed(|game| {
                    game.camera_controller.is_down = true;
                })
                .on_released(|game| {
                    game.camera_controller.is_down = false;
                }),
        );

        Self {
            world,
            camera: Camera::default(),
            camera_controller: CameraController::default(),
            player: None,
            input_instructions,
        }
    }
}
