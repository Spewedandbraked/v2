pub struct World {
    pub ambient_light: f32,
    pub gltf_instance: Option<rend3_gltf::GltfSceneInstance>,
}
impl Default for World {
    fn default() -> Self {
        Self {
            ambient_light: 0.1,
            gltf_instance: None,
        }
    }
}