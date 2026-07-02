use crate::{app::light::Light, game::{game_object::GameObject, gltf_model::GltfModel, skybox::Skybox}};
#[derive(Default)]
pub struct World {
    pub ambient_light: f32,
    pub lights: Vec<Light>,
    pub objects: Vec<GameObject>,
    pub models: Vec<GltfModel>,
    pub skybox: Option<Skybox>,
}

