use winit::{event::ElementState, keyboard::KeyCode};

use crate::game::GameState;

/// Структура, представляющая инструкцию для обработки нажатия клавиши.
/// 
/// # Пример
/// ```
/// let instruction = InputInstruction::new(KeyCode::Space)
///     .on_pressed(|game| {
///         game.player.as_mut().unwrap().jump();
///     })
///     .on_released(|game| {
///         println!("Space released");
///     });
/// ```
pub struct InputInstruction {
    pub key_code: KeyCode,
    on_pressed: Option<Box<dyn FnMut(&mut GameState)>>,
    on_released: Option<Box<dyn FnMut(&mut GameState)>>,
}

impl InputInstruction {
    pub fn new(key_code: KeyCode) -> Self {
        Self {
            key_code,
            on_pressed: None,
            on_released: None,
        }
    }

    pub fn on_pressed<F>(mut self, action: F) -> Self
    where
        F: FnMut(&mut GameState) + 'static,
    {
        self.on_pressed = Some(Box::new(action));
        self
    }

    pub fn on_released<F>(mut self, action: F) -> Self
    where
        F: FnMut(&mut GameState) + 'static,
    {
        self.on_released = Some(Box::new(action));
        self
    }

    pub fn execute(&mut self, game_state: &mut GameState, state: ElementState) {
        match state {
            ElementState::Pressed => {
                if let Some(action) = &mut self.on_pressed {
                    action(game_state);
                }
            }
            ElementState::Released => {
                if let Some(action) = &mut self.on_released {
                    action(game_state);
                }
            }
        }
    }
}